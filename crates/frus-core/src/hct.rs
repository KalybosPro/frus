//! **HCT** (Hue, Chroma, Tone): the Material 3 colour space, ported from
//! `material-color-utilities` (Google). It pairs the perceptual hue and chroma
//! of **CAM16** with CIELAB's **L\*** tone. Tone is what drives contrast: two
//! colours 40 or more tones apart are guaranteed to render legible text.
//!
//! Two directions:
//! - [`Hct::from_color`]: analysis (forward CAM16 plus L\*);
//! - [`Hct::solve`]: synthesis — finds the displayable sRGB colour closest to
//!   `(hue, chroma, tone)`. The requested chroma is a **ceiling**: out of
//!   gamut, it is reduced by bisection, while hue and tone are preserved.
//!
//! [`TonalPalette`] spreads one hue and chroma across the tone scale — the
//! building block of `ColorScheme::from_seed`.

use crate::Color;

// --- CIE: Y (relative luminance) <-> L* (perceptual tone) ------------------

/// `L*` (0..100) from the relative luminance `Y` (0..100).
fn lstar_from_y(y: f64) -> f64 {
    let e = 216.0 / 24389.0;
    let yn = y / 100.0;
    if yn <= e {
        (24389.0 / 27.0) * yn
    } else {
        116.0 * yn.cbrt() - 16.0
    }
}

/// Relative luminance `Y` (0..100) from `L*` (0..100).
fn y_from_lstar(lstar: f64) -> f64 {
    let ft = (lstar + 16.0) / 116.0;
    let ft3 = ft * ft * ft;
    if ft3 > 216.0 / 24389.0 {
        ft3 * 100.0
    } else {
        lstar / (24389.0 / 27.0) * 100.0
    }
}

// --- sRGB (components 0..1) <-> XYZ (0..100 scale, D65) --------------------

fn linearized(c: f64) -> f64 {
    // sRGB -> linear, output on 0..100.
    if c <= 0.040449936 {
        c / 12.92 * 100.0
    } else {
        ((c + 0.055) / 1.055).powf(2.4) * 100.0
    }
}

fn delinearized(c: f64) -> f64 {
    // Linear 0..100 -> sRGB 0..1, clamped to the gamut.
    let normalized = (c / 100.0).clamp(0.0, 1.0);
    if normalized <= 0.0031308 {
        normalized * 12.92
    } else {
        1.055 * normalized.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB -> XYZ (D65), with XYZ components on 0..100.
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

/// **Linear** RGB (0..100) -> an sRGB [`Color`], clamped to the gamut.
fn color_from_linrgb(rgb: [f64; 3]) -> Color {
    Color::rgb(
        delinearized(rgb[0]) as f32,
        delinearized(rgb[1]) as f32,
        delinearized(rgb[2]) as f32,
    )
}

// --- CAM16: the standard viewing conditions (sRGB) --------------------------

/// The default viewing conditions of `material-color-utilities`: D65 white,
/// adapting luminance about 11.73 cd/m2, an L\* 50 background, average surround.
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

    // The white point in cone space (the CAT16 matrix).
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

/// A colour's CAM16 hue (degrees) and chroma, under the standard conditions.
fn cam16_hue_chroma(color: Color) -> (f64, f64) {
    let vc = default_viewing_conditions();
    let [x, y, z] = xyz_from_color(color);

    // Cones (CAT16): chromatic adaptation, then compression.
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

    // The opponent axes a (red-green) and b (yellow-blue), and the hue.
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

    // Lightness J, then chroma.
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

// --- Solver: (hue, chroma, tone) -> sRGB ------------------------------------

/// The "linear 0..100" matrix, from the rescaled adapted cones (constants taken
/// from material-color-utilities' HctSolver).
const LINRGB_FROM_SCALED_DISCOUNT: [[f64; 3]; 3] = [
    [1373.2198709594231, -1100.4251190754821, -7.278681089101213],
    [-271.815969077903, 559.6580465940733, -32.46047482791194],
    [1.9622899599665666, -57.173814538844006, 308.7233197812385],
];
const Y_FROM_LINRGB: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// The neutral grey of tone `tone` (zero chroma).
fn gray(tone: f64) -> Color {
    let y = y_from_lstar(tone);
    // The target Y, applied to all three identical linear components.
    let component = delinearized(y) as f32;
    Color::rgb(component, component, component)
}

/// Attempts to solve `(hue, chroma, Y)` by iterating on lightness `J` (five
/// Newton steps on Y). `None` when the result falls outside the sRGB gamut.
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

        // Decompress the adapted cones (the inverse of the 0.42 compression).
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
        // A Newton step on Y (f(J) ~ Y, dY/dJ ~ 2Y/J).
        j -= (fnj - y_target) * j / (2.0 * fnj);
    }
    None
}

/// A colour in **HCT**: CAM16 hue (degrees), CAM16 chroma, and L\* tone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hct {
    /// Hue, in degrees (0..360).
    pub hue: f64,
    /// Chroma (0 = grey; the maximum depends on the hue and the tone).
    pub chroma: f64,
    /// Tone `L*` (0 = black, 100 = white) — the contrast axis.
    pub tone: f64,
}

impl Hct {
    /// Analyses an sRGB colour into HCT.
    pub fn from_color(color: Color) -> Self {
        let (hue, chroma) = cam16_hue_chroma(color);
        let tone = lstar_from_y(xyz_from_color(color)[1]);
        Self { hue, chroma, tone }
    }

    /// Finds the displayable sRGB colour for `(hue, chroma, tone)`. Chroma is
    /// reduced where necessary; hue and tone are preserved.
    pub fn solve(hue: f64, chroma: f64, tone: f64) -> Color {
        // Degenerate cases: pure grey, where tone alone decides.
        if chroma < 1e-4 || !(1e-4..=99.9999).contains(&tone) {
            return gray(tone.clamp(0.0, 100.0));
        }
        let hue = hue.rem_euclid(360.0);
        let hue_radians = hue.to_radians();
        let y = y_from_lstar(tone);

        if let Some(exact) = find_result_by_j(hue_radians, chroma, y) {
            return exact;
        }
        // Out of gamut: bisect on chroma, to a precision of 0.4 — the same as
        // the original material-color-utilities solver.
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

/// A **tonal palette**: one hue and chroma spread across the tone scale.
/// `tone(90)` is very light, `tone(10)` very dark, and both read as one colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TonalPalette {
    pub hue: f64,
    pub chroma: f64,
}

impl TonalPalette {
    /// A palette of hue `hue` (degrees), with `chroma` as the chroma ceiling.
    pub fn new(hue: f64, chroma: f64) -> Self {
        Self { hue, chroma }
    }

    /// This palette's colour at tone `tone` (0..100).
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
        // White, black and mid grey: the tone is CIELAB's L*.
        assert_close(Hct::from_color(Color::WHITE).tone, 100.0, 0.1, "ton blanc");
        assert_close(Hct::from_color(Color::BLACK).tone, 0.0, 0.1, "ton noir");
        // sRGB 0.5 -> L* about 53.59 (material grey: #808080 -> 53.585).
        let mid = Hct::from_color(Color::rgb8(0x80, 0x80, 0x80));
        assert_close(mid.tone, 53.585, 0.2, "ton gris moyen");
        // Under partial adaptation CAM16 leaves a residual chroma of about 1.9 on
        // greys (reference: materialyoucolor gives 1.896). This is not a bug.
        assert_close(mid.chroma, 1.896, 0.1, "grey's residual chroma");
    }

    #[test]
    fn google_blue_analyzes_to_known_hct() {
        // #4285F4: the material-color-utilities reference values (hue 265.979
        // degrees, chroma 62.269, tone 56.550), checked against the Python port
        // `materialyoucolor`.
        let hct = Hct::from_color(Color::rgb8(0x42, 0x85, 0xF4));
        assert_close(hct.hue, 265.979, 0.2, "teinte bleu Google");
        assert_close(hct.chroma, 62.269, 0.2, "chroma bleu Google");
        assert_close(hct.tone, 56.55, 0.2, "ton bleu Google");
    }

    #[test]
    fn solve_round_trips_in_gamut_colors() {
        // Targets well inside the gamut: hue and tone should come back almost
        // exactly, chroma to the solver's precision.
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
        // No hue has a chroma of 200: the solver must return the most chromatic
        // colour in the gamut, preserving hue and tone.
        let color = Hct::solve(265.0, 200.0, 30.0);
        let round = Hct::from_color(color);
        assert_close(round.tone, 30.0, 1.0, "tone preserved out of gamut");
        assert_close(round.hue, 265.0, 4.0, "hue preserved out of gamut");
        assert!(
            round.chroma > 20.0,
            "chroma maximal atteint ({})",
            round.chroma
        );
    }

    #[test]
    fn solve_matches_reference_implementation() {
        // Output of the Python port `materialyoucolor` (Google's HctSolver), to
        // within 1/255 per channel (implementation rounding).
        // Tolerance: +/-1 inside the gamut, +/-3 outside it — our chroma bisection
        // has a precision of 0.4 where Google bisects the exact gamut boundary.
        let cases: [(f64, f64, f64, [u8; 3], i32); 3] = [
            (27.0, 16.0, 50.0, [0x92, 0x6F, 0x69], 1),
            (120.0, 24.0, 70.0, [0xA8, 0xB0, 0x7E], 1),
            (265.0, 200.0, 30.0, [0x00, 0x44, 0x91], 3), // out of gamut -> capped
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
                    "solve({hue},{chroma},{tone}) = {actual:?}, reference {expected:?}"
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
        // Extremes: tone 0 is black, tone 100 is white; chroma is unreachable.
        assert!(palette.tone(0.0).compute_luminance() < 1e-3);
        assert!(palette.tone(100.0).compute_luminance() > 0.99);
    }

    #[test]
    fn degenerate_inputs_do_not_produce_nan() {
        for color in [
            Hct::solve(0.0, 0.0, 50.0),
            Hct::solve(-90.0, 30.0, 50.0), // negative hue, normalised
            Hct::solve(400.0, 30.0, 101.0),
            Hct::solve(120.0, 30.0, -5.0),
        ] {
            assert!(color.r.is_finite() && color.g.is_finite() && color.b.is_finite());
            assert!((0.0..=1.0).contains(&color.r));
        }
    }
}
