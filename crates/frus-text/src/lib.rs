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
/// Faces **grasse, oblique et grasse-oblique** embarquées : cosmic-text exige une
/// correspondance **exacte** de style sur la famille primaire — sans face
/// oblique, un simple `.italic()` **panique** (« no default font found ») partout
/// où seules les polices embarquées existent (attrapé sur l'appareil Android).
/// La matrice complète {400, 700} × {droit, italique} rend tous les styles
/// atteignables par l'API sûrs et déterministes.
const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf");
const DEJAVU_SANS_OBLIQUE: &[u8] = include_bytes!("../assets/DejaVuSans-Oblique.ttf");
const DEJAVU_SANS_BOLD_OBLIQUE: &[u8] = include_bytes!("../assets/DejaVuSans-BoldOblique.ttf");
const DEJAVU_MONO: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
/// **Arabe** (Noto Naskh) : DejaVu ne couvre pas l'écriture arabe (pas de formes
/// de jonction contextuelles) ; cette face fournit le repli pour les runs
/// arabes, embarquée pour un rendu déterministe partout (y compris Android, où
/// aucune police système n'est chargée).
const NOTO_ARABIC: &[u8] = include_bytes!("../assets/NotoNaskhArabic-Regular.ttf");
const NOTO_ARABIC_BOLD: &[u8] = include_bytes!("../assets/NotoNaskhArabic-Bold.ttf");

/// Nom de famille interne des polices embarquées (doit correspondre aux TTF).
const SANS_FAMILY: &str = "DejaVu Sans";
const MONO_FAMILY: &str = "DejaVu Sans Mono";
/// Famille de la face arabe embarquée (Noto Naskh).
const ARABIC_FAMILY: &str = "Noto Naskh Arabic";

/// `true` si `text` contient au moins un caractère de l'écriture **arabe**
/// (blocs Arabic, Supplement, Extended-A, Presentation Forms A/B).
fn contains_arabic(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
    })
}

/// La **famille de police** à employer pour `text` : la face arabe embarquée si
/// le texte contient de l'arabe, sinon la sans-serif par défaut.
///
/// Indispensable car cosmic-text ne fait **pas** de repli cross-famille sur
/// Android (listes de fallback plateforme vides) : sans assignation explicite,
/// un run arabe ne rendrait rien. On choisit donc la famille par script à la
/// source (mesure **et** rendu partagent cette règle).
pub fn family_for(text: &str) -> cosmic_text::Family<'static> {
    if contains_arabic(text) {
        cosmic_text::Family::Name(ARABIC_FAMILY)
    } else {
        cosmic_text::Family::Name(SANS_FAMILY)
    }
}

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
    db.load_font_data(DEJAVU_SANS_OBLIQUE.to_vec());
    db.load_font_data(DEJAVU_SANS_BOLD_OBLIQUE.to_vec());
    db.load_font_data(DEJAVU_MONO.to_vec());
    db.load_font_data(NOTO_ARABIC.to_vec());
    db.load_font_data(NOTO_ARABIC_BOLD.to_vec());
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

/// Poids **réellement disponible** dans les faces embarquées (400 ou 700) le
/// plus proche du poids demandé. Indispensable : cosmic-text exige une
/// correspondance **exacte** de poids sur la famille primaire — un poids absent
/// (Medium 500 sur DejaVu) le fait basculer sur des listes de repli plateforme…
/// inexistantes sur Android (panique « no default font found », attrapée sur
/// l'appareil). Router tous les `Attrs` par ici garantit un rendu déterministe.
pub fn available_weight(weight: FontWeight) -> u16 {
    if weight.to_u16() < 550 {
        400
    } else {
        700
    }
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
    measure_wrapped(text, size_px, weight, italic, None)
}

/// Mesure un texte stylé **sous contrainte de largeur** : au-delà de
/// `max_width`, le texte se replie à la ligne (la hauteur grandit). `None` =
/// non contraint (taille naturelle). C'est la mesure branchée sur la closure de
/// mesure de taffy pour les paragraphes.
pub fn measure_wrapped(
    text: &str,
    size_px: f32,
    weight: FontWeight,
    italic: bool,
    max_width: Option<f32>,
) -> Size {
    let line_h = line_height(size_px);
    if text.is_empty() {
        return Size::new(0.0, line_h);
    }

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(size_px, line_h);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    // Largeur contrainte (repli) ou libre ; hauteur toujours libre.
    buffer.set_size(&mut font_system, max_width, None);
    let attrs = Attrs::new()
        .family(family_for(text))
        .weight(Weight(available_weight(weight)))
        .style(if italic { Style::Italic } else { Style::Normal });
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
    measure_runs_wrapped(runs, None)
}

/// Mesure un texte riche **sous contrainte de largeur** : au-delà de
/// `max_width`, les runs reviennent à la ligne. `None` = non contraint.
pub fn measure_runs_wrapped(runs: &[TextRun], max_width: Option<f32>) -> Size {
    if runs.iter().all(|r| r.text.is_empty()) {
        return Size::new(0.0, 0.0);
    }
    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(base, line_height(base));
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, max_width, None);
    let spans = runs.iter().map(|run| {
        (
            run.text.as_str(),
            Attrs::new()
                .family(family_for(&run.text))
                .weight(Weight(available_weight(run.weight)))
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
        Self::wrapped(text, size_px, weight, italic, None)
    }

    /// Comme [`TextLayout::new`], mais **replie** le texte à `max_width` (repli doux,
    /// façon champ multi-lignes). `None` = non contraint (seuls les `\n` coupent).
    ///
    /// Chaque **ligne visuelle** (un `LayoutRun` cosmic-text) est délimitée par les
    /// **octets de ses glyphes**, pas par `run.text` (qui est la ligne *dure*
    /// entière, répétée pour chaque repli) : un repli doux ne fabrique donc aucun
    /// caractère fantôme, et le `start_char` de chaque ligne vient du décalage
    /// d'octet de sa ligne dure — indexage exact à travers les replis.
    pub fn wrapped(
        text: &str,
        size_px: f32,
        weight: FontWeight,
        italic: bool,
        max_width: Option<f32>,
    ) -> Self {
        let fallback_h = line_height(size_px);
        let mut lines: Vec<LayoutLine> = Vec::new();
        let mut width = 0.0_f32;
        let mut height = 0.0_f32;

        if !text.is_empty() {
            // Décalage d'octet du début de chaque ligne **dure** (séparée par `\n`).
            let hard: Vec<&str> = text.split('\n').collect();
            let mut line_byte_start = Vec::with_capacity(hard.len());
            {
                let mut b = 0usize;
                for l in &hard {
                    line_byte_start.push(b);
                    b += l.len() + 1; // +1 : le `\n`
                }
            }

            let mut font_system = font_system().lock().expect("FontSystem lock");
            let metrics = Metrics::new(size_px, fallback_h);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, max_width, None);
            let attrs = Attrs::new()
                .family(family_for(text))
                .weight(Weight(available_weight(weight)))
                .style(if italic { Style::Italic } else { Style::Normal });
            buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
            buffer.shape_until_scroll(&mut font_system, false);

            // On collecte les runs pour regarder le suivant (fin du segment courant).
            let runs: Vec<_> = buffer.layout_runs().collect();
            for (idx, run) in runs.iter().enumerate() {
                let line_text = hard[run.line_i];
                let first_of_line = idx == 0 || runs[idx - 1].line_i != run.line_i;
                // Segment [lo, hi) de la ligne dure (octets) que cette ligne visuelle
                // porte : de son premier glyphe (0 si première visuelle) au premier
                // glyphe de la visuelle suivante de la même ligne dure — sinon la fin
                // (englobe ainsi l'espace de coupure, retiré des glyphes).
                let lo = if first_of_line {
                    0
                } else {
                    run.glyphs.iter().map(|g| g.start).min().unwrap_or(0)
                };
                let hi = runs
                    .get(idx + 1)
                    .filter(|r| r.line_i == run.line_i)
                    .and_then(|r| r.glyphs.iter().map(|g| g.start).min())
                    .unwrap_or(line_text.len());

                // Frontières de caractères du segment (octets relatifs à la ligne dure).
                let span = &line_text[lo..hi];
                let char_bytes: Vec<usize> = span.char_indices().map(|(b, _)| lo + b).collect();
                let n = char_bytes.len();
                let mut offsets = vec![f32::NAN; n + 1];
                offsets[0] = 0.0;

                // Chaque glyphe couvre un cluster d'octets [start, end) sur [x, x+w)
                // (x **local à la ligne visuelle**) ; frontières internes interpolées.
                for glyph in run.glyphs.iter() {
                    let covered: Vec<usize> = (0..n)
                        .filter(|&i| char_bytes[i] >= glyph.start && char_bytes[i] < glyph.end)
                        .collect();
                    let k = covered.len().max(1) as f32;
                    for (j, &i) in covered.iter().enumerate() {
                        offsets[i] = glyph.x + glyph.w * (j as f32 / k);
                    }
                    if let Some(next) = (0..=n).find(|&i| {
                        let b = if i == n { lo + span.len() } else { char_bytes[i] };
                        b == glyph.end
                    }) {
                        offsets[next] = glyph.x + glyph.w;
                    }
                }
                // Frontières restées sans glyphe (espace de coupure…) : continuité.
                for i in 1..=n {
                    if offsets[i].is_nan() {
                        offsets[i] = offsets[i - 1];
                    }
                }

                let start_char = text[..line_byte_start[run.line_i] + lo].chars().count();
                width = width.max(run.line_w);
                height = height.max(run.line_top + run.line_height);
                lines.push(LayoutLine {
                    start_char,
                    offsets,
                    top: run.line_top,
                    height: run.line_height,
                });
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
            decoration: frus_core::TextDecoration::NONE,
            decoration_color: None,
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
    fn wrapped_text_grows_taller_within_the_width() {
        let text = "un texte assez long pour se replier sur plusieurs lignes";
        let free = measure_wrapped(text, 16.0, FontWeight::Regular, false, None);
        let narrow = measure_wrapped(text, 16.0, FontWeight::Regular, false, Some(120.0));
        assert!(narrow.width <= 120.0, "replié dans la largeur : {}", narrow.width);
        assert!(narrow.height > free.height, "le repli grandit la hauteur");
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
    fn soft_wrap_indexes_chars_correctly_across_lines() {
        // Repli doux sans `\n` : chaque mot sur sa ligne visuelle. L'indexage doit
        // rester exact — le caret d'un index tombe sur la bonne ligne, et les débuts
        // de lignes sont contigus (aucun caractère fantôme au point de coupure).
        let text = "aaaa bbbb cccc dddd"; // 19 caractères, un mot par ligne à 60 px
        let layout = TextLayout::wrapped(text, 18.0, FontWeight::Regular, false, Some(60.0));
        assert!(layout.size().height > line_height(18.0) * 2.0, "plusieurs lignes visuelles");

        // Début de chaque mot : "aaaa"@0, "bbbb"@5, "cccc"@10, "dddd"@15 — chacun à
        // x ≈ 0 (début de sa ligne), sur des lignes de y croissant.
        let mut prev_y = -1.0;
        for &start in &[0usize, 5, 10, 15] {
            let c = layout.caret_rect(start);
            assert!(c.x < 1.0, "début de mot {start} à x≈0 (x={})", c.x);
            assert!(c.y > prev_y, "lignes de y croissant à {start}");
            prev_y = c.y;
        }
        // Un point au **milieu** d'une ligne repliée fait l'aller-retour (index 11 =
        // 2e 'c' de "cccc", clairement pas une frontière de coupure).
        let c = layout.caret_rect(11);
        assert_eq!(layout.hit_test(Point::new(c.x, c.y + 1.0)), 11);
        // La dernière frontière = 19 (aucun +1 parasite injecté par les 3 replis).
        assert_eq!(layout.caret_rect(19).x, layout.caret_rect(99).x);
        assert_eq!(layout.caret_rect(19).y, layout.caret_rect(99).y);
    }

    #[test]
    fn empty_layout_keeps_caret_at_origin() {
        let layout = TextLayout::new("", 18.0, FontWeight::Regular, false);
        let caret = layout.caret_rect(0);
        assert_eq!(caret.x, 0.0);
        assert!(caret.height > 0.0, "hauteur de ligne de repli");
        assert_eq!(layout.hit_test(Point::new(50.0, 0.0)), 0);
    }

    /// Reproduit le pire cas Android (attrapé sur l'appareil) : **aucune**
    /// police système exploitable, seules les faces embarquées existent. Le
    /// shaping ne doit jamais paniquer (« no default font found ») — pour
    /// **toute** combinaison poids × italique atteignable par l'API. cosmic-text
    /// exige une correspondance *exacte* de style/poids sur la famille primaire :
    /// sans face oblique embarquée, `.italic()` paniquait sur l'appareil.
    #[test]
    fn embedded_only_font_system_shapes_every_style() {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
        db.load_font_data(DEJAVU_SANS_OBLIQUE.to_vec());
        db.load_font_data(DEJAVU_SANS_BOLD_OBLIQUE.to_vec());
        db.load_font_data(DEJAVU_MONO.to_vec());
        db.set_sans_serif_family(SANS_FAMILY);
        db.set_serif_family(SANS_FAMILY);
        db.set_cursive_family(SANS_FAMILY);
        db.set_fantasy_family(SANS_FAMILY);
        db.set_monospace_family(MONO_FAMILY);
        let mut fs = FontSystem::new_with_locale_and_db("en-TG".to_string(), db);

        for weight in [FontWeight::Regular, FontWeight::Medium, FontWeight::SemiBold, FontWeight::Bold] {
            for italic in [false, true] {
                let attrs = Attrs::new()
                    .weight(Weight(available_weight(weight)))
                    .style(if italic { Style::Italic } else { Style::Normal });
                let mut buffer = Buffer::new(&mut fs, Metrics::new(20.0, 24.0));
                buffer.set_size(&mut fs, None, None);
                buffer.set_text(&mut fs, "Nothing to show", attrs, Shaping::Advanced);
                buffer.shape_until_scroll(&mut fs, false); // panique ici si cassé
                let w: f32 = buffer.layout_runs().map(|r| r.line_w).fold(0.0, f32::max);
                assert!(w > 0.0, "poids {weight:?} italique {italic} : rien shapé");
            }
        }
    }

    /// Reproduit **exactement** le cas Android pour l'arabe : db embarquée
    /// seule (aucune police système, donc aucune liste de repli plateforme) —
    /// avec la face Noto Naskh chargée. `family_for` doit router le run arabe
    /// vers la famille Naskh, et le shaping doit produire de **vrais** glyphes
    /// (identifiants non nuls), pas des `.notdef`. Si `Family::Name` ne résout
    /// pas ici, on obtient des glyphes vides — le blanc observé sur l'appareil.
    #[test]
    fn arabic_shapes_with_embedded_only_font_system() {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(NOTO_ARABIC.to_vec());
        db.load_font_data(NOTO_ARABIC_BOLD.to_vec());
        db.set_sans_serif_family(SANS_FAMILY);
        let mut fs = FontSystem::new_with_locale_and_db("en-TG".to_string(), db);

        let text = "مهامي";
        let attrs = Attrs::new().family(family_for(text));
        let mut buffer = Buffer::new(&mut fs, Metrics::new(40.0, 48.0));
        buffer.set_size(&mut fs, None, None);
        buffer.set_text(&mut fs, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut fs, false);

        let mut glyphs = 0usize;
        let mut real = 0usize; // glyphes dont le glyph_id != 0 (pas .notdef)
        for run in buffer.layout_runs() {
            for g in run.glyphs.iter() {
                glyphs += 1;
                if g.glyph_id != 0 {
                    real += 1;
                }
            }
        }
        assert!(glyphs > 0, "aucun glyphe shapé pour l'arabe");
        assert!(
            real > 0,
            "seulement des .notdef ({glyphs} glyphes, 0 réel) : Family::Name(\"{ARABIC_FAMILY}\") n'a pas résolu la face Naskh"
        );
    }

    /// Diagnostic de position : un run RTL dans un buffer **large** (largeur =
    /// surface) se fait **aligner à droite** par cosmic-text → les glyphes
    /// atterrissent près du bord droit (x ≈ largeur), donc hors écran une fois
    /// décalés par `position.x`. Sans contrainte de largeur (`None`), ils
    /// commencent à x ≈ 0. C'est la cause du blanc arabe sur l'appareil.
    #[test]
    fn rtl_right_aligns_to_buffer_width() {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(NOTO_ARABIC.to_vec());
        db.set_sans_serif_family(SANS_FAMILY);
        let mut fs = FontSystem::new_with_locale_and_db("en-TG".to_string(), db);
        let text = "العربية";
        let attrs = Attrs::new().family(family_for(text));

        let mut first_glyph_x = |width: Option<f32>| {
            let mut buffer = Buffer::new(&mut fs, Metrics::new(40.0, 48.0));
            buffer.set_size(&mut fs, width, Some(200.0));
            buffer.set_text(&mut fs, text, attrs.clone(), Shaping::Advanced);
            buffer.shape_until_scroll(&mut fs, false);
            buffer
                .layout_runs()
                .flat_map(|r| r.glyphs.iter().map(|g| g.x))
                .fold(f32::MAX, f32::min)
        };

        let wide = first_glyph_x(Some(1080.0));
        let free = first_glyph_x(None);
        assert!(wide > 500.0, "RTL large devrait pousser à droite (x={wide})");
        assert!(free < 50.0, "RTL non contraint devrait commencer à gauche (x={free})");
    }

    #[test]
    fn arabic_falls_back_to_the_embedded_naskh_face() {
        // DejaVu ne couvre pas l'arabe : le repli embarqué (Noto Naskh) doit
        // prendre le relais et **façonner** les glyphes (largeur non nulle,
        // sensible). Un repli manquant donnerait 0 ou des .notdef.
        let hello = "مرحبا بالعالم"; // « bonjour le monde »
        let m = measure(hello, 24.0);
        assert!(m.width > 60.0, "l'arabe doit se façonner (largeur {})", m.width);
        assert!(m.height > 0.0);

        // Le repli n'écrase pas le latin : « Hello » garde une largeur cohérente.
        let latin = measure("Hello", 24.0);
        assert!(latin.width > 0.0);

        // Bidi mixte (arabe + chiffres latins) : mesuré d'un seul tenant, sans
        // panique (cosmic-text réordonne en interne).
        let mixed = measure("قيمة 42 نقطة", 24.0);
        assert!(mixed.width > 0.0, "texte bidi mixte non shapé");
    }

    #[test]
    fn weights_snap_to_embedded_faces() {
        // Un poids sans face exacte (Medium 500) DOIT se rabattre sur une face
        // embarquée — sinon cosmic-text bascule sur des listes de repli
        // plateforme, inexistantes sur Android (panique sur l'appareil).
        assert_eq!(available_weight(FontWeight::Regular), 400);
        assert_eq!(available_weight(FontWeight::Medium), 400);
        assert_eq!(available_weight(FontWeight::SemiBold), 700);
        assert_eq!(available_weight(FontWeight::Bold), 700);
        // Et le shaping est déterministe : Medium mesure comme Regular (même
        // face), SemiBold comme Bold.
        let text = "Titre de section";
        let regular = measure_styled(text, 20.0, FontWeight::Regular, false);
        let medium = measure_styled(text, 20.0, FontWeight::Medium, false);
        assert_eq!(medium.width, regular.width);
        let semibold = measure_styled(text, 20.0, FontWeight::SemiBold, false);
        let bold = measure_styled(text, 20.0, FontWeight::Bold, false);
        assert_eq!(semibold.width, bold.width);
        assert!(bold.width > regular.width);
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
