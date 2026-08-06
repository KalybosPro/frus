//! **HCT** (Hue, Chroma, Tone) : l'espace couleur de Material 3, porté depuis
//! `material-color-utilities` (Google). Il combine la teinte/chroma perceptifs
//! de **CAM16** avec le ton **L\*** de CIELAB — le ton pilotant le contraste,
//! deux couleurs de tons éloignés de 40+ garantissent un texte lisible.
//!
//! Deux directions :
//! - [`Hct::from_color`] : analyse (CAM16 avant + L\*) ;
//! - [`Hct::solve`] : synthèse — trouve le sRGB affichable le plus proche de
//!   `(teinte, chroma, ton)`. Le chroma demandé est un **plafond** : hors
//!   gamut, il est réduit par dichotomie (teinte et ton sont préservés).
//!
//! [`TonalPalette`] décline une teinte/chroma sur l'échelle des tons — la
//! brique de `ColorScheme::from_seed`.

use crate::Color;

// --- CIE : Y (luminance relative) ↔ L* (ton perceptif) ---------------------

/// `L*` (0..100) depuis la luminance relative `Y` (0..100).
fn lstar_from_y(y: f64) -> f64 {
    let e = 216.0 / 24389.0;
    let yn = y / 100.0;
    if yn <= e {
        (24389.0 / 27.0) * yn
    } else {
        116.0 * yn.cbrt() - 16.0
    }
}

/// Luminance relative `Y` (0..100) depuis `L*` (0..100).
fn y_from_lstar(lstar: f64) -> f64 {
    let ft = (lstar + 16.0) / 116.0;
    let ft3 = ft * ft * ft;
    if ft3 > 216.0 / 24389.0 {
        ft3 * 100.0
    } else {
        lstar / (24389.0 / 27.0) * 100.0
    }
}

// --- sRGB (composantes 0..1) ↔ XYZ (échelle 0..100, D65) -------------------

fn linearized(c: f64) -> f64 {
    // sRGB → linéaire, sortie 0..100.
    if c <= 0.040449936 {
        c / 12.92 * 100.0
    } else {
        ((c + 0.055) / 1.055).powf(2.4) * 100.0
    }
}

fn delinearized(c: f64) -> f64 {
    // linéaire 0..100 → sRGB 0..1 (borné au gamut).
    let normalized = (c / 100.0).clamp(0.0, 1.0);
    if normalized <= 0.0031308 {
        normalized * 12.92
    } else {
        1.055 * normalized.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB → XYZ (D65), composantes XYZ sur 0..100.
fn xyz_from_color(color: Color) -> [f64; 3] {
    let r = linearized(color.r as f64);
    let g = linearized(color.g as f64);
    let b = linearized(color.b as f64);
    [
        0.41233895 * r + 0.35762064 * g + 0.18051042 * b,
        0.2126 * r + 0.7152 * g + 0.0722 * b,
        0.01932141 * r + 0.11916382 * g + 0.95034478 * b,
    ]
}

/// RGB **linéaire** (0..100) → [`Color`] sRGB (borné au gamut).
fn color_from_linrgb(rgb: [f64; 3]) -> Color {
    Color::rgb(
        delinearized(rgb[0]) as f32,
        delinearized(rgb[1]) as f32,
        delinearized(rgb[2]) as f32,
    )
}

// --- CAM16 : conditions de vision standard (sRGB) ---------------------------

/// Conditions de vision par défaut de `material-color-utilities` (blanc D65,
/// luminance d'adaptation ≈ 11,73 cd/m², fond L\* 50, entourage moyen).
struct ViewingConditions {
    aw: f64,
    nbb: f64,
    ncb: f64,
    c: f64,
    nc: f64,
    n: f64,
    rgb_d: [f64; 3],
    fl: f64,
    z: f64,
}

fn default_viewing_conditions() -> ViewingConditions {
    let white_point = [95.047, 100.0, 108.883]; // D65
    let adapting_luminance = (200.0 / std::f64::consts::PI) * (y_from_lstar(50.0) / 100.0);
    let background_lstar = 50.0;
    let surround = 2.0;

    // Blanc dans l'espace des cônes (matrice CAT16).
    let r_w = white_point[0] * 0.401288 + white_point[1] * 0.650173 + white_point[2] * -0.051461;
    let g_w = white_point[0] * -0.250268 + white_point[1] * 1.204414 + white_point[2] * 0.045854;
    let b_w = white_point[0] * -0.002079 + white_point[1] * 0.048952 + white_point[2] * 0.953127;

    let f = 0.8 + surround / 10.0;
    let c = if f >= 0.9 {
        // lerp(0.59, 0.69, (f-0.9)*10)
        0.59 + (0.69 - 0.59) * ((f - 0.9) * 10.0)
    } else {
        0.525 + (0.59 - 0.525) * (f / 0.9)
    };
    let nc = f;
    let d = (f * (1.0 - (1.0 / 3.6) * ((-adapting_luminance - 42.0) / 92.0).exp())).clamp(0.0, 1.0);
    let rgb_d = [
        d * (100.0 / r_w) + 1.0 - d,
        d * (100.0 / g_w) + 1.0 - d,
        d * (100.0 / b_w) + 1.0 - d,
    ];
    let k = 1.0 / (5.0 * adapting_luminance + 1.0);
    let k4 = k * k * k * k;
    let k4f = 1.0 - k4;
    let fl = k4 * adapting_luminance + 0.1 * k4f * k4f * (5.0 * adapting_luminance).cbrt();
    let n = y_from_lstar(background_lstar) / white_point[1];
    let z = 1.48 + n.sqrt();
    let nbb = 0.725 / n.powf(0.2);
    let ncb = nbb;

    let adapt = |cone: f64, dc: f64| {
        let af = (fl * dc * cone / 100.0).powf(0.42);
        400.0 * af / (af + 27.13)
    };
    let r_a = adapt(r_w, rgb_d[0]);
    let g_a = adapt(g_w, rgb_d[1]);
    let b_a = adapt(b_w, rgb_d[2]);
    let aw = (2.0 * r_a + g_a + 0.05 * b_a) * nbb;

    ViewingConditions {
        aw,
        nbb,
        ncb,
        c,
        nc,
        n,
        rgb_d,
        fl,
        z,
    }
}

/// Teinte (degrés) et chroma CAM16 d'une couleur, sous conditions standard.
fn cam16_hue_chroma(color: Color) -> (f64, f64) {
    let vc = default_viewing_conditions();
    let [x, y, z] = xyz_from_color(color);

    // Cônes (CAT16), adaptation chromatique puis compression.
    let r_c = 0.401288 * x + 0.650173 * y - 0.051461 * z;
    let g_c = -0.250268 * x + 1.204414 * y + 0.045854 * z;
    let b_c = -0.002079 * x + 0.048952 * y + 0.953127 * z;
    let adapt = |cone: f64, dc: f64| {
        let scaled = vc.fl * dc * cone / 100.0;
        let af = scaled.abs().powf(0.42);
        scaled.signum() * 400.0 * af / (af + 27.13)
    };
    let r_a = adapt(r_c, vc.rgb_d[0]);
    let g_a = adapt(g_c, vc.rgb_d[1]);
    let b_a = adapt(b_c, vc.rgb_d[2]);

    // Axes opposés a (rouge-vert) / b (jaune-bleu) et teinte.
    let a = (11.0 * r_a + -12.0 * g_a + b_a) / 11.0;
    let b_axis = (r_a + g_a - 2.0 * b_a) / 9.0;
    let u = (20.0 * r_a + 20.0 * g_a + 21.0 * b_a) / 20.0;
    let p2 = (40.0 * r_a + 20.0 * g_a + b_a) / 20.0;
    let atan_deg = b_axis.atan2(a).to_degrees();
    let hue = if atan_deg < 0.0 {
        atan_deg + 360.0
    } else {
        atan_deg
    };

    // Clarté J puis chroma.
    let ac = p2 * vc.nbb;
    let j = 100.0 * (ac / vc.aw).powf(vc.c * vc.z);
    let hue_prime = if hue < 20.14 { hue + 360.0 } else { hue };
    let e_hue = 0.25 * ((hue_prime.to_radians() + 2.0).cos() + 3.8);
    let p1 = 50000.0 / 13.0 * e_hue * vc.nc * vc.ncb;
    let t = p1 * a.hypot(b_axis) / (u + 0.305);
    let alpha = t.powf(0.9) * (1.64 - 0.29_f64.powf(vc.n)).powf(0.73);
    let chroma = alpha * (j / 100.0).sqrt();
    (hue, chroma)
}

// --- Solveur : (teinte, chroma, ton) → sRGB ---------------------------------

/// Matrice « linéaire 0..100 » depuis les cônes adaptés remis à l'échelle
/// (constantes du HctSolver de material-color-utilities).
const LINRGB_FROM_SCALED_DISCOUNT: [[f64; 3]; 3] = [
    [1373.2198709594231, -1100.4251190754821, -7.278681089101213],
    [-271.815969077903, 559.6580465940733, -32.46047482791194],
    [1.9622899599665666, -57.173814538844006, 308.7233197812385],
];
const Y_FROM_LINRGB: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// Gris neutre de ton `tone` (chroma nul).
fn gray(tone: f64) -> Color {
    let y = y_from_lstar(tone);
    // Y cible sur les trois composantes linéaires identiques.
    let component = delinearized(y) as f32;
    Color::rgb(component, component, component)
}

/// Tente de résoudre `(teinte, chroma, Y)` par itération sur la clarté `J`
/// (5 pas de Newton sur Y). `None` si le résultat sort du gamut sRGB.
fn find_result_by_j(hue_radians: f64, chroma: f64, y_target: f64) -> Option<Color> {
    let vc = default_viewing_conditions();
    let mut j = y_target.sqrt() * 11.0;

    let t_inner_coeff = 1.0 / (1.64 - 0.29_f64.powf(vc.n)).powf(0.73);
    let e_hue = 0.25 * ((hue_radians + 2.0).cos() + 3.8);
    let p1 = e_hue * (50000.0 / 13.0) * vc.nc * vc.ncb;
    let h_sin = hue_radians.sin();
    let h_cos = hue_radians.cos();

    for iteration in 0..5 {
        let j_normalized = j / 100.0;
        let alpha = if chroma == 0.0 || j == 0.0 {
            0.0
        } else {
            chroma / j_normalized.sqrt()
        };
        let t = (alpha * t_inner_coeff).powf(1.0 / 0.9);
        let ac = vc.aw * j_normalized.powf(1.0 / (vc.c * vc.z));
        let p2 = ac / vc.nbb;
        let gamma = 23.0 * (p2 + 0.305) * t / (23.0 * p1 + 11.0 * t * h_cos + 108.0 * t * h_sin);
        let a = gamma * h_cos;
        let b = gamma * h_sin;
        let r_a = (460.0 * p2 + 451.0 * a + 288.0 * b) / 1403.0;
        let g_a = (460.0 * p2 - 891.0 * a - 261.0 * b) / 1403.0;
        let b_a = (460.0 * p2 - 220.0 * a - 6300.0 * b) / 1403.0;

        // Décompression des cônes adaptés (inverse de la compression 0,42).
        let inverse_adapt = |adapted: f64| {
            let adapted_abs = adapted.abs();
            let base = (27.13 * adapted_abs / (400.0 - adapted_abs)).max(0.0);
            adapted.signum() * base.powf(1.0 / 0.42)
        };
        let r_c = inverse_adapt(r_a);
        let g_c = inverse_adapt(g_a);
        let b_c = inverse_adapt(b_a);

        let m = &LINRGB_FROM_SCALED_DISCOUNT;
        let lin = [
            m[0][0] * r_c + m[0][1] * g_c + m[0][2] * b_c,
            m[1][0] * r_c + m[1][1] * g_c + m[1][2] * b_c,
            m[2][0] * r_c + m[2][1] * g_c + m[2][2] * b_c,
        ];
        if lin[0] < 0.0 || lin[1] < 0.0 || lin[2] < 0.0 {
            return None;
        }
        let fnj = Y_FROM_LINRGB[0] * lin[0] + Y_FROM_LINRGB[1] * lin[1] + Y_FROM_LINRGB[2] * lin[2];
        if fnj <= 0.0 {
            return None;
        }
        if iteration == 4 || (fnj - y_target).abs() < 0.002 {
            if lin[0] > 100.01 || lin[1] > 100.01 || lin[2] > 100.01 {
                return None;
            }
            return Some(color_from_linrgb(lin));
        }
        // Pas de Newton sur Y (f(J) ≈ Y, dY/dJ ≈ 2Y/J).
        j -= (fnj - y_target) * j / (2.0 * fnj);
    }
    None
}

/// Une couleur en **HCT** : teinte CAM16 (degrés), chroma CAM16, ton L\*.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hct {
    /// Teinte, en degrés (0..360).
    pub hue: f64,
    /// Chroma (0 = gris ; le maximum dépend de la teinte et du ton).
    pub chroma: f64,
    /// Ton `L*` (0 = noir, 100 = blanc) — l'axe du contraste.
    pub tone: f64,
}

impl Hct {
    /// Analyse une couleur sRGB en HCT.
    pub fn from_color(color: Color) -> Self {
        let (hue, chroma) = cam16_hue_chroma(color);
        let tone = lstar_from_y(xyz_from_color(color)[1]);
        Self { hue, chroma, tone }
    }

    /// Trouve la couleur sRGB affichable pour `(hue, chroma, tone)`. Le chroma
    /// est réduit si nécessaire (teinte et ton préservés).
    pub fn solve(hue: f64, chroma: f64, tone: f64) -> Color {
        // Cas dégénérés : gris pur (le ton seul décide).
        if chroma < 1e-4 || !(1e-4..=99.9999).contains(&tone) {
            return gray(tone.clamp(0.0, 100.0));
        }
        let hue = hue.rem_euclid(360.0);
        let hue_radians = hue.to_radians();
        let y = y_from_lstar(tone);

        if let Some(exact) = find_result_by_j(hue_radians, chroma, y) {
            return exact;
        }
        // Hors gamut : dichotomie sur le chroma (précision 0,4 — celle du
        // solveur historique de material-color-utilities).
        let (mut low, mut high) = (0.0_f64, chroma);
        let mut answer = gray(tone);
        while high - low > 0.4 {
            let mid = (low + high) / 2.0;
            match find_result_by_j(hue_radians, mid, y) {
                Some(color) => {
                    answer = color;
                    low = mid;
                }
                None => high = mid,
            }
        }
        answer
    }
}

/// Une **palette tonale** : une teinte et un chroma déclinés sur l'échelle des
/// tons. `tone(90)` = très clair, `tone(10)` = très sombre, même « couleur ».
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TonalPalette {
    pub hue: f64,
    pub chroma: f64,
}

impl TonalPalette {
    /// Palette de teinte `hue` (degrés) et chroma plafond `chroma`.
    pub fn new(hue: f64, chroma: f64) -> Self {
        Self { hue, chroma }
    }

    /// La couleur de ton `tone` (0..100) de cette palette.
    pub fn tone(&self, tone: f64) -> Color {
        Hct::solve(self.hue, self.chroma, tone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(value: f64, expected: f64, tolerance: f64, label: &str) {
        assert!(
            (value - expected).abs() <= tolerance,
            "{label} : {value} attendu ≈ {expected} (± {tolerance})"
        );
    }

    #[test]
    fn tone_matches_lstar() {
        // Blanc/noir/gris moyen : le ton est le L* CIELAB.
        assert_close(Hct::from_color(Color::WHITE).tone, 100.0, 0.1, "ton blanc");
        assert_close(Hct::from_color(Color::BLACK).tone, 0.0, 0.1, "ton noir");
        // sRGB 0.5 → L* ≈ 53.59 (gris matériel : #808080 → 53.585).
        let mid = Hct::from_color(Color::rgb8(0x80, 0x80, 0x80));
        assert_close(mid.tone, 53.585, 0.2, "ton gris moyen");
        // CAM16 sous adaptation partielle laisse un chroma résiduel ≈ 1,9 aux
        // gris (référence : materialyoucolor 1.896) — ce n'est pas un bug.
        assert_close(mid.chroma, 1.896, 0.1, "chroma résiduel du gris");
    }

    #[test]
    fn google_blue_analyzes_to_known_hct() {
        // #4285F4 : valeurs de référence material-color-utilities
        // (hue 265.979°, chroma 62.269, tone 56.550 — vérifiées contre le
        // port Python `materialyoucolor`).
        let hct = Hct::from_color(Color::rgb8(0x42, 0x85, 0xF4));
        assert_close(hct.hue, 265.979, 0.2, "teinte bleu Google");
        assert_close(hct.chroma, 62.269, 0.2, "chroma bleu Google");
        assert_close(hct.tone, 56.55, 0.2, "ton bleu Google");
    }

    #[test]
    fn solve_round_trips_in_gamut_colors() {
        // Des cibles bien dans le gamut : la teinte et le ton doivent se
        // retrouver quasi exactement, le chroma à la précision du solveur.
        for &(hue, chroma, tone) in &[
            (27.0, 16.0, 50.0),
            (120.0, 24.0, 70.0),
            (282.0, 40.0, 40.0),
            (200.0, 8.0, 90.0),
        ] {
            let color = Hct::solve(hue, chroma, tone);
            let round = Hct::from_color(color);
            assert_close(round.hue, hue, 4.0, "teinte round-trip");
            assert_close(round.chroma, chroma, 2.5, "chroma round-trip");
            assert_close(round.tone, tone, 0.5, "ton round-trip");
        }
    }

    #[test]
    fn solve_clamps_impossible_chroma_but_keeps_hue_and_tone() {
        // Chroma 200 n'existe pour aucune teinte : le solveur doit rendre la
        // couleur la plus chromatique du gamut, teinte et ton préservés.
        let color = Hct::solve(265.0, 200.0, 30.0);
        let round = Hct::from_color(color);
        assert_close(round.tone, 30.0, 1.0, "ton préservé hors gamut");
        assert_close(round.hue, 265.0, 4.0, "teinte préservée hors gamut");
        assert!(
            round.chroma > 20.0,
            "chroma maximal atteint ({})",
            round.chroma
        );
    }

    #[test]
    fn solve_matches_reference_implementation() {
        // Sorties du port Python `materialyoucolor` (HctSolver Google), à
        // ± 1/255 par canal près (arrondis d'implémentation).
        // Tolérance : ±1 dans le gamut ; ±3 hors gamut (notre dichotomie sur le
        // chroma a une précision de 0,4 là où Google bissecte la frontière
        // exacte du gamut).
        let cases: [(f64, f64, f64, [u8; 3], i32); 3] = [
            (27.0, 16.0, 50.0, [0x92, 0x6F, 0x69], 1),
            (120.0, 24.0, 70.0, [0xA8, 0xB0, 0x7E], 1),
            (265.0, 200.0, 30.0, [0x00, 0x44, 0x91], 3), // hors gamut → plafonné
        ];
        for (hue, chroma, tone, expected, tolerance) in cases {
            let color = Hct::solve(hue, chroma, tone);
            let actual = [
                (color.r * 255.0).round() as i32,
                (color.g * 255.0).round() as i32,
                (color.b * 255.0).round() as i32,
            ];
            for (a, e) in actual.iter().zip(expected.iter()) {
                assert!(
                    (a - *e as i32).abs() <= tolerance,
                    "solve({hue},{chroma},{tone}) = {actual:?}, référence {expected:?}"
                );
            }
        }
    }

    #[test]
    fn tonal_palette_is_monotonic_in_luminance() {
        let palette = TonalPalette::new(282.0, 48.0);
        let mut previous = -1.0_f32;
        for tone in [0.0, 10.0, 20.0, 40.0, 60.0, 80.0, 90.0, 99.0, 100.0] {
            let luminance = palette.tone(tone).compute_luminance();
            assert!(
                luminance >= previous,
                "luminance non monotone au ton {tone} : {luminance} < {previous}"
            );
            previous = luminance;
        }
        // Extrêmes : ton 0 = noir, ton 100 = blanc (chroma inatteignable).
        assert!(palette.tone(0.0).compute_luminance() < 1e-3);
        assert!(palette.tone(100.0).compute_luminance() > 0.99);
    }

    #[test]
    fn degenerate_inputs_do_not_produce_nan() {
        for color in [
            Hct::solve(0.0, 0.0, 50.0),
            Hct::solve(-90.0, 30.0, 50.0), // teinte négative normalisée
            Hct::solve(400.0, 30.0, 101.0),
            Hct::solve(120.0, 30.0, -5.0),
        ] {
            assert!(color.r.is_finite() && color.g.is_finite() && color.b.is_finite());
            assert!((0.0..=1.0).contains(&color.r));
        }
    }
}
