//! **Named** implicit-animation widgets: ergonomic sugar over [`Container`].
//!
//! Each one **wraps a configured [`Container`]** and delegates *everything* to it
//! (a transparent wrapper, like [`crate::Keyed`]): the inner `Container` is the
//! animated node, and its child stays a **separate** node — so the per-node
//! animated values never collide. The identities (`child_id`), and therefore the
//! animations, line up exactly with the paint walk.

use frus_core::{BorderRadius, Color, Curve, Rect, Scene, Size};
use frus_layout::Style;

use crate::container::Container;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Implements `Widget` for a `{ inner: Container<Msg> }` wrapper by delegating
/// exactly the methods `Container` overrides (the rest are trait defaults,
/// identical to `Container`'s). `debug_name` is **not** delegated: that way the
/// inspector shows the named widget's own name.
macro_rules! forward_to_container {
    ($ty:ident) => {
        // `Container` has **inherent** methods (the `on_click`, `repaint_boundary`…
        // builders) sharing their names with the trait's, so the trait is called in
        // fully qualified syntax (`Widget::…(&self.inner)`) to remove the
        // ambiguity.
        impl<Msg: Clone + 'static> Widget<Msg> for $ty<Msg> {
            fn style(&self) -> Style {
                Widget::style(&self.inner)
            }
            fn style_themed(&self, theme: &Theme) -> Style {
                Widget::style_themed(&self.inner, theme)
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
            fn clip_shape(&self) -> Option<frus_core::ClipShape> {
                Widget::clip_shape(&self.inner)
            }
            fn clip_path(&self) -> Option<&frus_core::Path> {
                Widget::clip_path(&self.inner)
            }
            fn barrier(&self) -> Option<$crate::barrier::ModalBarrier> {
                Widget::barrier(&self.inner)
            }
            fn interactive(&self) -> Option<(f32, f32)> {
                Widget::interactive(&self.inner)
            }
            fn fitted(&self) -> Option<frus_core::BoxFit> {
                Widget::fitted(&self.inner)
            }
            fn rotated_quarter_turns(&self) -> Option<i32> {
                Widget::rotated_quarter_turns(&self.inner)
            }
        }
    };
}

/// Applies a fixed **group opacity** `[0,1]` to its child, as one block. See
/// [`Container::opacity`].
pub struct Opacity<Msg> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> Opacity<Msg> {
    /// Wraps `child` in a group opacity of `opacity`.
    pub fn new(opacity: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Container::new().opacity(opacity).child(child),
        }
    }
}

forward_to_container!(Opacity);

/// **Fades** its child toward `opacity` on every change. See
/// [`Container::animated_opacity`].
pub struct AnimatedOpacity<Msg> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> AnimatedOpacity<Msg> {
    /// Wraps `child` in an animated group opacity (`duration`, `curve`).
    pub fn new(
        opacity: f32,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Container::new()
                .animated_opacity(opacity, duration, curve)
                .child(child),
        }
    }
}

forward_to_container!(AnimatedOpacity);

/// A box whose properties **animate** on every change: color, size, radius and
/// opacity — all with the **same** `(duration, curve)`. Builds a [`Container`]
/// under the hood.
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
    /// A new animated box: `duration` (seconds) and `curve` shared by all of its
    /// animated properties.
    pub fn new(duration: f32, curve: Curve) -> Self {
        Self {
            inner: Container::new(),
            duration,
            curve,
        }
    }

    /// A background whose color animates toward `color`.
    pub fn color(mut self, color: Color) -> Self {
        self.inner = self
            .inner
            .animated_color(color, self.duration, self.curve.clone());
        self
    }

    /// Animated `width×height` size (interpolated at layout time).
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.inner = self
            .inner
            .animated_size(width, height, self.duration, self.curve.clone());
        self
    }

    /// Animated corner radius (uniform via `f32`, or per corner via [`BorderRadius`]).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.inner = self
            .inner
            .animated_radius(radius, self.duration, self.curve.clone());
        self
    }

    /// Animated group opacity `[0,1]`.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.inner = self
            .inner
            .animated_opacity(opacity, self.duration, self.curve.clone());
        self
    }

    /// Inner padding (static).
    pub fn padding(mut self, padding: f32) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    /// The box's child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.inner = self.inner.child(child);
        self
    }
}

forward_to_container!(AnimatedContainer);

#[cfg(test)]
mod tests {
    use super::*;

    /// `AnimatedContainer` does declare the animated targets of its properties, with
    /// the shared duration and curve.
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
        assert_eq!(
            Widget::<()>::anim_radius(&w),
            Some(BorderRadius::from(12.0))
        );
        assert_eq!(Widget::<()>::anim_target(&w), Some(0.5)); // animated opacity
        assert_eq!(Widget::<()>::opacity_group(&w), Some(0.5));
        assert_eq!(Widget::<()>::anim_duration(&w), 0.25);
        assert_eq!(Widget::<()>::anim_curve(&w), Curve::ease_out());
    }

    /// `Opacity` is a fixed opacity group (no animated value) wrapping its child
    /// (a separate node).
    #[test]
    fn opacity_wraps_child_as_a_group() {
        let w: Opacity<()> = Opacity::new(0.4, crate::Container::new().width(10.0).height(10.0));
        assert_eq!(Widget::<()>::opacity_group(&w), Some(0.4));
        assert_eq!(Widget::<()>::anim_target(&w), None);
        assert_eq!(
            Widget::<()>::children(&w).len(),
            1,
            "the child is a separate node"
        );
    }

    /// `AnimatedOpacity` declares an animated opacity (the runtime tweens it).
    #[test]
    fn animated_opacity_declares_a_group_target() {
        let w: AnimatedOpacity<()> =
            AnimatedOpacity::new(0.0, 0.2, Curve::ease_in(), crate::Container::new());
        assert_eq!(Widget::<()>::opacity_group(&w), Some(0.0));
        assert_eq!(Widget::<()>::anim_target(&w), Some(0.0));
        assert_eq!(Widget::<()>::anim_duration(&w), 0.2);
        // Its own name for the inspector (not delegated to the Container).
        assert_eq!(Widget::<()>::debug_name(&w), "AnimatedOpacity");
    }
}
