//! Widgets **nommés** d'animation implicite, sucre ergonomique au-dessus de
//! [`Container`] (façon `Opacity` / `AnimatedOpacity` / `AnimatedContainer` de
//! Flutter).
//!
//! Chacun **enveloppe un [`Container`]** configuré et lui délègue *tout*
//! (wrapper transparent, comme [`crate::Keyed`]) : le `Container` interne est le
//! nœud animé, son enfant reste un nœud **séparé** — aucune collision de la
//! valeur animée par nœud. Les identités (`child_id`) et donc les animations
//! s'alignent exactement sur la marche de peinture.

use frus_core::{BorderRadius, Color, Curve, Rect, Scene, Size};
use frus_layout::Style;

use crate::container::Container;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Implémente `Widget` pour un wrapper `{ inner: Container<Msg> }` en déléguant
/// exactement les méthodes que `Container` surcharge (le reste = défauts du
/// trait, identiques à ceux de `Container`). `debug_name` n'est **pas** délégué :
/// l'inspecteur affiche ainsi le nom du widget nommé.
macro_rules! forward_to_container {
    ($ty:ident) => {
        // `Container` a des méthodes **inhérentes** (builders `on_click`,
        // `repaint_boundary`…) de même nom que le trait ; on appelle donc le trait
        // en syntaxe pleinement qualifiée (`Widget::…(&self.inner)`) pour lever
        // l'ambiguïté.
        impl<Msg: Clone + 'static> Widget<Msg> for $ty<Msg> {
            fn style(&self) -> Style {
                Widget::style(&self.inner)
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                Widget::children(&self.inner)
            }
            fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
                Widget::paint(&self.inner, bounds, status, theme, scene)
            }
            fn on_click(&self) -> Option<Msg> {
                Widget::on_click(&self.inner)
            }
            fn on_long_press(&self) -> Option<Msg> {
                Widget::on_long_press(&self.inner)
            }
            fn repaint_boundary(&self) -> bool {
                Widget::repaint_boundary(&self.inner)
            }
            fn opacity_group(&self) -> Option<f32> {
                Widget::opacity_group(&self.inner)
            }
            fn anim_target(&self) -> Option<f32> {
                Widget::anim_target(&self.inner)
            }
            fn anim_color(&self) -> Option<Color> {
                Widget::anim_color(&self.inner)
            }
            fn anim_size(&self) -> Option<Size> {
                Widget::anim_size(&self.inner)
            }
            fn anim_radius(&self) -> Option<BorderRadius> {
                Widget::anim_radius(&self.inner)
            }
            fn anim_duration(&self) -> f32 {
                Widget::anim_duration(&self.inner)
            }
            fn anim_curve(&self) -> Curve {
                Widget::anim_curve(&self.inner)
            }
            fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
                Widget::alignment_geometry(&self.inner)
            }
            fn transform_translate(&self) -> Option<(f32, f32)> {
                Widget::transform_translate(&self.inner)
            }
            fn transform_scale(&self) -> Option<(f32, f32, frus_core::Alignment)> {
                Widget::transform_scale(&self.inner)
            }
            fn transform_rotate(&self) -> Option<(f32, frus_core::Alignment)> {
                Widget::transform_rotate(&self.inner)
            }
        }
    };
}

/// Applique une **opacité de groupe** fixe `[0,1]` à son enfant, d'un bloc (façon
/// `Opacity` de Flutter). Voir [`Container::opacity`].
pub struct Opacity<Msg> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> Opacity<Msg> {
    /// Enveloppe `child` d'une opacité de groupe `opacity`.
    pub fn new(opacity: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self { inner: Container::new().opacity(opacity).child(child) }
    }
}

forward_to_container!(Opacity);

/// Fait **fondre** son enfant vers `opacity` à chaque changement (façon
/// `AnimatedOpacity` de Flutter). Voir [`Container::animated_opacity`].
pub struct AnimatedOpacity<Msg> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> AnimatedOpacity<Msg> {
    /// Enveloppe `child` d'une opacité de groupe animée (`duration`, `curve`).
    pub fn new(
        opacity: f32,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self { inner: Container::new().animated_opacity(opacity, duration, curve).child(child) }
    }
}

forward_to_container!(AnimatedOpacity);

/// Boîte dont les propriétés **s'animent** à chaque changement (façon
/// `AnimatedContainer` de Flutter) : couleur, taille, rayon, opacité — toutes
/// avec la **même** `(durée, courbe)`. Construit un [`Container`] sous le capot.
///
/// ```ignore
/// AnimatedContainer::new(0.3, Curve::ease_in_out())
///     .color(theme.primary)
///     .size(200.0, 100.0)
///     .radius(12.0)
///     .child(Text::new("hi"))
/// ```
pub struct AnimatedContainer<Msg> {
    inner: Container<Msg>,
    duration: f32,
    curve: Curve,
}

impl<Msg: Clone + 'static> AnimatedContainer<Msg> {
    /// Nouvelle boîte animée : `duration` (secondes) et `curve` partagées par
    /// toutes ses propriétés animées.
    pub fn new(duration: f32, curve: Curve) -> Self {
        Self { inner: Container::new(), duration, curve }
    }

    /// Fond dont la couleur s'anime vers `color`.
    pub fn color(mut self, color: Color) -> Self {
        self.inner = self.inner.animated_color(color, self.duration, self.curve.clone());
        self
    }

    /// Taille animée `width×height` (interpolée au layout).
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.inner = self.inner.animated_size(width, height, self.duration, self.curve.clone());
        self
    }

    /// Rayon de coin animé (uniforme via `f32` ou par coin via [`BorderRadius`]).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.inner = self.inner.animated_radius(radius, self.duration, self.curve.clone());
        self
    }

    /// Opacité de groupe animée `[0,1]`.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.inner = self.inner.animated_opacity(opacity, self.duration, self.curve.clone());
        self
    }

    /// Marge intérieure (statique).
    pub fn padding(mut self, padding: f32) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    /// Enfant de la boîte.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.inner = self.inner.child(child);
        self
    }
}

forward_to_container!(AnimatedContainer);

#[cfg(test)]
mod tests {
    use super::*;

    /// `AnimatedContainer` déclare bien les cibles animées de ses propriétés, avec
    /// la durée et la courbe partagées.
    #[test]
    fn animated_container_declares_all_targets() {
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let w: AnimatedContainer<()> = AnimatedContainer::new(0.25, Curve::ease_out())
            .color(blue)
            .size(200.0, 100.0)
            .radius(12.0)
            .opacity(0.5);
        assert_eq!(Widget::<()>::anim_color(&w), Some(blue));
        assert_eq!(Widget::<()>::anim_size(&w), Some(Size::new(200.0, 100.0)));
        assert_eq!(Widget::<()>::anim_radius(&w), Some(BorderRadius::from(12.0)));
        assert_eq!(Widget::<()>::anim_target(&w), Some(0.5)); // opacité animée
        assert_eq!(Widget::<()>::opacity_group(&w), Some(0.5));
        assert_eq!(Widget::<()>::anim_duration(&w), 0.25);
        assert_eq!(Widget::<()>::anim_curve(&w), Curve::ease_out());
    }

    /// `Opacity` est un groupe d'opacité fixe (pas de valeur animée) enveloppant
    /// son enfant (un nœud séparé).
    #[test]
    fn opacity_wraps_child_as_a_group() {
        let w: Opacity<()> = Opacity::new(0.4, crate::Container::new().width(10.0).height(10.0));
        assert_eq!(Widget::<()>::opacity_group(&w), Some(0.4));
        assert_eq!(Widget::<()>::anim_target(&w), None);
        assert_eq!(Widget::<()>::children(&w).len(), 1, "l'enfant est un nœud séparé");
    }

    /// `AnimatedOpacity` déclare une opacité animée (le runtime la tween).
    #[test]
    fn animated_opacity_declares_a_group_target() {
        let w: AnimatedOpacity<()> =
            AnimatedOpacity::new(0.0, 0.2, Curve::ease_in(), crate::Container::new());
        assert_eq!(Widget::<()>::opacity_group(&w), Some(0.0));
        assert_eq!(Widget::<()>::anim_target(&w), Some(0.0));
        assert_eq!(Widget::<()>::anim_duration(&w), 0.2);
        // Nom propre pour l'inspecteur (pas délégué au Container).
        assert_eq!(Widget::<()>::debug_name(&w), "AnimatedOpacity");
    }
}
