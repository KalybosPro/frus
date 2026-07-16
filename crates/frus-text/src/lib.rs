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
use frus_core::{FontWeight, Size};

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
