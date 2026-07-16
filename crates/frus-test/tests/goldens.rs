//! Tests du harnais lui-même : snapshot de scène, rendu de widgets, goldens.
//!
//! Sans adaptateur GPU les tests s'ignorent (le harnais renvoie `None`).

use frus_core::{Color, Point, Rect, Scene, TextStyle};
use frus_test::{render_scene, render_widget};
use frus_widgets::{Container, Flex, Text, Theme};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Une scène mixte (rect arrondi + texte décoré) reproduit son golden à
/// l'identique — le pipeline entier est déterministe dans cet environnement.
#[test]
fn scene_matches_golden() {
    let mut scene = Scene::new();
    scene.draw_rect(
        Rect::new(8.0, 8.0, 104.0, 48.0),
        Color::rgb8(46, 160, 96),
        10.0,
        0.0,
        Color::TRANSPARENT,
    );
    scene.text_styled(
        Point::new(16.0, 20.0),
        "Golden",
        &TextStyle::new(20.0).underline(),
        Color::WHITE,
    );
    let Some(snapshot) = render_scene(&scene, 120, 64, Color::BLACK) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    // Sanité avant golden : le fond est noir, le rect est bien dessiné.
    assert_eq!(snapshot.pixel(2, 2), [0, 0, 0, 255], "coin = clear");
    assert!(snapshot.lit_pixels(16) > 100, "rect + texte dessinés");
    snapshot.assert_golden(golden("scene_rect_text"));
}

/// Un arbre de widgets rend comme le ferait le shell (layout + thème), et
/// reproduit son golden.
#[test]
fn widget_tree_matches_golden() {
    let theme = Theme::dark();
    let root: Container<()> = Container::new().padding(12.0).child(
        Flex::column()
            .gap(8.0)
            .child(Text::styled("Title", theme.text.title_medium))
            .child(Text::new("done item").strikethrough().size(14.0)),
    );
    let Some(snapshot) = render_widget(&root, 160, 80, &theme) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    assert!(snapshot.lit_pixels(40) > 50, "du texte est dessiné");
    snapshot.assert_golden(golden("widget_column_text"));
}

/// Le comparateur : identique → 0 diff ; un pixel changé → 1 diff.
#[test]
fn diff_count_is_exact() {
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), Color::rgb(0.3, 0.5, 0.7));
    let Some(a) = render_scene(&scene, 64, 64, Color::BLACK) else {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
        return;
    };
    let mut b = render_scene(&scene, 64, 64, Color::BLACK).unwrap();
    assert_eq!(a.diff_count(&b, 0), 0, "deux rendus identiques");
    // Corrompt un pixel au-delà de la tolérance.
    b.rgba[0] = b.rgba[0].wrapping_add(64);
    assert_eq!(a.diff_count(&b, 2), 1);
    assert_eq!(a.diff_count(&b, 255), 0, "tolérance maximale absorbe tout");
}
