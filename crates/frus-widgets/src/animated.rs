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
            fn anim_transform(&self) -> Option<$crate::runtime::TransformValues> {
                Widget::anim_transform(&self.inner)
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
            // Absent until milestone 477, and silently so: no wrapper set a padding, so
            // nothing missed it. `AnimatedPadding` is the first, and the runtime looked
            // straight through the wrapper and found nothing to drive.
            fn anim_padding(&self) -> Option<frus_core::Insets> {
                Widget::anim_padding(&self.inner)
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

/// **Grows and shrinks its child** rather than letting it jump between sizes — a
/// paint-time scale, so nothing around it moves.
///
/// The reference's `AnimatedScale`. It wraps a [`Transform`](crate::Transform) that was
/// told to animate: the widget declares the scale it is heading for and the runtime
/// drives it there, on the same clock as every other implicit animation here.
///
/// ```
/// use frus_core::Curve;
/// use frus_widgets::{AnimatedScale, Text};
///
/// let pressed = true;
/// let _sunken: AnimatedScale<()> = AnimatedScale::new(
///     if pressed { 0.95 } else { 1.0 },
///     0.12,
///     Curve::ease_out(),
///     Text::new("Tap"),
/// );
/// ```
pub struct AnimatedScale<Msg> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> AnimatedScale<Msg> {
    /// Scales `child` towards `scale`, about its **centre**.
    pub fn new(scale: f32, duration: f32, curve: Curve, child: impl Widget<Msg> + 'static) -> Self {
        Self::about(scale, frus_core::Alignment::CENTER, duration, curve, child)
    }

    /// The same, about a `pivot` of the caller's choosing — the corner or edge that
    /// stays put while the rest grows away from it.
    ///
    /// The pivot itself does not animate, whatever it is set to: it is a choice of
    /// origin, not a quantity, and moving it would slide the child across the screen
    /// with nothing describing the child having changed.
    pub fn about(
        scale: f32,
        pivot: frus_core::Alignment,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Box::new(
                crate::Transform::scale_xy_from(scale, scale, pivot)
                    .animated(duration, curve)
                    .child(child),
            ),
        }
    }
}

impl<Msg> AnimatedScale<Msg> {
    /// A transparent wrapper: the box is the child's. In an impl with no bounds,
    /// because the forwarding macro's own impl has none — a `restyle` declared beside
    /// the constructors could not be reached from it.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(AnimatedScale {
    /// Every one of these is **forwarded**: an animated transform is not an identity,
    /// not a place, not a theme and not a surface. It is its child, moving.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

/// **Turns its child** rather than letting it snap between angles — a paint-time
/// rotation, so nothing around it moves. The reference's `AnimatedRotation`.
///
/// Its unit is **radians**, clockwise, like every other angle in this framework, and not
/// the reference's turns: a turn is a nice number to type and a bad one to mix with the
/// `Path` and `Transform` calls beside it, which are all radians.
///
/// ```
/// use frus_core::Curve;
/// use frus_widgets::{AnimatedRotation, Icon, Icons};
///
/// let open = true;
/// let _chevron: AnimatedRotation<()> = AnimatedRotation::new(
///     if open { std::f32::consts::PI } else { 0.0 },
///     0.2,
///     Curve::ease_in_out(),
///     Icon::new(Icons::EXPAND_MORE),
/// );
/// ```
pub struct AnimatedRotation<Msg> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> AnimatedRotation<Msg> {
    /// Turns `child` towards `radians`, about its **centre**.
    pub fn new(
        radians: f32,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self::about(
            radians,
            frus_core::Alignment::CENTER,
            duration,
            curve,
            child,
        )
    }

    /// The same, about a `pivot` — the point the child turns around.
    pub fn about(
        radians: f32,
        pivot: frus_core::Alignment,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Box::new(
                crate::Transform::rotate_from(radians, pivot)
                    .animated(duration, curve)
                    .child(child),
            ),
        }
    }
}

impl<Msg> AnimatedRotation<Msg> {
    /// A transparent wrapper: the box is the child's. In an impl with no bounds,
    /// because the forwarding macro's own impl has none — a `restyle` declared beside
    /// the constructors could not be reached from it.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(AnimatedRotation {
    /// Every one of these is **forwarded**: an animated transform is not an identity,
    /// not a place, not a theme and not a surface. It is its child, moving.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

/// **Moves its child's inset** rather than letting the space around it jump. The
/// reference's `AnimatedPadding`.
///
/// Unlike the two above, this one is **layout**: the interpolated padding is injected
/// while the tree is being measured, so everything beside and below the child follows it
/// as it moves. That is the point of padding animating at all — a paint-time offset
/// would slide the child over its neighbours instead of making room.
///
/// ```
/// use frus_core::Curve;
/// use frus_widgets::{AnimatedPadding, Text};
///
/// let selected = true;
/// let _row: AnimatedPadding<()> = AnimatedPadding::new(
///     if selected { 24.0 } else { 8.0 },
///     0.2,
///     Curve::ease_in_out(),
///     Text::new("Inbox"),
/// );
/// ```
pub struct AnimatedPadding<Msg> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> AnimatedPadding<Msg> {
    /// Insets `child` by `padding` on all four sides, moving there over `duration`.
    pub fn new(
        padding: f32,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Container::new()
                .animated_padding(padding, duration, curve)
                .child(child),
        }
    }
}

forward_to_container!(AnimatedPadding);

#[cfg(test)]
mod implicit_tests {
    use super::*;
    use crate::interaction::WidgetId;
    use crate::{build_ui, Runtime, Text};
    use frus_core::{Insets, Primitive};

    /// A tree with one animated scale on it, so the runtime has a target to drive.
    fn scaled(to: f32) -> AnimatedScale<()> {
        AnimatedScale::new(to, 0.10, Curve::Linear, Text::new("x"))
    }

    /// **Mounted, mid-flight, at rest** — the three the issue asks each of these to pin.
    ///
    /// The first is the one worth stating: a widget that appears already scaled **adopts**
    /// its target rather than growing into it from nothing. Without that rule every
    /// implicit animation in the framework would play once on the frame it was born, and
    /// a page would breathe when it opened.
    #[test]
    fn a_scale_mounts_settled_moves_and_arrives() {
        let mut rt = Runtime::default();
        assert!(
            !rt.advance_transforms(&scaled(2.0), 1.0),
            "a mount is not a transition"
        );
        let id = WidgetId::ROOT;
        assert_eq!(
            rt.anim_transform(id).map(|t| t.scale_x),
            Some(2.0),
            "and it adopts the target whole"
        );

        // Halfway along a linear curve of 0.10 s: halfway between 2 and 1.
        assert!(rt.advance_transforms(&scaled(1.0), 0.05));
        let mid = rt.anim_transform(id).expect("in flight");
        assert!(
            (mid.scale_x - 1.5).abs() < 1e-3 && (mid.scale_y - 1.5).abs() < 1e-3,
            "halfway between the two: {mid:?}"
        );
        assert_eq!(mid.rotation, 0.0, "and it turned nothing on the way");

        rt.advance_transforms(&scaled(1.0), 1.0);
        assert_eq!(rt.anim_transform(id).map(|t| t.scale_x), Some(1.0));
        assert!(
            !rt.advance_transforms(&scaled(1.0), 0.05),
            "and stops asking for frames once it is there"
        );
    }

    /// The same three for a rotation, and that the two do not leak into one another: a
    /// widget that only turns keeps its scale at **one**, not at nought — which is the
    /// difference between a turning child and a child that vanished.
    #[test]
    fn a_rotation_mounts_settled_moves_and_arrives() {
        let turned = |to: f32| -> AnimatedRotation<()> {
            AnimatedRotation::new(to, 0.10, Curve::Linear, Text::new("x"))
        };
        let mut rt = Runtime::default();
        assert!(!rt.advance_transforms(&turned(0.0), 1.0));
        let id = WidgetId::ROOT;
        let mounted = rt.anim_transform(id).expect("mounted");
        assert_eq!((mounted.scale_x, mounted.scale_y), (1.0, 1.0), "identity");

        assert!(rt.advance_transforms(&turned(1.0), 0.05));
        let mid = rt.anim_transform(id).expect("in flight");
        assert!((mid.rotation - 0.5).abs() < 1e-3, "halfway: {mid:?}");
        assert_eq!(
            (mid.scale_x, mid.scale_y),
            (1.0, 1.0),
            "and the scale stayed the identity, not nought"
        );

        rt.advance_transforms(&turned(1.0), 1.0);
        assert_eq!(rt.anim_transform(id).map(|t| t.rotation), Some(1.0));
    }

    /// And the padding, which is the one that is **layout** rather than paint: the
    /// interpolated value is injected while the tree is measured.
    #[test]
    fn a_padding_mounts_settled_moves_and_arrives() {
        let padded = |to: f32| -> AnimatedPadding<()> {
            AnimatedPadding::new(to, 0.10, Curve::Linear, Text::new("x"))
        };
        let mut rt = Runtime::default();
        assert!(!rt.advance_paddings(&padded(0.0), 1.0));
        let id = WidgetId::ROOT;
        assert_eq!(rt.anim_padding(id), Some(Insets::uniform(0.0)));

        assert!(rt.advance_paddings(&padded(20.0), 0.05));
        let mid = rt.anim_padding(id).expect("in flight");
        assert!((mid.left - 10.0).abs() < 1e-3, "halfway: {mid:?}");

        rt.advance_paddings(&padded(20.0), 1.0);
        assert_eq!(rt.anim_padding(id), Some(Insets::uniform(20.0)));
    }

    /// **And the paint reads it**, which is a separate question from whether the runtime
    /// computes it. A tween the walk never asks for is a number that moves correctly and
    /// changes nothing on the screen, and this framework has shipped that bug before.
    #[test]
    fn the_paint_uses_the_tweened_transform_and_not_the_target() {
        let matrix_of = |rt: &Runtime, widget: &AnimatedScale<()>| {
            let ui = build_ui(widget, Size::new(100.0, 100.0), rt, &Theme::default());
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Layer {
                        transform: Some(t), ..
                    } => Some(t.affine),
                    _ => None,
                })
                .map(|m| m.m[0])
        };
        let mut rt = Runtime::default();
        rt.advance_transforms(&scaled(3.0), 1.0);
        // Now heading back to 1, half way along.
        rt.advance_transforms(&scaled(1.0), 0.05);
        let painted = matrix_of(&rt, &scaled(1.0)).expect("a transformed layer");
        assert!(
            (painted - 2.0).abs() < 1e-2,
            "the paint took the tweened scale (2), not the target (1): {painted}"
        );

        // A runtime that has never heard of it draws the target, which is what an
        // isolated frame — a test, a golden — has to show.
        let fresh = Runtime::default();
        let painted = matrix_of(&fresh, &scaled(3.0)).expect("a transformed layer");
        assert!((painted - 3.0).abs() < 1e-2, "the target: {painted}");
    }
}
