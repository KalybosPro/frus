//! Tests **au pixel** de la découpe en forme d'un calque
//! ([`frus_core::Primitive::Layer`] + [`frus_core::ClipShape`]) : un calque plein
//! est composité par le GPU avec une découpe **arrondie** ou **elliptique**, et on
//! vérifie que les pixels **hors de la forme** sont gommés tandis que le cœur reste
//! peint. C'est la pièce que les tests de `frus-widgets` ne couvrent pas : ils
//! prouvent que le bon [`frus_core::ClipShape`] est émis ; ceux-ci prouvent que le
//! shader de compositing (`composite.wgsl`) l'**applique** (couverture SDF).
//!
//! Sans adaptateur GPU, `render_scene` renvoie `None` et le test s'ignore.
//!
//! Convention : origine haut-gauche, y vers le bas.

use frus_core::{BorderRadius, ClipShape, Color, Primitive, Rect, Scene};
use frus_test::{render_scene, Snapshot};

/// Une scène : un rectangle plein `inner` (rouge) enveloppé dans un calque découpé
/// à la forme `shape`, dont la boîte est `inner` lui-même.
fn clipped_layer(inner: Rect, shape: ClipShape) -> Scene {
    let mut content = Scene::new();
    content.fill_rect(inner, Color::rgb(1.0, 0.0, 0.0));
    let primitives = content.split_off(0);
    let mut scene = Scene::new();
    scene.push_primitive(Primitive::Layer {
        primitives,
        opacity: 1.0,
        clip: inner,
        clip_shape: shape,
        transform: None,
        owner: 0,
    });
    scene
}

fn is_red(px: [u8; 4]) -> bool {
    px[0] > 200 && px[1] < 60 && px[2] < 60
}

fn is_clear(px: [u8; 4]) -> bool {
    px[0] < 60 && px[1] < 60 && px[2] < 60
}

/// Faute d'adaptateur, on s'ignore proprement (comme les autres tests de rendu).
fn render(scene: &Scene) -> Option<Snapshot> {
    let frame = render_scene(scene, 64, 64, Color::BLACK);
    if frame.is_none() {
        eprintln!("aucun adaptateur GPU disponible : test ignoré");
    }
    frame
}

/// **Coins arrondis.** Un carré plein découpé à un grand rayon perd ses coins : le
/// centre reste peint, mais le coin haut-gauche du carré est gommé (transparent).
#[test]
fn rrect_clip_rounds_off_the_corners() {
    // Carré centré 40×40 : x, y ∈ [12, 52].
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0);
    let m = ClipShape::RRect(BorderRadius::uniform(16.0));
    let Some(frame) = render(&clipped_layer(sq, m)) else { return };

    // Le cœur et les milieux de bord restent peints.
    assert!(is_red(frame.pixel(32, 32)), "centre → rouge");
    assert!(is_red(frame.pixel(32, 14)), "milieu du bord haut → rouge");
    assert!(is_red(frame.pixel(14, 32)), "milieu du bord gauche → rouge");
    // Les coins (dans le carré, hors du rayon) sont gommés.
    assert!(is_clear(frame.pixel(14, 14)), "coin haut-gauche → gommé par l'arrondi");
    assert!(is_clear(frame.pixel(50, 50)), "coin bas-droit → gommé par l'arrondi");
}

/// **Rayon par coin.** Seul le coin **haut-gauche** est arrondi (rayon 16) ; les trois
/// autres restent **nets**. Le coin haut-gauche est gommé, les autres conservés.
#[test]
fn rrect_clip_rounds_only_the_specified_corner() {
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0); // x, y ∈ [12, 52]
    let br = BorderRadius { top_left: 16.0, top_right: 0.0, bottom_right: 0.0, bottom_left: 0.0 };
    let Some(frame) = render(&clipped_layer(sq, ClipShape::RRect(br))) else { return };

    assert!(is_clear(frame.pixel(15, 15)), "coin haut-gauche → arrondi (gommé)");
    assert!(is_red(frame.pixel(49, 15)), "coin haut-droit → net (conservé)");
    assert!(is_red(frame.pixel(15, 49)), "coin bas-gauche → net (conservé)");
    assert!(is_red(frame.pixel(49, 49)), "coin bas-droit → net (conservé)");
}

/// **Ellipse.** Un carré plein découpé en ovale garde son centre mais perd ses
/// quatre coins (hors du disque inscrit).
#[test]
fn oval_clip_keeps_the_inscribed_disc() {
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0); // centre (32, 32), rayon 20
    let Some(frame) = render(&clipped_layer(sq, ClipShape::Oval)) else { return };

    assert!(is_red(frame.pixel(32, 32)), "centre → rouge");
    assert!(is_red(frame.pixel(32, 14)), "haut du disque → rouge");
    assert!(is_red(frame.pixel(50, 32)), "droite du disque → rouge");
    // Coins du carré : hors du disque inscrit → gommés.
    assert!(is_clear(frame.pixel(15, 15)), "coin haut-gauche → hors du disque");
    assert!(is_clear(frame.pixel(49, 49)), "coin bas-droit → hors du disque");
}

/// **Rayon nul = rectangle.** `RRect(0)` ne gomme aucun coin : le carré reste plein
/// (garde-fou : la forme arrondie dégénère bien vers la découpe rectangulaire).
#[test]
fn rrect_zero_radius_is_a_plain_rect() {
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0);
    let Some(frame) = render(&clipped_layer(sq, ClipShape::RRect(BorderRadius::uniform(0.0)))) else { return };

    assert!(is_red(frame.pixel(14, 14)), "coin conservé (rayon nul)");
    assert!(is_red(frame.pixel(50, 50)), "coin conservé (rayon nul)");
    // Hors du carré : rien.
    assert!(is_clear(frame.pixel(4, 4)), "hors de la boîte → fond");
}
