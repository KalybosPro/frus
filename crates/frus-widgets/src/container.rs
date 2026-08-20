//! [`Container`]: a decorated box (size, padding, color, rounded corners, border,
//! click) with an optional child.

use frus_core::{
    AlignmentGeometry, Border, BorderRadius, BoxDecoration, BoxShadow, Color, Curve, Insets,
    LinearGradient, Rect, Scene, Size,
};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::interaction::{Interaction, Status};
use crate::theme::Theme;
use crate::widget::Widget;

/// An easing curve (smoothstep) to soften the transitions.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A decorated rectangular box.
pub struct Container<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    flex_shrink: f32,
    padding: Insets,
    /// The **outer** margin (around the box, outside the decoration).
    margin: Insets,
    radius: BorderRadius,
    border_width: f32,
    border_color: Color,
    color: Option<Color>,
    hover_color: Option<Color>,
    pressed_color: Option<Color>,
    /// The gradient: (end color, direction in `[0,1]²` space).
    gradient: Option<(Color, [f32; 2])>,
    /// The shadow: (dx, dy, blur, color).
    shadow: Option<(f32, f32, f32, Color)>,
    on_click: Option<Msg>,
    on_long_press: Option<Msg>,
    /// A repaint boundary: it caches the painted subtree (see
    /// [`crate::Widget::repaint_boundary`]).
    repaint_boundary: bool,
    /// A **group** opacity `[0,1]` applied to the whole subtree. `None` = opaque.
    opacity: Option<f32>,
    /// When the group opacity is **animated**: the transition's `(duration, curve)`.
    /// `None` = a fixed opacity, with no transition.
    opacity_anim: Option<(f32, Curve)>,
    /// When the background is an **animated color**: the transition's `(target,
    /// duration, curve)`. `None` = a fixed color.
    color_anim: Option<(Color, f32, Curve)>,
    /// When the **size** is animated: `(target, duration, curve)` — interpolated at
    /// layout time. `None` = a fixed size.
    size_anim: Option<(Size, f32, Curve)>,
    /// When the **corner radius** is animated: `(target, duration, curve)` —
    /// interpolated at paint time. `None` = a fixed radius.
    radius_anim: Option<(BorderRadius, f32, Curve)>,
    /// When the **padding** is animated: `(duration, curve)` — the target is
    /// `self.padding`, interpolated at layout time. `None` = fixed padding.
    padding_anim: Option<(f32, Curve)>,
    /// The child's anchoring within the box, physical or directional (resolved for
    /// RTL at render time). `None` = the default flex behaviour, in which the child
    /// stretches to fill.
    alignment: Option<AlignmentGeometry>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Container<Msg> {
    /// Creates an empty container (automatic size, no decoration).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::ZERO,
            margin: Insets::ZERO,
            radius: BorderRadius::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            color: None,
            hover_color: None,
            pressed_color: None,
            gradient: None,
            shadow: None,
            on_click: None,
            on_long_press: None,
            repaint_boundary: false,
            opacity: None,
            opacity_anim: None,
            color_anim: None,
            size_anim: None,
            radius_anim: None,
            padding_anim: None,
            alignment: None,
            children: Vec::new(),
        }
    }

    /// The **effective** layout padding: the inner padding plus, when a border is
    /// visible, the room it reserves (the content of a bordered box is not eaten by
    /// the stroke). The single source for `style()` and for the **target** of an
    /// animated padding (`anim_padding`).
    fn effective_padding(&self) -> Insets {
        let mut padding = self.padding;
        if Border::new(self.border_width, self.border_color).is_visible() {
            padding.top += self.border_width;
            padding.right += self.border_width;
            padding.bottom += self.border_width;
            padding.left += self.border_width;
        }
        padding
    }

    /// Marks this container as a **repaint boundary**: its subtree is cached and
    /// reused for as long as its geometry and its descendants' interaction state
    /// stay stable. To be placed around **static** content that would otherwise be
    /// repainted on every frame of a neighbouring animation.
    pub fn repaint_boundary(mut self) -> Self {
        self.repaint_boundary = true;
        self
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// How much of a row's deficit this box absorbs. The default is `0.0` — the
    /// reference's rule, where an inflexible child is never squeezed and a row that does
    /// not fit overflows and says so.
    ///
    /// `shrink(1.0)` asks for flexbox's behaviour instead: give way rather than let the
    /// row run over. It is the right answer for a box whose size is a preference rather
    /// than a requirement, and the wrong one for fixed chrome — an icon button at the end
    /// of a row should keep its width however long the label beside it grows.
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }

    /// This box never shrinks — the default said out loud, kept because a layout that
    /// depends on it reads better for saying so. See [`Self::shrink`].
    pub fn no_shrink(self) -> Self {
        self.shrink(0.0)
    }

    /// Uniform inner padding, in logical pixels.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// Inner padding per side (top, right, bottom, left).
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// A uniform **outer** margin: space reserved **around** the box — outside the
    /// decoration, it pushes the siblings away without growing the background or
    /// the border.
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = Insets::uniform(margin);
        self
    }

    /// Outer margin per side (top, right, bottom, left).
    pub fn margin_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.margin = Insets::new(top, right, bottom, left);
        self
    }

    /// The rounded corner radii: uniform via `f32` (`.radius(10.0)`) or per corner
    /// via [`BorderRadius`] (`.radius(BorderRadius::top(12.0))`).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// The border: width (in px) and color.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    /// The background color at rest.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The background color on hover.
    pub fn hover_color(mut self, color: Color) -> Self {
        self.hover_color = Some(color);
        self
    }

    /// The background color when pressed.
    pub fn pressed_color(mut self, color: Color) -> Self {
        self.pressed_color = Some(color);
        self
    }

    /// A linear background gradient (`color` → `end`), with `dir` in `[0,1]²` space
    /// (`[0.0, 1.0]` = top→bottom, for instance).
    pub fn gradient(mut self, end: Color, dir: [f32; 2]) -> Self {
        self.gradient = Some((end, dir));
        self
    }

    /// A drop shadow: the `(dx, dy)` offset, the blur radius and the color.
    pub fn shadow(mut self, dx: f32, dy: f32, blur: f32, color: Color) -> Self {
        self.shadow = Some((dx, dy, blur, color));
        self
    }

    /// The message emitted when the container is clicked.
    pub fn on_click(mut self, message: Msg) -> Self {
        self.on_click = Some(message);
        self
    }

    /// The message emitted by a **long press** (a press held without movement). A
    /// long press supersedes the click.
    pub fn on_long_press(mut self, message: Msg) -> Self {
        self.on_long_press = Some(message);
        self
    }

    /// Sets the container's child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// Applies a **group opacity** `[0,1]` to the whole subtree, as one block: the
    /// rendering goes through a composited layer, so overlaps are not
    /// double-blended. `1.0` = no effect at all.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self
    }

    /// Like [`Container::opacity`], but the opacity **animates** toward `opacity` on
    /// every change, with `duration` (in seconds) and `curve`. The fade applies to
    /// the whole group.
    pub fn animated_opacity(mut self, opacity: f32, duration: f32, curve: Curve) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self.opacity_anim = Some((duration, curve));
        self
    }

    /// A background whose color **animates** toward `color` on every change, with
    /// `duration` (in seconds) and `curve`. The runtime interpolates from the
    /// current color; on mount it adopts `color` with no transition. Note: one box
    /// shares a single `(duration, curve)` across its animations (opacity, color).
    pub fn animated_color(mut self, color: Color, duration: f32, curve: Curve) -> Self {
        self.color = Some(color);
        self.color_anim = Some((color, duration, curve));
        self
    }

    /// A box whose **size** animates toward `width×height` on every change, with
    /// `duration` and `curve`. The interpolated size is injected **at layout time**
    /// (the children reposition accordingly). On mount it adopts the target with no
    /// transition.
    pub fn animated_size(mut self, width: f32, height: f32, duration: f32, curve: Curve) -> Self {
        self.width = Dimension::Length(width);
        self.height = Dimension::Length(height);
        self.size_anim = Some((Size::new(width, height), duration, curve));
        self
    }

    /// A box whose **corner radius** animates toward `radius` on every change, with
    /// `duration` and `curve` — the corners morph smoothly. Uniform via `f32`, or
    /// per corner via [`BorderRadius`]. On mount it adopts the target with no
    /// transition.
    pub fn animated_radius(
        mut self,
        radius: impl Into<BorderRadius>,
        duration: f32,
        curve: Curve,
    ) -> Self {
        let radius = radius.into();
        self.radius = radius;
        self.radius_anim = Some((radius, duration, curve));
        self
    }

    /// A box whose (uniform) **inner padding** animates toward `padding` on every
    /// change, with `duration` and `curve`. The interpolated padding is injected
    /// **at layout time** (the content repositions). On mount it adopts the target
    /// with no transition.
    pub fn animated_padding(mut self, padding: f32, duration: f32, curve: Curve) -> Self {
        self.padding = Insets::uniform(padding);
        self.padding_anim = Some((duration, curve));
        self
    }

    /// **Anchors the child** within the box: centred, in a corner, against an edge…
    /// Accepts a **physical** anchor ([`Alignment`](frus_core::Alignment)) or a
    /// **directional** one
    /// ([`AlignmentDirectional`](frus_core::AlignmentDirectional), resolved for RTL
    /// at render time) — both through `Into`. By default, with no anchor, the child
    /// stretches to fill the container (the flex behaviour); setting an anchor
    /// leaves the child at its natural size and positions it.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    /// Applies a **composite decoration** as one block: background, gradient, border,
    /// radius and shadow gathered in a reusable [`BoxDecoration`]. Each part present
    /// overrides the corresponding setting; the radius is always adopted. The
    /// animations (color, radius…) still apply on top. (A shadow's `spread` is not
    /// kept — the container's shadow model has none.)
    pub fn decoration(mut self, decoration: BoxDecoration) -> Self {
        if let Some(color) = decoration.color {
            self.color = Some(color);
        }
        if let Some(gradient) = decoration.gradient {
            self.gradient = Some((gradient.end, gradient.direction));
        }
        if let Some(border) = decoration.border {
            self.border_width = border.width;
            self.border_color = border.color;
        }
        self.radius = decoration.radius;
        if let Some(shadow) = decoration.shadow {
            self.shadow = Some((shadow.offset.0, shadow.offset.1, shadow.blur, shadow.color));
        }
        self
    }
}

impl<Msg> Default for Container<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Container<Msg> {
    fn style(&self) -> Style {
        let mut style = Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            padding: self.effective_padding(),
            margin: self.margin,
            ..Default::default()
        };
        // Anchoring the child: taffy is left to place it at the **top left** of the
        // content box, at its natural size (Start / Start, no stretching), and the
        // walk then offsets it within the free space according to the `Alignment`'s
        // fractions (a manual, fractional placement — outside the discrete flex).
        if self.alignment.is_some() {
            style.justify = Justify::Start;
            style.align = Align::Start;
        }
        style
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        // A background with an **animated color**: the color the runtime interpolates
        // wins (the hover/press interpolation does not apply to an animated
        // background). Pressed: instant. Otherwise, an animated rest → hover
        // transition.
        let color = if self.color_anim.is_some() {
            status.anim_color.or(self.color)
        } else if status.interaction == Interaction::Pressed {
            self.pressed_color.or(self.hover_color).or(self.color)
        } else if let (Some(base), Some(hover)) = (self.color, self.hover_color) {
            Some(base.lerp(hover, ease(status.hover_progress)))
        } else {
            self.color
        };

        // Compose the decoration (background/gradient/border/shadow) and lower it into
        // primitives in the fixed order shadow → background → border. The opacity
        // (the fade-in) modulates every color.
        // An animated radius: the radius the runtime interpolates wins over the fixed one.
        let radius = if self.radius_anim.is_some() {
            status.anim_radius.unwrap_or(self.radius)
        } else {
            self.radius
        };

        let decoration = BoxDecoration {
            color,
            gradient: self
                .gradient
                .map(|(end, dir)| LinearGradient::new(end, dir)),
            border: (self.border_width > 0.0)
                .then(|| Border::new(self.border_width, self.border_color)),
            radius,
            shadow: self
                .shadow
                .map(|(dx, dy, blur, c)| BoxShadow::new(dx, dy, blur, c)),
        };
        decoration.paint_into(scene, bounds, status.opacity);
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.on_long_press.clone()
    }

    fn repaint_boundary(&self) -> bool {
        self.repaint_boundary
    }

    fn opacity_group(&self) -> Option<f32> {
        self.opacity
    }

    /// The animated opacity's target (only when `animated_opacity` is set) — this is
    /// the value the runtime tweens and the walk reads back for the layer.
    fn anim_target(&self) -> Option<f32> {
        self.opacity_anim.as_ref().and(self.opacity)
    }

    fn anim_color(&self) -> Option<Color> {
        self.color_anim.as_ref().and(self.color)
    }

    fn anim_size(&self) -> Option<Size> {
        self.size_anim.as_ref().map(|(s, _, _)| *s)
    }

    fn anim_radius(&self) -> Option<BorderRadius> {
        self.radius_anim.as_ref().map(|(r, _, _)| *r)
    }

    fn anim_padding(&self) -> Option<Insets> {
        // The target = the effective padding (content + border), consistent with `style()`.
        self.padding_anim.as_ref().map(|_| self.effective_padding())
    }

    fn alignment_geometry(&self) -> Option<AlignmentGeometry> {
        self.alignment
    }

    fn anim_duration(&self) -> f32 {
        // One box's animations (opacity/color/size/radius/padding) share a single
        // duration (in the order: opacity, color, size, radius, padding).
        self.opacity_anim
            .as_ref()
            .map(|(d, _)| *d)
            .or(self.color_anim.as_ref().map(|(_, d, _)| *d))
            .or(self.size_anim.as_ref().map(|(_, d, _)| *d))
            .or(self.radius_anim.as_ref().map(|(_, d, _)| *d))
            .or(self.padding_anim.as_ref().map(|(d, _)| *d))
            .unwrap_or(crate::runtime::ANIM_DURATION)
    }

    fn anim_curve(&self) -> Curve {
        self.opacity_anim
            .as_ref()
            .map(|(_, c)| c.clone())
            .or(self.color_anim.as_ref().map(|(_, _, c)| c.clone()))
            .or(self.size_anim.as_ref().map(|(_, _, c)| c.clone()))
            .or(self.radius_anim.as_ref().map(|(_, _, c)| c.clone()))
            .or(self.padding_anim.as_ref().map(|(_, c)| c.clone()))
            .unwrap_or(Curve::Linear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box in an over-budget row **keeps its width** — the reference's rule, and the
    /// default since milestone 349. Flexbox's default was the opposite, and the smallest
    /// fixed thing in the row paid for the whole deficit: milestone 333 found an icon
    /// button at 13 px of 40, drawn off the card and out of the hit registry.
    ///
    /// `shrink(1.0)` asks for flexbox's behaviour back, for a box that would rather give
    /// way than let the row overflow.
    #[test]
    fn a_box_in_an_over_budget_row_keeps_its_width() {
        use crate::interaction::WidgetId;
        use crate::runtime::Runtime;
        use crate::theme::Theme;
        use crate::Flex;

        let widths = |row: &Flex<()>| -> Vec<f32> {
            let runtime = Runtime::default();
            let theme = Theme::light();
            let mut layout = frus_layout::Layout::new();
            let node = crate::ui::build_layout(row, WidgetId::ROOT, &runtime, &theme, &mut layout);
            layout.compute_filled(node, 100.0, 50.0);
            layout
                .absolute_rects(node)
                .iter()
                .skip(1)
                .take(2)
                .map(|(r, _)| r.width)
                .collect()
        };

        // 120 px of children in 100 px of row. Nobody was asked to give way, so nobody
        // does, and the row overflows — which is now visible, striped and labelled.
        let row = Flex::row()
            .child(Container::<()>::new().width(80.0).height(20.0))
            .child(Container::<()>::new().width(40.0).height(20.0));
        assert_eq!(
            widths(&row),
            vec![80.0, 40.0],
            "an inflexible child is never squeezed"
        );

        // Unless it says so, and then the whole deficit is its own.
        let giving_way = Flex::row()
            .child(Container::<()>::new().width(80.0).height(20.0).shrink(1.0))
            .child(Container::<()>::new().width(40.0).height(20.0));
        assert_eq!(widths(&giving_way), vec![60.0, 40.0]);
    }

    /// A `Container` with a group opacity < 1 has its painted subtree wrapped in a
    /// [`frus_core::Primitive::Layer`] at that opacity.
    #[test]
    fn opacity_group_wraps_subtree_in_a_layer() {
        use frus_core::{Primitive, Size};
        let root: Container<()> = Container::new()
            .width(40.0)
            .height(40.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .opacity(0.5);
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(64.0, 64.0), &rt, &theme);
        let layer = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Layer {
                opacity,
                primitives,
                ..
            } => Some((*opacity, primitives.len())),
            _ => None,
        });
        let (op, n) = layer.expect("a group opacity layer");
        assert!((op - 0.5).abs() < 1e-6, "group opacity = {op}");
        assert!(
            n >= 1,
            "the layer wraps the painted content ({n} primitives)"
        );
    }

    /// Full opacity (`1.0`): no layer is emitted (the opaque path, at zero cost).
    #[test]
    fn full_opacity_emits_no_layer() {
        use frus_core::{Primitive, Size};
        let root: Container<()> = Container::new()
            .width(40.0)
            .height(40.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .opacity(1.0);
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(64.0, 64.0), &rt, &theme);
        assert!(
            !ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Layer { .. })),
            "no layer at full opacity"
        );
    }

    /// `animated_opacity` declares an animated value (the runtime tweens it) with the
    /// duration and curve supplied; `opacity` alone does not (a fixed opacity).
    #[test]
    fn animated_opacity_declares_anim_target() {
        let animated: Container<()> = Container::new().animated_opacity(0.0, 0.3, Curve::ease_in());
        assert_eq!(Widget::<()>::anim_target(&animated), Some(0.0));
        assert_eq!(Widget::<()>::anim_duration(&animated), 0.3);
        assert_eq!(Widget::<()>::anim_curve(&animated), Curve::ease_in());
        assert_eq!(Widget::<()>::opacity_group(&animated), Some(0.0));

        // A fixed opacity: a group, yes, but no animated value.
        let fixed: Container<()> = Container::new().opacity(0.5);
        assert_eq!(Widget::<()>::anim_target(&fixed), None);
        assert_eq!(Widget::<()>::opacity_group(&fixed), Some(0.5));
    }

    /// An `animated_color` background paints the color the runtime **interpolates**
    /// (not the target): mounted at red, transitioning to blue half-way → the
    /// background rectangle is about half red, half blue.
    #[test]
    fn animated_color_paints_the_interpolated_color() {
        use frus_core::{Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let mut rt = crate::runtime::Runtime::default();
        let start: Container<()> =
            Container::new()
                .width(20.0)
                .height(20.0)
                .animated_color(red, 0.10, Curve::Linear);
        rt.advance_colors(&start, 1.0); // montage → rouge
        let to_blue: Container<()> =
            Container::new()
                .width(20.0)
                .height(20.0)
                .animated_color(blue, 0.10, Curve::Linear);
        rt.advance_colors(&to_blue, 0.05); // t = 0.5

        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&to_blue, Size::new(20.0, 20.0), &rt, &theme);
        let color = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .expect("un rectangle de fond");
        assert!(
            (color.r - 0.5).abs() < 0.1 && (color.b - 0.5).abs() < 0.1,
            "the interpolated color is painted: {color:?}"
        );
    }

    /// An `animated_size` **drives the layout**: half-way through (20×20 → 40×40,
    /// linear), the background rectangle measures about 30×30 (the interpolated
    /// size injected into layout through `effective_style`).
    #[test]
    fn animated_size_drives_the_layout() {
        use frus_core::{Primitive, Size};
        let mut rt = crate::runtime::Runtime::default();
        let red = Color::rgb(1.0, 0.0, 0.0);
        let start: Container<()> =
            Container::new()
                .color(red)
                .animated_size(20.0, 20.0, 0.10, Curve::Linear);
        rt.advance_sizes(&start, 1.0); // montage → 20×20
        let to_big: Container<()> =
            Container::new()
                .color(red)
                .animated_size(40.0, 40.0, 0.10, Curve::Linear);
        rt.advance_sizes(&to_big, 0.05); // t = 0.5 → 30×30

        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&to_big, Size::new(100.0, 100.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("un rectangle de fond");
        assert!(
            (rect.width - 30.0).abs() < 1.0,
            "interpolated width: {}",
            rect.width
        );
        assert!(
            (rect.height - 30.0).abs() < 1.0,
            "interpolated height: {}",
            rect.height
        );
    }

    /// An `animated_radius` paints the **interpolated** radius: mounted at 0,
    /// transitioning to 20 half-way → the background rectangle has a radius of ~10.
    #[test]
    fn animated_radius_paints_the_interpolated_radius() {
        use frus_core::{Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let mut rt = crate::runtime::Runtime::default();
        let sharp: Container<()> = Container::new()
            .width(40.0)
            .height(40.0)
            .color(red)
            .animated_radius(0.0, 0.10, Curve::Linear);
        rt.advance_radii(&sharp, 1.0); // montage → 0
        let round: Container<()> = Container::new()
            .width(40.0)
            .height(40.0)
            .color(red)
            .animated_radius(20.0, 0.10, Curve::Linear);
        rt.advance_radii(&round, 0.05); // t = 0.5 → 10

        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&round, Size::new(60.0, 60.0), &rt, &theme);
        let radius = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { radius, .. } => Some(*radius),
                _ => None,
            })
            .expect("un rectangle de fond");
        assert!(
            (radius.top_left - 10.0).abs() < 1.0,
            "interpolated radius: {}",
            radius.top_left
        );
    }

    /// An `animated_padding` **insets the child at layout time**: mounted at 0,
    /// transitioning to 20 half-way → the child is inset by ~10 (the interpolated
    /// padding).
    #[test]
    fn animated_padding_insets_the_child_at_layout() {
        use frus_core::{Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let build = |pad: f32| {
            Container::<()>::new()
                .width(60.0)
                .height(60.0)
                .animated_padding(pad, 0.10, Curve::Linear)
                .child(Container::new().width(20.0).height(20.0).color(red))
        };
        let mut rt = crate::runtime::Runtime::default();
        rt.advance_paddings(&build(0.0), 1.0); // montage → 0
        let bigger = build(20.0);
        rt.advance_paddings(&bigger, 0.05); // t = 0.5 → 10

        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&bigger, Size::new(60.0, 60.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("the child's red background");
        assert!(
            (rect.x - 10.0).abs() < 1.0 && (rect.y - 10.0).abs() < 1.0,
            "the child is inset by ~10 by the interpolated padding: {rect:?}"
        );
    }

    /// `alignment(Center)` positions the (20×20) child at the centre of a 100×100 box
    /// → its background sits at about (40, 40).
    #[test]
    fn alignment_centers_the_child() {
        use frus_core::{Alignment, Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root: Container<()> = Container::new()
            .width(100.0)
            .height(100.0)
            .alignment(Alignment::CENTER)
            .child(Container::new().width(20.0).height(20.0).color(red));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 100.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("the child's red background");
        assert!(
            (rect.x - 40.0).abs() < 1.0 && (rect.y - 40.0).abs() < 1.0,
            "centred: {rect:?}"
        );
    }

    /// `alignment(BottomRight)` anchors the (20×20) child to the bottom-right corner
    /// of a 100×100 box → its background sits at about (80, 80).
    #[test]
    fn alignment_anchors_child_to_a_corner() {
        use frus_core::{Alignment, Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root: Container<()> = Container::new()
            .width(100.0)
            .height(100.0)
            .alignment(Alignment::BOTTOM_RIGHT)
            .child(Container::new().width(20.0).height(20.0).color(red));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 100.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("the child's red background");
        assert!(
            (rect.x - 80.0).abs() < 1.0 && (rect.y - 80.0).abs() < 1.0,
            "bottom-right corner: {rect:?}"
        );
    }

    /// A **fractional** anchor (outside the nine discrete positions) places the child
    /// proportionally: `x = 0.5, y = -0.5` → fractions (0.75, 0.25) → in a 100×100
    /// box with a 20×20 child (80×80 free), the background lands at about (60, 20).
    /// This is what the discrete flex could not do — and what makes
    /// `Tween<Alignment>` visually continuous.
    #[test]
    fn fractional_alignment_places_child_proportionally() {
        use frus_core::{Alignment, Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root: Container<()> = Container::new()
            .width(100.0)
            .height(100.0)
            .alignment(Alignment::new(0.5, -0.5))
            .child(Container::new().width(20.0).height(20.0).color(red));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 100.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("the child's red background");
        assert!(
            (rect.x - 60.0).abs() < 1.0 && (rect.y - 20.0).abs() < 1.0,
            "fractionnel : {rect:?}"
        );
    }

    /// A **directional** anchor follows the reading direction: `CENTER_START` places
    /// the child on the **left** in LTR, on the **right** in RTL (resolved at render
    /// time).
    #[test]
    fn directional_alignment_flips_the_child_in_rtl() {
        use frus_core::{AlignmentDirectional, Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let rt = crate::runtime::Runtime::default();
        let child_x = |theme: &crate::Theme| {
            let root: Container<()> = Container::new()
                .width(100.0)
                .height(100.0)
                .alignment(AlignmentDirectional::CENTER_START)
                .child(Container::new().width(20.0).height(20.0).color(red));
            let ui = crate::ui::build_ui(&root, Size::new(100.0, 100.0), &rt, theme);
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(rect.x),
                    _ => None,
                })
                .expect("the child's red background")
        };
        assert!(
            child_x(&crate::Theme::dark()).abs() < 1.0,
            "start on the left in LTR"
        );
        assert!(
            (child_x(&crate::Theme::dark().rtl()) - 80.0).abs() < 1.0,
            "start on the right in RTL"
        );
    }

    /// `decoration(...)` applies background, radius and border as one block: the
    /// background paints the given color and radius, and the border reserves its
    /// width at layout time.
    #[test]
    fn decoration_applies_composite_fields() {
        use frus_core::{BorderRadius, BoxDecoration, Primitive, Size};
        let green = Color::rgb(0.0, 1.0, 0.0);
        let deco = BoxDecoration {
            color: Some(green),
            radius: BorderRadius::uniform(8.0),
            border: Some(Border::new(2.0, Color::WHITE)),
            ..Default::default()
        };
        let root: Container<()> = Container::new().width(40.0).height(40.0).decoration(deco);
        // The border reserves its width in the layout padding.
        assert_eq!(Widget::style(&root).padding, Insets::uniform(2.0));

        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(40.0, 40.0), &rt, &theme);
        let (color, radius) = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { color, radius, .. } if color.g > 0.5 => Some((*color, *radius)),
                _ => None,
            })
            .expect("the decorated green background");
        assert!(color.g > 0.9, "a green background: {color:?}");
        assert!(
            (radius.top_left - 8.0).abs() < 1e-3,
            "rayon composite : {}",
            radius.top_left
        );
    }

    /// `margin(...)` reserves space **around** the box: it pushes the siblings away
    /// and offsets the background without growing it. In a column, a second child
    /// (20 tall) with a margin of 10 starts at `y = 20 (sibling) + 10 (margin)` and
    /// is inset by 10 on the left.
    #[test]
    fn margin_pushes_siblings_and_insets() {
        use frus_core::{Primitive, Size};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(Container::new().height(20.0).color(red))
            .child(Container::new().height(20.0).margin(10.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the 2nd child's green background");
        assert!(
            (rect.y - 30.0).abs() < 0.5 && (rect.x - 10.0).abs() < 0.5,
            "margin: pushed to y=30, inset to x=10: {rect:?}"
        );
        assert!(
            (rect.height - 20.0).abs() < 0.5,
            "the margin does not grow the box: {rect:?}"
        );
    }

    #[test]
    fn visible_border_reserves_layout_padding() {
        // A visible border: the layout padding reserves its width.
        let bordered: Container<()> = Container::new().padding(4.0).border(2.0, Color::WHITE);
        assert_eq!(Widget::style(&bordered).padding, Insets::uniform(6.0));

        // With no border (or an invisible one): the padding is unchanged.
        let plain: Container<()> = Container::new().padding(4.0);
        assert_eq!(Widget::style(&plain).padding, Insets::uniform(4.0));
        let invisible: Container<()> = Container::new()
            .padding(4.0)
            .border(2.0, Color::TRANSPARENT);
        assert_eq!(Widget::style(&invisible).padding, Insets::uniform(4.0));
    }
}
