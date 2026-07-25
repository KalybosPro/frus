//! Tests **au pixel** du pipeline de transformation : un calque
//! ([`frus_core::Primitive::Layer`]) porteur d'une matrice affine est composité
//! par le GPU, et on vérifie que les pixels atterrissent **là où la matrice le
//! prédit**. C'est la pièce que les tests de `frus-widgets` ne peuvent pas couvrir :
//! ils prouvent que la *bonne matrice* est émise ; ceux-ci prouvent que le shader de
//! compositing (`composite.wgsl`) la **rend correctement** (échantillonnage à `M⁻¹`).
//!
//! Sans adaptateur GPU, `render_scene` renvoie `None` et le test s'ignore.
//!
//! Convention : origine haut-gauche, y vers le bas, rotation **horaire**.

use std::f32::consts::FRAC_PI_2;

use frus_core::{Affine, Color, LayerTransform, Point, Primitive, Rect, Scene};
use frus_test::{render_scene, Snapshot};

/// Une scène : un rectangle plein `inner`, de couleur `color`, enveloppé dans un
/// calque transformé par `m` (rendu à plat puis composité transformé).
fn transformed_layer(inner: Rect, color: Color, m: LayerTransform) -> Scene {
    let mut content = Scene::new();
    content.fill_rect(inner, color);
    let primitives = content.split_off(0);
    let mut scene = Scene::new();
    scene.push_primitive(Primitive::Layer {
        primitives,
        opacity: 1.0,
        clip: Rect::UNBOUNDED,
        transform: Some(m),
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

/// **Rotation.** Une barre **horizontale** tournée de +90° autour du centre devient
/// **verticale** : ses pixels sont là où serait la barre verticale, et plus là où
/// était l'horizontale.
#[test]
fn rotation_turns_a_horizontal_bar_vertical() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    // Barre horizontale : x ∈ [8,56], y ∈ [28,36].
    let bar = Rect::new(8.0, 28.0, 48.0, 8.0);
    let m = LayerTransform::rotation(FRAC_PI_2, Point::new(32.0, 32.0));
    let Some(frame) = render(&transformed_layer(bar, red, m)) else { return };

    // Après +90° la barre est verticale : x ∈ [28,36], y ∈ [8,56].
    assert!(is_red(frame.pixel(32, 12)), "haut de la barre verticale → rouge");
    assert!(is_red(frame.pixel(32, 52)), "bas de la barre verticale → rouge");
    assert!(is_red(frame.pixel(32, 32)), "centre → rouge");
    // Là où était la barre horizontale (mais plus la verticale) → fond.
    assert!(is_clear(frame.pixel(12, 32)), "ancien bord gauche → fond (a tourné)");
    assert!(is_clear(frame.pixel(52, 32)), "ancien bord droit → fond (a tourné)");
}

/// **Échelle uniforme.** Un petit carré ×2 autour du centre couvre une aire quatre
/// fois plus grande : un point hors du carré d'origine mais dans son image est peint.
#[test]
fn uniform_scale_enlarges_about_center() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    // Carré 16×16 centré : x,y ∈ [24,40].
    let sq = Rect::new(24.0, 24.0, 16.0, 16.0);
    let m = LayerTransform::new(Affine::scale(2.0, 2.0).about(Point::new(32.0, 32.0)));
    let Some(frame) = render(&transformed_layer(sq, red, m)) else { return };

    // Image ×2 : x,y ∈ [16,48].
    assert!(is_red(frame.pixel(20, 20)), "dans l'image agrandie (hors carré d'origine) → rouge");
    assert!(is_red(frame.pixel(32, 32)), "centre → rouge");
    assert!(is_clear(frame.pixel(6, 6)), "hors de l'image → fond");
}

/// **Échelle non uniforme.** `scale(3, 1)` élargit le carré en x sans toucher y :
/// un point élargi horizontalement est peint, un point décalé en y ne l'est pas.
#[test]
fn non_uniform_scale_widens_x_only() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    let sq = Rect::new(24.0, 24.0, 16.0, 16.0); // x,y ∈ [24,40]
    let m = LayerTransform::new(Affine::scale(3.0, 1.0).about(Point::new(32.0, 32.0)));
    let Some(frame) = render(&transformed_layer(sq, red, m)) else { return };

    // Image : x ∈ [8,56] (×3), y ∈ [24,40] (inchangé).
    assert!(is_red(frame.pixel(12, 32)), "élargi en x → rouge");
    assert!(is_clear(frame.pixel(32, 12)), "y non mis à l'échelle → fond au-dessus");
    assert!(is_clear(frame.pixel(4, 32)), "au-delà de l'élargissement → fond");
}

/// **Composition.** `scale ×2` **puis** rotation +90° (une seule matrice) : le carré
/// agrandi *et* tourné couvre l'image attendue — l'échantillonnage `M⁻¹` compose bien.
#[test]
fn scale_then_rotate_composes() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    // Carré 16×8 (large et court) centré : x ∈ [24,40], y ∈ [28,36].
    let sq = Rect::new(24.0, 28.0, 16.0, 8.0);
    let pivot = Point::new(32.0, 32.0);
    // échelle ×2 (→ 32×16) puis rotation +90° (→ 16 large × 32 haut).
    let m = LayerTransform::new(
        Affine::scale(2.0, 2.0)
            .about(pivot)
            .then(Affine::rotation(FRAC_PI_2).about(pivot)),
    );
    let Some(frame) = render(&transformed_layer(sq, red, m)) else { return };

    // Après ×2 : x ∈ [16,48], y ∈ [24,40]. Après +90° autour du centre :
    // x ∈ [24,40], y ∈ [16,48] — haut et étroit.
    assert!(is_red(frame.pixel(32, 20)), "haut de l'image composée → rouge");
    assert!(is_red(frame.pixel(32, 44)), "bas de l'image composée → rouge");
    assert!(is_clear(frame.pixel(12, 32)), "hors de l'image étroite → fond");
    assert!(is_clear(frame.pixel(52, 32)), "hors de l'image étroite → fond");
}
