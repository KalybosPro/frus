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
            fn opaque(&self) -> bool {
                Widget::opaque(&self.inner)
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
            // And this one is why that comment is here: `AnimatedAlign` is the first
            // wrapper to set an offset, and a hook the macro does not forward is a value
            // the runtime never finds.
            fn anim_offset(&self) -> Option<(f32, f32)> {
                Widget::anim_offset(&self.inner)
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

/// **Slides its child** to each new anchor instead of letting it jump across the box. The
/// reference's `AnimatedAlign`.
///
/// The anchor is where the child sits in the free space around it, so this is the widget
/// for a thing that moves *within* something — a thumb crossing a track, a badge changing
/// corner, a label that settles against the other edge when a panel opens.
///
/// ```
/// use frus_core::{Alignment, Curve};
/// use frus_widgets::{AnimatedAlign, Container};
///
/// let on = true;
/// let _thumb: AnimatedAlign<()> = AnimatedAlign::new(
///     if on { Alignment::CENTER_RIGHT } else { Alignment::CENTER_LEFT },
///     0.15,
///     Curve::ease_out(),
///     Container::new().width(20.0).height(20.0),
/// );
/// ```
///
/// The **kind** of anchor is kept the whole way: a directional one stays directional, so
/// in a right-to-left script the whole movement mirrors rather than the two ends
/// mirroring separately and the child crossing the box the wrong way.
///
/// Unlike a scale's pivot, an anchor **is** the quantity here — it says where the child
/// is, not where the maths starts from — which is why this one animates and that one does
/// not.
pub struct AnimatedAlign<Msg> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> AnimatedAlign<Msg> {
    /// Anchors `child` at `alignment`, moving there over `duration`.
    pub fn new(
        alignment: impl Into<frus_core::AlignmentGeometry>,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Container::new()
                .animated_alignment(alignment, duration, curve)
                .child(child),
        }
    }
}

forward_to_container!(AnimatedAlign);

/// **Slides its child by a fraction of its own size**, at paint time, moving there rather
/// than jumping. The reference's `AnimatedSlide`.
///
/// `(1.0, 0.0)` is one whole width to the right — exactly clear of where the child was,
/// whatever that width turns out to be. That is the difference between this and an
/// animated `Transform::translate`: the number the caller writes is a multiple of a size
/// the **layout** decides, so a panel can be parked off its own edge without anybody
/// having to know how wide it ended up.
///
/// ```
/// use frus_core::Curve;
/// use frus_widgets::{AnimatedSlide, Text};
///
/// let open = true;
/// let _panel: AnimatedSlide<()> = AnimatedSlide::new(
///     if open { 0.0 } else { -1.0 },
///     0.0,
///     0.25,
///     Curve::ease_out(),
///     Text::new("Filters"),
/// );
/// ```
///
/// Layout is untouched: the box stays where it was put and the neighbours do not move, so
/// a child sliding out leaves its space behind rather than dragging the page after it.
pub struct AnimatedSlide<Msg> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> AnimatedSlide<Msg> {
    /// Slides `child` towards `fx` widths across and `fy` heights down.
    pub fn new(
        fx: f32,
        fy: f32,
        duration: f32,
        curve: Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self {
            inner: Box::new(
                crate::FractionalTranslation::new(fx, fy)
                    .animated(duration, curve)
                    .child(child),
            ),
        }
    }
}

impl<Msg> AnimatedSlide<Msg> {
    /// A transparent wrapper: the box is the child's. In an impl with no bounds, because
    /// the forwarding macro's own impl has none — a `restyle` declared beside the
    /// constructors could not be reached from it.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(AnimatedSlide {
    /// Every one of these is **forwarded**: a paint-time slide is not an identity, not a
    /// place, not a theme and not a surface. It is its child, moving.
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

/// **Moves a stack layer between two sets of pins** instead of letting it appear at the
/// new ones. The reference's `AnimatedPositioned`.
///
/// A panel that slides in from an edge, a sheet that grows to fill the page, a card that
/// travels to a corner: each is a layer of a [`crate::Stack`] whose distances from the
/// stack's own edges change, and each of them jumped before this.
///
/// ```
/// use frus_core::Curve;
/// use frus_widgets::{AnimatedPositioned, Container, Stack};
///
/// let open = true;
/// let _sheet: Stack<()> = Stack::new().layer(
///     AnimatedPositioned::new(0.25, Curve::ease_out(), Container::new())
///         .left(0.0)
///         .right(0.0)
///         .bottom(0.0)
///         .height(if open { 320.0 } else { 0.0 }),
/// );
/// ```
///
/// **A pin that is unset does not animate.** `None` on an edge is not nought there: it is
/// a layer not pinned on that side, sized by itself and placed by the stack's alignment,
/// which is a different arrangement rather than a different number. So an edge arriving
/// takes hold at once, an edge leaving lets go at once, and only an edge set at both ends
/// travels. Where a movement is wanted, both ends must say where they are.
///
/// The six pins share **one timeline**, as a scale and a turn do: a layer changing two of
/// its edges arrives on both at the same moment rather than on two clocks that happen to
/// agree.
///
/// Unlike [`crate::Positioned`], which is a transparent wrapper, this is a **node of its
/// own** with the child beneath it. An animated value belongs to a node, and a wrapper
/// that fused with its child would put the layer's timeline and the child's on the same
/// one — the rule the whole of this module is built on.
pub struct AnimatedPositioned<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    spec: crate::positioned::Positioning,
    duration: f32,
    curve: Curve,
}

impl<Msg> AnimatedPositioned<Msg> {
    /// A layer holding `child`, pinned nowhere yet, moving to each new set of pins over
    /// `duration` seconds on `curve`.
    pub fn new(duration: f32, curve: Curve, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            children: vec![Box::new(child)],
            spec: crate::positioned::Positioning::default(),
            duration,
            curve,
        }
    }

    /// Distance from the stack's left edge.
    pub fn left(mut self, px: f32) -> Self {
        self.spec.left = Some(px);
        self
    }

    /// Distance from the stack's top edge.
    pub fn top(mut self, px: f32) -> Self {
        self.spec.top = Some(px);
        self
    }

    /// Distance from the stack's right edge.
    pub fn right(mut self, px: f32) -> Self {
        self.spec.right = Some(px);
        self
    }

    /// Distance from the stack's bottom edge.
    pub fn bottom(mut self, px: f32) -> Self {
        self.spec.bottom = Some(px);
        self
    }

    /// An explicit width, used when only one horizontal edge is pinned.
    pub fn width(mut self, px: f32) -> Self {
        self.spec.width = Some(px);
        self
    }

    /// An explicit height, used when only one vertical edge is pinned.
    pub fn height(mut self, px: f32) -> Self {
        self.spec.height = Some(px);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for AnimatedPositioned<Msg> {
    fn style(&self) -> Style {
        // Nothing of its own: the stack forces a pinned layer into what its edges say,
        // and an axis nobody pinned is left to the child underneath.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure layout widget: no decoration of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// Where it is **going**. The stack reads the runtime's interpolated pins instead
    /// wherever there are any, which is what makes the difference between the two.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        Some(self.spec)
    }

    fn anim_pins(&self) -> Option<crate::positioned::Positioning> {
        Some(self.spec)
    }

    fn anim_duration(&self) -> f32 {
        self.duration
    }

    fn anim_curve(&self) -> Curve {
        self.curve.clone()
    }

    fn debug_name(&self) -> &'static str {
        "AnimatedPositioned"
    }
}

/// **Grows and shrinks its share of the parent** rather than taking the new share at
/// once. The reference's `AnimatedFractionallySizedBox`, and a name over
/// [`crate::FractionallySizedBox::animated`].
///
/// The quantity is a fraction and not a length, which is the reason to reach for this
/// rather than an animated size: a bar filling half its row, a panel taking two thirds of
/// a page, a sheet at a third of the height — none of them knows what those come to, and
/// a resize of the window during the movement is answered by the layout rather than by
/// a number that was right when it was written.
///
/// ```
/// use frus_core::Curve;
/// use frus_widgets::{AnimatedFractionallySizedBox, Container};
///
/// let full = true;
/// let _bar: AnimatedFractionallySizedBox<()> =
///     AnimatedFractionallySizedBox::new(0.3, Curve::ease_out(), Container::new())
///         .width_factor(if full { 1.0 } else { 0.25 });
/// ```
///
/// An axis whose factor is **unset** follows its content, and does not travel to or from
/// a factor: that is a change of arrangement, not of value.
pub struct AnimatedFractionallySizedBox<Msg> {
    inner: crate::fractional::FractionallySizedBox<Msg>,
}

impl<Msg: Clone + 'static> AnimatedFractionallySizedBox<Msg> {
    /// A fractional box holding `child`, with no factor yet, moving to each new one over
    /// `duration` seconds on `curve`.
    pub fn new(duration: f32, curve: Curve, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: crate::fractional::FractionallySizedBox::new()
                .animated(duration, curve)
                .child(child),
        }
    }

    /// Fraction `0.0..=1.0` of the parent's **width** (clamped to `>= 0`).
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.inner = self.inner.width_factor(factor);
        self
    }

    /// Fraction `0.0..=1.0` of the parent's **height** (clamped to `>= 0`).
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.inner = self.inner.height_factor(factor);
        self
    }
}

// Written out rather than macro-forwarded: the inner widget is a `FractionallySizedBox`
// and not a `Container`, and it overrides seven hooks — the four a layout widget has and
// the three this one is for.
impl<Msg: Clone + 'static> Widget<Msg> for AnimatedFractionallySizedBox<Msg> {
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
    fn anim_fractions(&self) -> Option<(Option<f32>, Option<f32>)> {
        Widget::anim_fractions(&self.inner)
    }
    fn anim_duration(&self) -> f32 {
        Widget::anim_duration(&self.inner)
    }
    fn anim_curve(&self) -> Curve {
        Widget::anim_curve(&self.inner)
    }
}

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

    /// A tree with one animated anchor on it, so the runtime has a pair to drive.
    fn anchored(to: frus_core::Alignment) -> AnimatedAlign<()> {
        AnimatedAlign::new(to, 0.10, Curve::Linear, Text::new("x"))
    }

    /// **Mounted, mid-flight, at rest**, for the anchor.
    #[test]
    fn an_anchor_mounts_settled_moves_and_arrives() {
        let mut rt = Runtime::default();
        assert!(
            !rt.advance_offsets(&anchored(frus_core::Alignment::CENTER_LEFT), 1.0),
            "a mount is not a transition"
        );
        let id = WidgetId::ROOT;
        assert_eq!(
            rt.anim_offset(id),
            Some((-1.0, 0.0)),
            "and it adopts the anchor whole"
        );

        // Halfway along a linear curve of 0.10 s: halfway from the left edge to the right.
        assert!(rt.advance_offsets(&anchored(frus_core::Alignment::CENTER_RIGHT), 0.05));
        let mid = rt.anim_offset(id).expect("in flight");
        assert!(mid.0.abs() < 1e-3, "at the centre on the way past: {mid:?}");
        assert_eq!(mid.1, 0.0, "and it did not drift vertically");

        rt.advance_offsets(&anchored(frus_core::Alignment::CENTER_RIGHT), 1.0);
        assert_eq!(rt.anim_offset(id), Some((1.0, 0.0)));
        assert!(
            !rt.advance_offsets(&anchored(frus_core::Alignment::CENTER_RIGHT), 0.05),
            "and stops asking for frames once it is there"
        );
    }

    /// And for the slide, whose pair is a multiple of the child's own box rather than a
    /// share of the free space around it.
    #[test]
    fn a_slide_mounts_settled_moves_and_arrives() {
        let slid = |fx: f32| -> AnimatedSlide<()> {
            AnimatedSlide::new(fx, 0.0, 0.10, Curve::Linear, Text::new("x"))
        };
        let mut rt = Runtime::default();
        assert!(!rt.advance_offsets(&slid(-1.0), 1.0));
        let id = WidgetId::ROOT;
        assert_eq!(rt.anim_offset(id), Some((-1.0, 0.0)));

        assert!(rt.advance_offsets(&slid(0.0), 0.05));
        let mid = rt.anim_offset(id).expect("in flight");
        assert!((mid.0 + 0.5).abs() < 1e-3, "halfway back: {mid:?}");

        rt.advance_offsets(&slid(0.0), 1.0);
        assert_eq!(rt.anim_offset(id), Some((0.0, 0.0)));
    }

    /// **And the paint reads it** — the question the value tests above cannot ask. A
    /// tween the walk never consults moves correctly and changes nothing on the screen,
    /// and this framework has shipped that bug before.
    ///
    /// Both rules are checked, because they are two separate lines in the walk: an
    /// anchor's box slides through the free space, and a slide's box moves by its own
    /// width.
    #[test]
    fn the_paint_uses_the_tweened_offset_and_not_the_target() {
        let mark = |root: &dyn Widget<()>, rt: &Runtime| {
            let ui = build_ui(root, Size::new(100.0, 40.0), rt, &Theme::default());
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(rect.x),
                    _ => None,
                })
                .expect("the mark")
        };
        let red = frus_core::Color::rgb(1.0, 0.0, 0.0);
        let mark_widget = || {
            crate::Container::<()>::new()
                .width(20.0)
                .height(20.0)
                .color(red)
        };

        // An anchor crossing a 100 px box: at the halfway point the 20 px mark sits at 40,
        // the middle of the 80 px of free space — not at 0 and not at 80.
        let aligned = |to: frus_core::Alignment| {
            crate::Container::<()>::new()
                .width(100.0)
                .height(40.0)
                .animated_alignment(to, 0.10, Curve::Linear)
                .child(mark_widget())
        };
        let mut rt = Runtime::default();
        rt.advance_offsets(&aligned(frus_core::Alignment::CENTER_LEFT), 1.0);
        rt.advance_offsets(&aligned(frus_core::Alignment::CENTER_RIGHT), 0.05);
        let x = mark(&aligned(frus_core::Alignment::CENTER_RIGHT), &rt);
        assert!(
            (x - 40.0).abs() < 0.5,
            "the anchor's tween reached the paint: {x}"
        );

        // A slide of one whole width, half done: the 20 px mark has moved 10.
        let slid = |fx: f32| AnimatedSlide::new(fx, 0.0, 0.10, Curve::Linear, mark_widget());
        let mut rt = Runtime::default();
        rt.advance_offsets(&slid(0.0), 1.0);
        rt.advance_offsets(&slid(1.0), 0.05);
        let x = mark(&slid(1.0), &rt);
        assert!(
            (x - 10.0).abs() < 0.5,
            "the slide's tween reached the paint: {x}"
        );
    }

    /// The anchor is interpolated in **its own** coordinates, so a directional one stays
    /// directional the whole way: in a right-to-left script the mirrored movement is the
    /// mirror of the movement, rather than a slide between two already-mirrored ends.
    #[test]
    fn a_directional_anchor_mirrors_as_a_whole() {
        let start = frus_core::AlignmentDirectional::CENTER_START;
        let end = frus_core::AlignmentDirectional::CENTER_END;
        let aligned = |to: frus_core::AlignmentDirectional| {
            crate::Container::<()>::new()
                .width(100.0)
                .height(40.0)
                .animated_alignment(to, 0.10, Curve::Linear)
                .child(
                    crate::Container::<()>::new()
                        .width(20.0)
                        .height(20.0)
                        .color(frus_core::Color::rgb(1.0, 0.0, 0.0)),
                )
        };
        let mut rt = Runtime::default();
        rt.advance_offsets(&aligned(start), 1.0);
        // A quarter of the way from the start edge towards the end one.
        rt.advance_offsets(&aligned(end), 0.025);

        let x_in = |rtl: bool| {
            let theme = match rtl {
                true => Theme::default().rtl(),
                false => Theme::default(),
            };
            let ui = build_ui(&aligned(end), Size::new(100.0, 40.0), &rt, &theme);
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(rect.x),
                    _ => None,
                })
                .expect("the mark")
        };
        let ltr = x_in(false);
        let rtl = x_in(true);
        assert!(
            (ltr - 20.0).abs() < 0.5,
            "a quarter across, from the left: {ltr}"
        );
        assert!(
            (rtl - 60.0).abs() < 0.5,
            "and the same quarter from the right: {rtl}"
        );
    }

    /// A stack with one animated layer on it, pinned at `left` and 20 px wide.
    fn pinned_at(left: f32) -> crate::Stack<()> {
        crate::Stack::new().layer(
            AnimatedPositioned::new(
                0.10,
                Curve::Linear,
                crate::Container::<()>::new()
                    .color(frus_core::Color::rgb(1.0, 0.0, 0.0))
                    .child(Text::new("x")),
            )
            .left(left)
            .top(0.0)
            .width(20.0)
            .height(20.0),
        )
    }

    /// **Mounted, mid-flight, at rest** for a layer's pins.
    ///
    /// A layer that appears already pinned somewhere **adopts** its pins rather than
    /// travelling to them from the last place a stack would have put it — the same mount
    /// rule as every other implicit animation here, and the reason a page does not
    /// rearrange itself when it opens.
    #[test]
    fn a_layer_mounts_settled_moves_and_arrives() {
        let mut rt = Runtime::default();
        assert!(
            !rt.advance_pins(&pinned_at(0.0), 1.0),
            "a mount is not a transition"
        );
        // Child 0 of the stack, which is the layer.
        let id = WidgetId::ROOT.child(0);
        assert_eq!(rt.anim_pins(id).and_then(|p| p.left), Some(0.0));

        // Halfway along a linear curve of 0.10 s: halfway between 0 and 80.
        assert!(rt.advance_pins(&pinned_at(80.0), 0.05));
        let mid = rt.anim_pins(id).expect("in flight");
        assert!(
            mid.left.is_some_and(|l| (l - 40.0).abs() < 1e-3),
            "halfway between the two: {mid:?}"
        );
        assert_eq!(
            mid.width,
            Some(20.0),
            "and the extents it never asked to move stayed"
        );

        rt.advance_pins(&pinned_at(80.0), 1.0);
        assert_eq!(rt.anim_pins(id).and_then(|p| p.left), Some(80.0));
        assert!(
            !rt.advance_pins(&pinned_at(80.0), 0.05),
            "and stops asking for frames once it is there"
        );
    }

    /// **A pin that is unset does not travel**, in either direction.
    ///
    /// `None` on an edge is not nought there — it is a layer not pinned on that side at
    /// all, sized by itself and placed by the stack's alignment. Sliding between the two
    /// would be interpolating between two arrangements, and the number it passed through
    /// on the way would mean neither of them.
    #[test]
    fn a_pin_that_is_unset_takes_hold_and_lets_go_at_once() {
        let layer = |left: Option<f32>, right: Option<f32>| {
            let mut l: AnimatedPositioned<()> =
                AnimatedPositioned::new(0.10, Curve::Linear, Text::new("x")).top(0.0);
            if let Some(px) = left {
                l = l.left(px);
            }
            if let Some(px) = right {
                l = l.right(px);
            }
            crate::Stack::<()>::new().layer(l)
        };
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        rt.advance_pins(&layer(Some(0.0), None), 1.0);

        // Swap which edge is pinned, and step a fifth of the way in.
        rt.advance_pins(&layer(None, Some(10.0)), 0.02);
        let mid = rt.anim_pins(id).expect("present");
        assert_eq!(
            mid.left, None,
            "the edge that let go let go at once: {mid:?}"
        );
        assert_eq!(
            mid.right,
            Some(10.0),
            "and the edge that took hold took hold at once: {mid:?}"
        );
    }

    /// **And the stack lays the layer out where the tween says.** The value tests above
    /// cannot ask this: pins that move correctly and are read from the widget instead of
    /// the runtime give a layer that jumps, and every one of them still passes.
    #[test]
    fn the_stack_places_the_layer_by_the_tween_and_not_the_target() {
        let mut rt = Runtime::default();
        rt.advance_pins(&pinned_at(0.0), 1.0);
        rt.advance_pins(&pinned_at(80.0), 0.05);
        let ui = build_ui(
            &pinned_at(80.0),
            Size::new(100.0, 40.0),
            &rt,
            &Theme::default(),
        );
        let x = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(rect.x),
                _ => None,
            })
            .expect("the layer");
        assert!(
            (x - 40.0).abs() < 0.5,
            "halfway across, not at either end: {x}"
        );
    }

    /// A box taking half its parent, then all of it.
    fn share(width_factor: f32) -> crate::Flex<()> {
        crate::Flex::column().width(100.0).height(40.0).child(
            AnimatedFractionallySizedBox::new(
                0.10,
                Curve::Linear,
                crate::Container::<()>::new()
                    .flex(1.0)
                    .height(20.0)
                    .color(frus_core::Color::rgb(1.0, 0.0, 0.0)),
            )
            .width_factor(width_factor),
        )
    }

    /// **Mounted, mid-flight, at rest** for a share of the parent.
    #[test]
    fn a_share_of_the_parent_mounts_settled_moves_and_arrives() {
        let mut rt = Runtime::default();
        assert!(
            !rt.advance_fractions(&share(0.25), 1.0),
            "a mount is not a transition"
        );
        let id = WidgetId::ROOT.child(0);
        assert_eq!(rt.anim_fractions(id), Some((Some(0.25), None)));

        assert!(rt.advance_fractions(&share(0.75), 0.05));
        let (width, height) = rt.anim_fractions(id).expect("in flight");
        assert!(
            width.is_some_and(|w| (w - 0.5).abs() < 1e-3),
            "halfway between a quarter and three quarters: {width:?}"
        );
        assert_eq!(
            height, None,
            "and the axis nobody set still follows its content"
        );

        rt.advance_fractions(&share(0.75), 1.0);
        assert_eq!(rt.anim_fractions(id), Some((Some(0.75), None)));
        assert!(
            !rt.advance_fractions(&share(0.75), 0.05),
            "and stops asking for frames once it is there"
        );
    }

    /// **And the layout reads it.** A fraction is consumed while the tree is measured, so
    /// the check is the width the child actually came out — half of a hundred, halfway
    /// between a quarter of it and three quarters.
    #[test]
    fn the_layout_uses_the_tweened_fraction_and_not_the_target() {
        let mut rt = Runtime::default();
        rt.advance_fractions(&share(0.25), 1.0);
        rt.advance_fractions(&share(0.75), 0.05);
        let ui = build_ui(&share(0.75), Size::new(100.0, 40.0), &rt, &Theme::default());
        let width = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(rect.width),
                _ => None,
            })
            .expect("the child's background");
        assert!(
            (width - 50.0).abs() < 0.5,
            "half the parent, on the way from a quarter to three quarters: {width}"
        );
    }
}
