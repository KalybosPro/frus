//! `frus-transforms` — une vitrine **animée et interactive** de l'arsenal de
//! disposition/peinture : la palette [`Transform`] (translation, échelle non
//! uniforme, rotation, pivot décalé, compositions), [`AspectRatio`] et
//! [`FractionallySizedBox`], pilotées par un [`Tween`] au fil du temps **et** par
//! l'utilisateur (curseur, boutons).
//!
//! Fait notable : un bouton **cliquable placé dans un `Transform` tourné** réagit
//! quand même — preuve *visible* que le hit-test traverse les transformations (via la
//! matrice inverse).
//!
//! Modèle Elm au complet : un petit état, un `update` pur, une souscription qui bat la
//! mesure (~60 fps, en pause à l'arrêt), et une `view` pure.
//!
//! Lancer sur bureau : `cargo run -p frus-transforms`.

use std::time::Duration;

use frus_shell::{Application, Command, Subscription};
use frus_widgets::{
    Align, Alignment, AspectRatio, Button, Color, Container, Curve, Flex, FractionallySizedBox,
    Justify, Scroll, Slider, Text, Theme, Transform, Tween, Variant, Widget,
};

/// Pas de temps fixe par image (~60 fps) : garde `update` **déterministe** et
/// testable, tout en suivant le rythme de la souscription.
const FRAME_DT: f32 = 1.0 / 60.0;

/// Durée d'un cycle d'aller-retour (échelle, largeur fractionnaire), en secondes.
const CYCLE: f32 = 2.4;

/// L'état : le temps écoulé, l'animation en cours ou non, la position du curseur
/// d'échelle (`0..1`) et le nombre de clics sur le bouton transformé.
struct Showcase {
    time: f32,
    running: bool,
    scale_knob: f32,
    taps: u32,
}

impl Default for Showcase {
    fn default() -> Self {
        // `scale_knob = 1/3` correspond à une échelle manuelle de 1.0 (voir `view`).
        Self { time: 0.0, running: true, scale_knob: 1.0 / 3.0, taps: 0 }
    }
}

/// Les messages : horloge, lecture/pause, curseur d'échelle, clic transformé.
#[derive(Clone)]
enum Msg {
    /// Une image est passée : avance l'horloge.
    Frame,
    /// Bascule lecture / pause de l'animation automatique.
    ToggleRunning,
    /// Nouvelle position du curseur d'échelle (`0..1`).
    SetKnob(f32),
    /// Clic sur le bouton placé dans un `Transform` tourné.
    Tap,
}

/// Échelle manuelle (`0.5 → 2.0`) dérivée de la position du curseur (`0..1`).
fn knob_to_scale(knob: f32) -> f32 {
    0.5 + knob * 1.5
}

impl Application for Showcase {
    type Message = Msg;

    /// `update` est **pur** : il fait évoluer l'état, sans effet ni GPU.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Frame => {
                if self.running {
                    self.time += FRAME_DT;
                }
            }
            Msg::ToggleRunning => self.running = !self.running,
            Msg::SetKnob(v) => self.scale_knob = v.clamp(0.0, 1.0),
            Msg::Tap => self.taps += 1,
        }
        Command::none()
    }

    /// Bat la mesure ~60 fps **tant que l'animation tourne** ; en pause, aucune
    /// souscription (les interactions, elles, restent événementielles).
    fn subscription(&self) -> Subscription<Msg> {
        if self.running {
            Subscription::every(Duration::from_millis(16), |_| Msg::Frame)
        } else {
            Subscription::none()
        }
    }

    /// `view` est une fonction **pure** de l'état : elle recalcule les valeurs
    /// animées puis reconstruit la scène.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        // Phase d'aller-retour `0 → 1 → 0` sur un cycle, adoucie.
        let phase = (self.time % CYCLE) / CYCLE;
        let ping = 1.0 - (2.0 * phase - 1.0).abs();
        let eased = Curve::ease_in_out().transform(ping);

        // Valeurs animées (Tween / sinusoïdes).
        let angle = self.time * 0.9; // rotation continue (rad)
        let scale = Tween::new(1.0, 1.4).eval(eased); // pulsation
        let bob = (self.time * 2.2).sin() * 22.0; // va-et-vient vertical
        let drift = (self.time * 1.6).sin() * 26.0; // dérive horizontale
        let squash = (self.time * 2.6).sin() * 0.35; // écrasement/étirement
        let width_factor = Tween::new(0.25, 1.0).eval(eased);
        let manual_scale = knob_to_scale(self.scale_knob);

        // Petit carré arrondi de couleur.
        let square = |color: Color| {
            Container::<Msg>::new().width(64.0).height(64.0).color(color).radius(14.0)
        };
        // Un carré dégradé (pour le héros composé).
        let gradient_square = || {
            Container::<Msg>::new()
                .width(64.0)
                .height(64.0)
                .color(theme.primary)
                .gradient(theme.scheme.secondary, [1.0, 1.0])
                .radius(14.0)
        };
        // Une tuile : contenu centré dans une scène fixe (marge pour déborder), légendé.
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

        // Galerie 1 : translation, échelle non uniforme, composition rotation+échelle.
        let gallery1 = Flex::row()
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
                Box::new(Transform::rotate(angle).and_scale(scale).child(gradient_square())),
                "rotate + scale",
            ));

        // Galerie 2 : rotation autour d'un **pivot décalé**, et translation+rotation.
        let gallery2 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(
                Box::new(
                    Transform::rotate_from(angle, Alignment::TOP_LEFT)
                        .child(square(theme.scheme.secondary)),
                ),
                "rotate @ corner",
            ))
            .child(tile(
                Box::new(
                    Transform::rotate(angle * 0.6)
                        .and_translate(drift, 0.0)
                        .child(square(theme.primary)),
                ),
                "translate + rotate",
            ));

        // Interactif : un bouton **cliquable dans un Transform tourné** (le hit-test
        // traverse la transformation), un curseur qui pilote une échelle en direct, et
        // un bouton lecture/pause.
        let tap_stage = Flex::column()
            .width(240.0)
            .height(96.0)
            .justify(Justify::Center)
            .align(Align::Center)
            .child(
                Transform::rotate(0.35)
                    .and_scale(1.15)
                    .child(Button::new("Tap me (rotated)").on_press(Msg::Tap)),
            );
        let tap_col = Flex::column()
            .gap(8.0)
            .align(Align::Center)
            .child(tap_stage)
            .child(Text::new(format!("taps: {}", self.taps)).size(14.0).color(theme.on_surface));

        let knob_stage = Flex::column()
            .width(200.0)
            .height(96.0)
            .justify(Justify::Center)
            .align(Align::Center)
            .child(Transform::scale(manual_scale).child(square(theme.primary)));
        let knob_col = Flex::column()
            .gap(8.0)
            .align(Align::Center)
            .child(knob_stage)
            .child(Slider::new(self.scale_knob).width(200.0).on_change(|v| Msg::SetKnob(v)))
            .child(
                Text::new(format!("scale: {manual_scale:.2}"))
                    .size(12.0)
                    .color(theme.on_surface),
            );

        let interactive = Flex::column()
            .gap(14.0)
            .align(Align::Center)
            .child(Flex::row().gap(28.0).align(Align::Center).child(tap_col).child(knob_col))
            .child(
                Button::new(if self.running { "pause" } else { "play" })
                    .variant(Variant::Secondary)
                    .on_press(Msg::ToggleRunning),
            );

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

        // FractionallySizedBox : une barre dont la largeur (fraction du parent) respire.
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

        // Colonne complète, centrée, avec un peu de marge.
        let content = Flex::column()
            .width(width)
            .gap(22.0)
            .padding(24.0)
            .align(Align::Center)
            .child(
                Text::new("Transform · AspectRatio · FractionallySizedBox")
                    .size(20.0)
                    .color(theme.on_surface),
            )
            .child(gallery1)
            .child(gallery2)
            .child(Text::new("Interactive — the button below is inside a rotated Transform").size(13.0).color(theme.on_surface))
            .child(interactive)
            .child(Text::new("AspectRatio 16:9").size(13.0).color(theme.on_surface))
            .child(aspect)
            .child(Text::new("FractionallySizedBox").size(13.0).color(theme.on_surface))
            .child(bar);

        // Défilable : la vitrine reste utilisable même sur une petite fenêtre.
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                .child(Scroll::new().child(content)),
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

    /// En **pause**, l'horloge ne bouge plus et la souscription se tait.
    #[test]
    fn pause_stops_the_clock_and_the_subscription() {
        let mut app = Showcase::default();
        assert!(!app.subscription().is_empty(), "en marche : souscription active");
        app.update(Msg::ToggleRunning);
        app.update(Msg::Frame);
        assert_eq!(app.time, 0.0, "en pause : l'horloge est figée");
        assert!(app.subscription().is_empty(), "en pause : plus de souscription");
    }

    /// Le curseur pilote l'échelle manuelle (`0..1 → 0.5..2.0`).
    #[test]
    fn knob_drives_the_manual_scale() {
        let mut app = Showcase::default();
        app.update(Msg::SetKnob(1.0));
        assert!((knob_to_scale(app.scale_knob) - 2.0).abs() < 1e-6);
        app.update(Msg::SetKnob(0.0));
        assert!((knob_to_scale(app.scale_knob) - 0.5).abs() < 1e-6);
    }

    /// Chaque clic sur le bouton transformé incrémente le compteur.
    #[test]
    fn taps_increment() {
        let mut app = Showcase::default();
        app.update(Msg::Tap);
        app.update(Msg::Tap);
        assert_eq!(app.taps, 2);
    }

    /// Rendu **headless** d'une image : la `view` produit un calque **transformé**
    /// (les `Transform` de la scène), preuve que la vitrine câble la pile de bout en
    /// bout — sans GPU.
    #[test]
    fn renders_a_transformed_layer() {
        use frus_core::Primitive;
        use frus_widgets::{build_ui, Runtime, Size};
        let app = Showcase { time: 0.5, ..Default::default() };
        let theme = Theme::dark();
        let view = app.view(&theme, 400.0, 640.0);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(400.0, 640.0), &rt, &theme);
        let transformed = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Layer { transform: Some(_), .. }));
        assert!(transformed, "un Transform de la scène émet un calque transformé");
    }
}
