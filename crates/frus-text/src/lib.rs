//! `frus-text` — mesure (et, à terme, shaping) de texte, au-dessus de
//! [`cosmic_text`](https://docs.rs/cosmic-text).
//!
//! Pour ce jalon, l'API se limite à [`measure`] : la taille naturelle d'une
//! ligne de texte, dont le moteur de layout a besoin pour dimensionner un
//! widget `Text`.
//!
//! Le `FontSystem` (chargement des polices) est coûteux : on l'initialise
//! **paresseusement** et on le partage derrière un `Mutex`. C'est un choix v1
//! pragmatique ; l'unification avec le `FontSystem` du renderer viendra plus tard.

use std::sync::{Mutex, OnceLock};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, Weight};
use frus_core::{FontWeight, Point, Rect, Size, TextRun};

/// Rapport interligne / taille de police par défaut.
const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Police de repli **embarquée** (sans-serif) et sa variante monospace. Les
/// embarquer garantit un rendu de texte déterministe sur **toutes** les
/// plateformes — notamment Android, où l'alias système « sans-serif » (défini
/// dans `fonts.xml`, non lu par fontdb) ne résout aucune police par défaut.
const DEJAVU_SANS: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
/// Face **grasse** embarquée : sans elle, un poids `Bold` retomberait en douce sur
/// la face normale partout où seules les polices embarquées existent (Android).
const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf");
const DEJAVU_MONO: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");

/// Nom de famille interne des polices embarquées (doit correspondre aux TTF).
const SANS_FAMILY: &str = "DejaVu Sans";
const MONO_FAMILY: &str = "DejaVu Sans Mono";

/// Construit un `FontSystem` prêt à l'emploi : polices système (repli emoji /
/// scripts) **plus** la police embarquée, fixée comme famille par défaut. À
/// utiliser partout où un `FontSystem` est créé (mesure ici, rendu dans
/// `frus-gpu`) pour un rendu de texte cohérent et sans dépendance aux polices
/// système, qui peuvent manquer un défaut résoluble (cas Android).
pub fn new_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    let db = font_system.db_mut();
    db.load_font_data(DEJAVU_SANS.to_vec());
    db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
    db.load_font_data(DEJAVU_MONO.to_vec());
    // Fait résoudre chaque famille générique vers une police réellement présente.
    db.set_sans_serif_family(SANS_FAMILY);
    db.set_serif_family(SANS_FAMILY);
    db.set_cursive_family(SANS_FAMILY);
    db.set_fantasy_family(SANS_FAMILY);
    db.set_monospace_family(MONO_FAMILY);
    font_system
}

fn font_system() -> &'static Mutex<FontSystem> {
    static FONT_SYSTEM: OnceLock<Mutex<FontSystem>> = OnceLock::new();
    FONT_SYSTEM.get_or_init(|| Mutex::new(new_font_system()))
}

/// Interligne pour une taille de police donnée (en pixels).
pub fn line_height(size_px: f32) -> f32 {
    size_px * LINE_HEIGHT_FACTOR
}

/// Mesure la taille naturelle d'un texte (multi-lignes autorisé), en pixels,
/// en graisse normale. Voir [`measure_styled`] pour la graisse/l'italique.
pub fn measure(text: &str, size_px: f32) -> Size {
    measure_styled(text, size_px, FontWeight::Regular, false)
}

/// Mesure la taille naturelle d'un texte **stylé** (graisse/italique comptent :
/// un gras est plus large qu'un normal — la mise en page doit le savoir).
pub fn measure_styled(text: &str, size_px: f32, weight: FontWeight, italic: bool) -> Size {
    let line_h = line_height(size_px);
    if text.is_empty() {
        return Size::new(0.0, line_h);
    }

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(size_px, line_h);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    // Largeur/hauteur non contraintes : on mesure la taille naturelle.
    buffer.set_size(&mut font_system, None, None);
    let attrs = Attrs::new().weight(Weight(weight.to_u16())).style(if italic {
        Style::Italic
    } else {
        Style::Normal
    });
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut width = 0.0_f32;
    let mut lines = 0.0_f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        lines += 1.0;
    }

    Size::new(width, lines.max(1.0) * line_h)
}

/// Mesure la taille naturelle d'un **texte riche** (runs résolus, styles/tailles
/// mêlés) : largeur de la plus longue ligne, hauteur réelle des lignes shapées.
pub fn measure_runs(runs: &[TextRun]) -> Size {
    if runs.iter().all(|r| r.text.is_empty()) {
        return Size::new(0.0, 0.0);
    }
    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(base, line_height(base));
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, None, None);
    let spans = runs.iter().map(|run| {
        (
            run.text.as_str(),
            Attrs::new()
                .weight(Weight(run.weight.to_u16()))
                .style(if run.italic { Style::Italic } else { Style::Normal })
                .metrics(Metrics::new(run.size, line_height(run.size))),
        )
    });
    buffer.set_rich_text(&mut font_system, spans, Attrs::new(), Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        height = height.max(run.line_top + run.line_height);
    }
    Size::new(width, height)
}

/// Une ligne shapée d'un [`TextLayout`] : les offsets `x` de chaque **frontière
/// de caractère**, extraits des glyphes réels (kerning/ligatures compris).
struct LayoutLine {
    /// Index (en caractères, global au texte) de la première frontière de la ligne.
    start_char: usize,
    /// `x` de chaque frontière de caractère de la ligne (`chars + 1` entrées).
    offsets: Vec<f32>,
    /// Bord haut de la ligne.
    top: f32,
    /// Hauteur de la ligne.
    height: f32,
}

/// La mise en forme **shapée** d'un texte (une seule passe cosmic-text), exposant
/// la géométrie dont un widget d'édition a besoin : position de caret par index
/// de caractère, hit-test inverse, rectangles de sélection. Les coordonnées sont
/// **locales** au texte (origine à son coin haut-gauche). Les indices sont en
/// **caractères** (la convention d'édition de frus), frontières `0..=len`.
///
/// Contrairement à une mesure de préfixe re-shapée sous-chaîne par sous-chaîne,
/// les offsets viennent de la ligne shapée **entière** : cohérents entre eux
/// (kerning), et calculés en une passe au lieu de `n`.
pub struct TextLayout {
    lines: Vec<LayoutLine>,
    size: Size,
    /// Nombre total de caractères (la dernière frontière valide).
    chars: usize,
}

impl TextLayout {
    /// Shape `text` (multi-lignes autorisé, non contraint en largeur) au style
    /// donné et en extrait la géométrie.
    pub fn new(text: &str, size_px: f32, weight: FontWeight, italic: bool) -> Self {
        let fallback_h = line_height(size_px);
        let mut lines: Vec<LayoutLine> = Vec::new();
        let mut width = 0.0_f32;
        let mut height = 0.0_f32;
        let mut start_char = 0usize;

        if !text.is_empty() {
            let mut font_system = font_system().lock().expect("FontSystem lock");
            let metrics = Metrics::new(size_px, fallback_h);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, None, None);
            let attrs = Attrs::new().weight(Weight(weight.to_u16())).style(if italic {
                Style::Italic
            } else {
                Style::Normal
            });
            buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
            buffer.shape_until_scroll(&mut font_system, false);

            for run in buffer.layout_runs() {
                // Frontières de caractères de la ligne, en offsets d'octets.
                let char_bytes: Vec<usize> = run.text.char_indices().map(|(b, _)| b).collect();
                let n = char_bytes.len();
                let mut offsets = vec![f32::NAN; n + 1];
                offsets[0] = 0.0;

                // Chaque glyphe couvre un cluster d'octets [start, end) sur
                // [x, x+w) ; les frontières internes d'un cluster (ligature)
                // sont interpolées linéairement.
                for glyph in run.glyphs.iter() {
                    let covered: Vec<usize> = (0..n)
                        .filter(|&i| char_bytes[i] >= glyph.start && char_bytes[i] < glyph.end)
                        .collect();
                    let k = covered.len().max(1) as f32;
                    for (j, &i) in covered.iter().enumerate() {
                        offsets[i] = glyph.x + glyph.w * (j as f32 / k);
                    }
                    // La frontière qui SUIT le cluster (si c'est un début de char).
                    if let Some(next) = (0..=n).find(|&i| {
                        let b = if i == n { run.text.len() } else { char_bytes[i] };
                        b == glyph.end
                    }) {
                        offsets[next] = glyph.x + glyph.w;
                    }
                }
                // Frontières restées sans glyphe (espaces réduits…) : continuité.
                for i in 1..=n {
                    if offsets[i].is_nan() {
                        offsets[i] = offsets[i - 1];
                    }
                }

                width = width.max(run.line_w);
                height = height.max(run.line_top + run.line_height);
                lines.push(LayoutLine {
                    start_char,
                    offsets,
                    top: run.line_top,
                    height: run.line_height,
                });
                // +1 : le séparateur de ligne (`\n`) compte un caractère.
                start_char += n + 1;
            }
        }

        // Texte vide (ou aucune ligne shapée) : une ligne vide synthétique pour
        // que caret/hit restent définis (caret à x = 0).
        if lines.is_empty() {
            lines.push(LayoutLine {
                start_char: 0,
                offsets: vec![0.0],
                top: 0.0,
                height: fallback_h,
            });
            height = fallback_h;
        }

        let chars = text.chars().count();
        Self {
            lines,
            size: Size::new(width, height),
            chars,
        }
    }

    /// Taille naturelle du texte shapé.
    pub fn size(&self) -> Size {
        self.size
    }

    /// La ligne contenant la frontière de caractère `index`.
    fn line_of(&self, index: usize) -> &LayoutLine {
        self.lines
            .iter()
            .rev()
            .find(|line| index >= line.start_char)
            .unwrap_or(&self.lines[0])
    }

    /// Rectangle du caret à la frontière `index` (largeur nulle : au widget de
    /// choisir l'épaisseur du trait). `index` est borné au texte.
    pub fn caret_rect(&self, index: usize) -> Rect {
        let index = index.min(self.chars);
        let line = self.line_of(index);
        let local = (index - line.start_char).min(line.offsets.len() - 1);
        Rect::new(line.offsets[local], line.top, 0.0, line.height)
    }

    /// Frontière de caractère la **plus proche** de `point` (coordonnées locales
    /// au texte). Le `y` choisit la ligne (borné), le `x` la frontière.
    pub fn hit_test(&self, point: Point) -> usize {
        let line = self
            .lines
            .iter()
            .find(|line| point.y < line.top + line.height)
            .unwrap_or(self.lines.last().expect("au moins une ligne"));

        let mut best = 0;
        let mut best_dist = f32::MAX;
        for (i, x) in line.offsets.iter().enumerate() {
            let dist = (x - point.x).abs();
            if dist < best_dist {
                best_dist = dist;
                best = i;
            }
        }
        (line.start_char + best).min(self.chars)
    }

    /// Rectangles couvrant la plage de caractères `[start, end)` (un par ligne
    /// traversée ; vides omis).
    pub fn selection_rects(&self, start: usize, end: usize) -> Vec<Rect> {
        let (start, end) = (start.min(self.chars), end.min(self.chars));
        if start >= end {
            return Vec::new();
        }
        let mut rects = Vec::new();
        for line in &self.lines {
            let line_len = line.offsets.len() - 1;
            let lo = start.max(line.start_char);
            let hi = end.min(line.start_char + line_len);
            if lo >= hi {
                continue;
            }
            let x0 = line.offsets[lo - line.start_char];
            let x1 = line.offsets[hi - line.start_char];
            if x1 > x0 {
                rects.push(Rect::new(x0, line.top, x1 - x0, line.height));
            }
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_zero_width() {
        let size = measure("", 16.0);
        assert_eq!(size.width, 0.0);
        assert!(size.height > 0.0);
    }

    #[test]
    fn non_empty_text_has_positive_size() {
        let size = measure("Bonjour", 24.0);
        assert!(size.width > 0.0, "largeur = {}", size.width);
        assert!(size.height > 0.0, "hauteur = {}", size.height);
    }

    #[test]
    fn rich_runs_measure_mixed_styles() {
        use frus_core::Color;
        let run = |text: &str, size: f32, weight: FontWeight| TextRun {
            text: text.to_string(),
            size,
            weight,
            italic: false,
            color: Color::WHITE,
        };
        // « normal GRAS » : plus large que « normal » seul ; hauteur = du plus grand run.
        let plain = measure_runs(&[run("normal", 16.0, FontWeight::Regular)]);
        let mixed = measure_runs(&[
            run("normal", 16.0, FontWeight::Regular),
            run(" GRAS", 24.0, FontWeight::Bold),
        ]);
        assert!(mixed.width > plain.width);
        assert!(
            mixed.height >= line_height(24.0) - 1.0,
            "hauteur pilotée par le run 24 px : {}",
            mixed.height
        );
        // Vide → taille nulle.
        assert_eq!(measure_runs(&[]), Size::new(0.0, 0.0));
    }

    #[test]
    fn layout_offsets_are_monotonic_and_match_size() {
        let layout = TextLayout::new("Bonjour le monde", 18.0, FontWeight::Regular, false);
        let mut prev = -1.0;
        for i in 0..=16 {
            let x = layout.caret_rect(i).x;
            assert!(x >= prev, "offset décroissant à la frontière {i}");
            prev = x;
        }
        // La dernière frontière atteint la largeur naturelle.
        assert!((layout.caret_rect(16).x - layout.size().width).abs() < 0.5);
        // Frontière au-delà du texte : bornée.
        assert_eq!(layout.caret_rect(99).x, layout.caret_rect(16).x);
    }

    #[test]
    fn hit_test_roundtrips_caret_positions() {
        let layout = TextLayout::new("Bonjour", 18.0, FontWeight::Regular, false);
        for i in 0..=7 {
            let caret = layout.caret_rect(i);
            let hit = layout.hit_test(Point::new(caret.x, caret.y + 1.0));
            assert_eq!(hit, i, "aller-retour caret→hit à la frontière {i}");
        }
        // Loin à gauche / à droite : bornes.
        assert_eq!(layout.hit_test(Point::new(-100.0, 0.0)), 0);
        assert_eq!(layout.hit_test(Point::new(10_000.0, 0.0)), 7);
    }

    #[test]
    fn selection_rects_cover_the_range() {
        let layout = TextLayout::new("Bonjour", 18.0, FontWeight::Regular, false);
        let rects = layout.selection_rects(2, 5);
        assert_eq!(rects.len(), 1);
        let r = rects[0];
        assert!((r.x - layout.caret_rect(2).x).abs() < 0.01);
        assert!((r.x + r.width - layout.caret_rect(5).x).abs() < 0.01);
        assert!(r.height > 0.0);
        // Plage vide ou inversée : aucun rectangle.
        assert!(layout.selection_rects(3, 3).is_empty());
        assert!(layout.selection_rects(5, 2).is_empty());
    }

    #[test]
    fn multiline_layout_maps_lines_and_indices() {
        // "ab\ncd" : frontières 0..=2 sur la ligne 1, 3..=5 sur la ligne 2.
        let layout = TextLayout::new("ab\ncd", 18.0, FontWeight::Regular, false);
        let first = layout.caret_rect(0);
        let second = layout.caret_rect(3);
        assert!(second.y > first.y, "la 2e ligne est plus bas");
        assert_eq!(second.x, 0.0, "début de 2e ligne à x = 0");
        // Hit dans la 2e ligne → indices de la 2e ligne.
        let hit = layout.hit_test(Point::new(0.0, second.y + 1.0));
        assert_eq!(hit, 3);
    }

    #[test]
    fn empty_layout_keeps_caret_at_origin() {
        let layout = TextLayout::new("", 18.0, FontWeight::Regular, false);
        let caret = layout.caret_rect(0);
        assert_eq!(caret.x, 0.0);
        assert!(caret.height > 0.0, "hauteur de ligne de repli");
        assert_eq!(layout.hit_test(Point::new(50.0, 0.0)), 0);
    }

    #[test]
    fn bold_measures_wider_than_regular() {
        // La face grasse embarquée doit réellement être choisie : un gras est
        // plus large qu'un normal à taille égale.
        let regular = measure_styled("Bonjour le monde", 24.0, FontWeight::Regular, false);
        let bold = measure_styled("Bonjour le monde", 24.0, FontWeight::Bold, false);
        assert!(
            bold.width > regular.width,
            "gras {} <= normal {}",
            bold.width,
            regular.width
        );
    }
}
