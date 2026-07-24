//! `frus-transforms` — une vitrine **animée** de l'arsenal de disposition/peinture :
//! la palette [`Transform`] (translation, échelle non uniforme, rotation + échelle
//! **composées**), [`AspectRatio`] et [`FractionallySizedBox`], le tout piloté par un
//! [`Tween`] au fil du temps.
//!
//! C'est la première démo *tangible* de la couche de transformation : on y **voit**
//! la rotation et l'échelle rendues par le GPU (au-delà des tests headless).
//!
//! Modèle Elm au complet : un état minuscule (le temps écoulé), un `update` pur qui
//! avance l'horloge, une souscription qui bat la mesure (~60 fps), et une `view` pure
//! qui reconstruit la scène pour l'instant courant.
//!
//! Lancer sur bureau : `cargo run -p frus-transforms`.

use std::time::Duration;

use frus_shell::{Application, Command, Subscription};
use frus_widgets::{
    AspectRatio, Color, Container, Curve, Flex, FractionallySizedBox, Align, Justify, Text, Theme,
    Transform, Tween, Widget,
};

/// Pas de temps fixe par image (~60 fps) : garde `update` **déterministe** et
/// testable, tout en suivant le rythme de la souscription.
const FRAME_DT: f32 = 1.0 / 60.0;

/// Durée d'un cycle d'aller-retour (échelle, largeur fractionnaire), en secondes.
const CYCLE: f32 = 2.4;

/// L'état : le temps écoulé depuis le lancement, en secondes.
#[derive(Default)]
struct Showcase {
    time: f32,
}

/// Les messages : une seule pulsation d'horloge.
#[derive(Clone)]
enum Msg {
    /// Une image est passée : avance l'horloge.
    Frame,
}

impl Application for Showcase {
    type Message = Msg;

    /// `update` est **pur** : il avance l'horloge d'un pas fixe. Aucun effet.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Frame => self.time += FRAME_DT,
        }
        Command::none()
    }

    /// Bat la mesure : ~60 images par seconde, tant que la fenêtre est ouverte.
    fn subscription(&self) -> Subscription<Msg> {
        Subscription::every(Duration::from_millis(16), |_| Msg::Frame)
    }

    /// `view` est une fonction **pure** de l'instant : elle recalcule les valeurs
    /// animées puis reconstruit la scène. Les transformations sortent d'un [`Tween`].
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        // Phase d'aller-retour `0 → 1 → 0` sur un cycle, adoucie.
        let phase = (self.time % CYCLE) / CYCLE;
        let ping = 1.0 - (2.0 * phase - 1.0).abs();
        let eased = Curve::ease_in_out().transform(ping);

        // Valeurs animées, chacune interpolée par un Tween ou une sinusoïde.
        let angle = self.time * 0.9; // rotation continue (rad)
        let scale = Tween::new(1.0, 1.4).eval(eased); // pulsation
        let bob = (self.time * 2.2).sin() * 22.0; // va-et-vient vertical (px)
        let squash = (self.time * 2.6).sin() * 0.35; // écrasement/étirement
        let width_factor = Tween::new(0.25, 1.0).eval(eased); // barre fractionnaire

        // Une tuile : un carré transformé, centré dans une scène fixe (marge pour
        // que la transformation déborde sans bousculer les voisines), sous-titré.
        let tile = |inner: Box<dyn Widget<Msg>>, label: &str| {
            Flex::column()
                .gap(10.0)
                .align(Align::Center)
                .child(
                    Flex::column()
                        .width(128.0)
                        .height(128.0)
                        .justify(Justify::Center)
                        .align(Align::Center)
                        .child(inner),
                )
                .child(Text::new(label).size(12.0).color(theme.on_surface))
        };
        let square = |color: Color| Container::<Msg>::new().width(64.0).height(64.0).color(color).radius(14.0);

        // Galerie : la palette complète de `Transform` côte à côte.
        // - translate : va-et-vient vertical (décalage de peinture pur).
        // - scale_xy : écrasement/étirement (échelle **non uniforme**, opposée en x/y).
        // - rotate + scale : la **composition** en une seule matrice.
        let gallery = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(
                Box::new(Transform::translate(0.0, bob).child(square(theme.primary))),
                "translate",
            ))
            .child(tile(
                Box::new(
                    Transform::scale_xy(1.0 + squash, 1.0 - squash)
                        .child(square(theme.scheme.secondary)),
                ),
                "scale_xy",
            ))
            .child(tile(
                Box::new(
                    Transform::rotate(angle).and_scale(scale).child(
                        Container::<Msg>::new()
                            .width(64.0)
                            .height(64.0)
                            .color(theme.primary)
                            .gradient(theme.scheme.secondary, [1.0, 1.0])
                            .radius(14.0),
                    ),
                ),
                "rotate + scale",
            ));

        // AspectRatio 16:9 : la boîte prend la largeur (240) et en dérive la hauteur.
        let aspect = Container::new().width(240.0).child(
            AspectRatio::new(16.0 / 9.0).child(
                Container::new()
                    .flex(1.0)
                    .color(theme.primary_container)
                    .gradient(theme.scheme.secondary, [0.0, 1.0])
                    .radius(12.0),
            ),
        );

        // FractionallySizedBox : une barre dont la largeur (fraction du parent)
        // respire au même rythme.
        let bar = Container::new()
            .width(240.0)
            .height(18.0)
            .color(theme.surface)
            .radius(9.0)
            .child(
                FractionallySizedBox::new().width_factor(width_factor).child(
                    Container::new().flex(1.0).color(theme.primary).radius(9.0),
                ),
            );

        let content = Flex::column()
            .gap(22.0)
            .align(Align::Center)
            .child(
                Text::new("Transform · AspectRatio · FractionallySizedBox")
                    .size(20.0)
                    .color(theme.on_surface),
            )
            .child(gallery)
            .child(Text::new("AspectRatio 16:9").size(13.0).color(theme.on_surface))
            .child(aspect)
            .child(Text::new("FractionallySizedBox").size(13.0).color(theme.on_surface))
            .child(bar);

        // Plein-fenêtre, centré, sur le fond du thème.
        let centered = Flex::column()
            .width(width)
            .height(height)
            .justify(Justify::Center)
            .align(Align::Center)
            .child(content);

        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                .child(centered),
        )
    }

    fn title(&self) -> String {
        "frus — transforms".to_string()
    }
}

/// Point d'entrée **bureau** : ouvre la fenêtre et lance la boucle.
#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> anyhow::Result<()> {
    frus_shell::run(Showcase::default())
}

/// Point d'entrée **Android** : appelé par l'activité native.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: frus_shell::AndroidApp) {
    if let Err(err) = frus_shell::run_android(Showcase::default(), android_app) {
        log::error!("frus-transforms (android) s'est arrêté : {err:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `update` avance l'horloge d'un pas fixe par image — pur, sans GPU.
    #[test]
    fn frames_advance_the_clock() {
        let mut app = Showcase::default();
        app.update(Msg::Frame);
        app.update(Msg::Frame);
        assert!((app.time - 2.0 * FRAME_DT).abs() < 1e-6, "temps = {}", app.time);
    }

    /// La souscription bat la mesure en continu (l'animation ne s'arrête jamais).
    #[test]
    fn ticks_continuously() {
        let app = Showcase::default();
        assert!(!app.subscription().is_empty());
    }

    /// Rendu **headless** d'une image : la `view` produit bien un calque **transformé**
    /// (le `Transform` composé du héros), preuve que la vitrine câble la pile de
    /// transformation de bout en bout — sans GPU.
    #[test]
    fn renders_a_transformed_layer() {
        use frus_core::Primitive;
        use frus_widgets::{build_ui, Runtime, Size};
        // Instant non nul : rotation et échelle non identitaires.
        let app = Showcase { time: 0.5 };
        let theme = Theme::dark();
        let view = app.view(&theme, 400.0, 640.0);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(400.0, 640.0), &rt, &theme);
        let transformed = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Layer { transform: Some(_), .. }));
        assert!(transformed, "le Transform composé du héros émet un calque transformé");
    }
}
