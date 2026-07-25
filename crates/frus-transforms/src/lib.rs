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
    Align, Alignment, AspectRatio, BoxFit, Button, ClipOval, ClipPath, ClipRRect, Color, Container,
    Curve, FittedBox, Flex, FractionallySizedBox, InteractiveViewer, Justify, Path, Point, RotatedBox,
    Scroll, Slider, Text, Theme, Transform, Tween, Variant, Widget,
};

/// Un chemin **en étoile** à 5 branches inscrit dans une boîte `size × size`
/// (coordonnées locales) — pour la tuile `ClipPath`.
fn star_path(size: f32) -> Path {
    let c = size / 2.0;
    let (outer, inner) = (c * 0.98, c * 0.42);
    let mut p = Path::new();
    for i in 0..10 {
        let r = if i % 2 == 0 { outer } else { inner };
        let a = -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::PI / 5.0;
        let pt = Point::new(c + r * a.cos(), c + r * a.sin());
        p = if i == 0 { p.move_to(pt) } else { p.line_to(pt) };
    }
    p.close()
}

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

        // Galerie 3 : découpe **en forme** — un carré dégradé à coins **nets** rogné
        // en rectangle arrondi (`ClipRRect`) puis en cercle (`ClipOval`). Le contraste
        // avec les coins nets d'origine rend la découpe visible.
        let sharp = || {
            Container::<Msg>::new()
                .width(96.0)
                .height(96.0)
                .color(theme.primary)
                .gradient(theme.scheme.secondary, [1.0, 1.0])
        };
        let gallery3 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(Box::new(ClipRRect::new(24.0).child(sharp())), "ClipRRect(24)"))
            .child(tile(Box::new(ClipOval::new().child(sharp())), "ClipOval"))
            .child(tile(Box::new(ClipPath::new(star_path(96.0)).child(sharp())), "ClipPath (star)"));

        // Galerie 4 : transformations qui **affectent la mise en page**. `RotatedBox`
        // tourne un texte d'un quart (sa boîte devient haute et étroite) ; `FittedBox`
        // met un grand texte à l'échelle pour **tenir** (Contain) dans un cadre.
        let rotated = RotatedBox::new(3).child(Text::new("ROTATED").size(16.0).color(theme.on_surface));
        let fitted = Container::new().width(120.0).height(80.0).color(theme.surface).radius(8.0).child(
            FittedBox::new(BoxFit::Contain)
                .width(120.0)
                .height(80.0)
                .child(Text::new("Fit").size(48.0).color(theme.primary)),
        );
        let gallery4 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(Box::new(rotated), "RotatedBox(3)"))
            .child(tile(Box::new(fitted), "FittedBox·Contain"));

        // Fenêtre **interactive** : une grille de pastilles sur fond dégradé, que
        // l'utilisateur **déplace** (glisser) et **zoome** (molette, ancrée au curseur).
        // Le contenu déborde la fenêtre à fort zoom — il est découpé au cadre.
        let viewer_content = Container::new()
            .color(theme.surface)
            .gradient(theme.primary_container, [1.0, 1.0])
            .child(
                Flex::column()
                    .justify(Justify::Center)
                    .align(Align::Center)
                    .gap(12.0)
                    .child(
                        Flex::row()
                            .gap(12.0)
                            .child(square(theme.primary))
                            .child(square(theme.scheme.secondary))
                            .child(square(theme.primary)),
                    )
                    .child(
                        Flex::row()
                            .gap(12.0)
                            .child(square(theme.scheme.secondary))
                            .child(square(theme.primary))
                            .child(square(theme.scheme.secondary)),
                    ),
            );
        let viewer = Container::new().radius(12.0).color(theme.surface).child(
            InteractiveViewer::new()
                .width(260.0)
                .height(180.0)
                .min_scale(0.5)
                .max_scale(4.0)
                .child(viewer_content),
        );

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
                    Container::new()
                    .flex(1.0)
                    .color(theme.primary)
                    .radius(9.0),
                ),
            );

        // Colonne complète, centrée, avec un peu de marge.
        let content = Flex::column()
            .width(width)
            .gap(22.0)
            .padding(24.0)
            .align(Align::Center)
            .child(
                Text::new("Transform · Clip · RotatedBox · FittedBox · InteractiveViewer")
                    .size(20.0)
                    .color(theme.on_surface),
            )
            .child(gallery1)
            .child(gallery2)
            .child(Text::new("ClipRRect · ClipOval · ClipPath — clipped to shape").size(13.0).color(theme.on_surface))
            .child(gallery3)
            .child(Text::new("RotatedBox · FittedBox — transforms that change layout").size(13.0).color(theme.on_surface))
            .child(gallery4)
            .child(Text::new("InteractiveViewer — drag to pan, wheel to zoom").size(13.0).color(theme.on_surface))
            .child(viewer)
            .child(Text::new("Interactive — the button below is inside a rotated Transform").size(13.0).color(theme.on_surface))
            .child(interactive)
            .child(Text::new("AspectRatio 16:9").size(13.0).color(theme.on_surface))
            .child(aspect)
            .child(Text::new("FractionallySizedBox").size(13.0).color(theme.on_surface))
            .child(bar);

        // Défilable : le viewport remplit la fenêtre (largeur/hauteur explicites — un
        // `Scroll` par défaut ne fait que 200 px et une largeur automatique) ; le
        // contenu, plus grand, défile.
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                .child(Scroll::new().width(width).height(height).child(content)),
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

    /// La `view` émet des calques **découpés en forme** : la galerie clip produit un
    /// `ClipShape::RRect` **et** un `ClipShape::Oval` — preuve que la vitrine câble la
    /// découpe de bout en bout.
    #[test]
    fn renders_clip_shapes() {
        use frus_core::{BorderRadius, ClipShape, Primitive};
        use frus_widgets::{build_ui, Runtime, Size};
        let app = Showcase::default();
        let theme = Theme::dark();
        let view = app.view(&theme, 500.0, 900.0);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(500.0, 900.0), &rt, &theme);
        // Collecte récursive des formes de découpe (les calques peuvent être imbriqués).
        fn shapes(prims: &[Primitive], out: &mut Vec<ClipShape>) {
            for p in prims {
                if let Primitive::Layer { clip_shape, primitives, .. } = p {
                    out.push(clip_shape.clone());
                    shapes(primitives, out);
                }
            }
        }
        let mut found = Vec::new();
        shapes(ui.scene().primitives(), &mut found);
        assert!(found.contains(&ClipShape::RRect(BorderRadius::uniform(24.0))), "ClipRRect(24) rendu : {found:?}");
        assert!(found.contains(&ClipShape::Oval), "ClipOval rendu : {found:?}");
        assert!(
            found.iter().any(|s| matches!(s, ClipShape::Path(_))),
            "ClipPath (étoile) rendu : {found:?}"
        );
    }

    /// Garde-fou anti-page-blanche : le contenu doit être **réellement dimensionné**
    /// et posé **dans** la fenêtre — au moins un rectangle large, à une position
    /// visible (le viewport `Scroll` remplit bien la fenêtre, il ne s'effondre pas).
    #[test]
    fn content_is_laid_out_within_the_window() {
        use frus_core::Primitive;
        use frus_widgets::{build_ui, Runtime, Size};
        let (w, h) = (1000.0_f32, 800.0_f32);
        let app = Showcase::default();
        let view = app.view(&Theme::dark(), w, h);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(w, h), &rt, &Theme::dark());
        let wide_on_screen = ui.scene().primitives().iter().any(|p| match p {
            Primitive::Rect { rect, .. } => {
                rect.width > 100.0 && rect.x >= -1.0 && rect.x < w && rect.y >= -1.0 && rect.y < h
            }
            _ => false,
        });
        assert!(wide_on_screen, "aucun contenu large visible : viewport effondré ?");
    }
}
