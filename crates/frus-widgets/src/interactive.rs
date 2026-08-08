//! [`InteractiveViewer`]: a viewport that lets its child be **panned** and
//! **zoomed**. The transformation (scale + translation) is **state retained** in
//! the runtime, driven by the shell's gestures (drag to pan, wheel or pinch to
//! zoom).

use frus_core::{Affine, Point, Rect, Scene};

/// The friction of a released pan's *fling* (exponential decay, per second).
pub(crate) const PAN_FRICTION: f32 = 6.0;
/// Below this speed (px/s), the fling stops and comes to rest.
pub(crate) const PAN_MIN_VELOCITY: f32 = 24.0;
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The **retained** state of an [`InteractiveViewer`]: the scale factor and the
/// translation applied to the child. The screen point `q` receives the content
/// painted flat at `p` according to `q = scale · p + (tx, ty)`. The identity
/// (`scale = 1`, zero translation) places the child as is in the viewport.
///
/// All the **gesture mathematics** lives here (pure, testable): the shell merely
/// calls [`pan`](InteractiveView::pan) and [`zoom_at`](InteractiveView::zoom_at),
/// then renders the [`matrix`](InteractiveView::matrix).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractiveView {
    /// The current scale factor (`1.0` = natural size).
    pub scale: f32,
    /// The paint translation, in logical pixels.
    pub tx: f32,
    pub ty: f32,
}

impl Default for InteractiveView {
    fn default() -> Self {
        Self {
            scale: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }
}

impl InteractiveView {
    /// The matrix `q = scale · p + t` (scaling about the origin, then translation).
    pub fn matrix(&self) -> Affine {
        Affine::scale(self.scale, self.scale).then(Affine::translation(self.tx, self.ty))
    }

    /// **Pans** the content by `(dx, dy)` screen pixels: the finger or cursor pushes
    /// the content by the same delta.
    pub fn pan(self, dx: f32, dy: f32) -> Self {
        Self {
            tx: self.tx + dx,
            ty: self.ty + dy,
            ..self
        }
    }

    /// **Clamps** the translation so that the content (at the current scale) always
    /// **covers** the `viewport`: an edge of the content cannot be dragged inside
    /// the viewport. When the content is **smaller** than the viewport (zoomed out
    /// below 1), it is **centred**. At scale 1, panning is nil (the content fills
    /// exactly).
    pub fn clamped(self, viewport: Rect) -> Self {
        // Screen content `q = s·p + t`, with `p` covering the viewport at scale 1. The
        // content's left edge ≤ the viewport's left edge → `t ≤ (1−s)·o`; its right
        // edge ≥ the viewport's right edge → `t ≥ (1−s)·(o+len)`. For `s < 1` the
        // interval flips → centre instead.
        let clamp_axis = |t: f32, s: f32, o: f32, len: f32| -> f32 {
            let hi = (1.0 - s) * o;
            let lo = (1.0 - s) * (o + len);
            if lo <= hi {
                t.clamp(lo, hi)
            } else {
                (lo + hi) * 0.5
            }
        };
        Self {
            scale: self.scale,
            tx: clamp_axis(self.tx, self.scale, viewport.x, viewport.width),
            ty: clamp_axis(self.ty, self.scale, viewport.y, viewport.height),
        }
    }

    /// **Zooms** by `factor` while keeping the screen point `cursor` fixed (a
    /// cursor-anchored zoom), with the final scale clamped to `[min, max]`. The
    /// point of the content under the cursor does not move — the behaviour expected
    /// of a wheel or a pinch.
    pub fn zoom_at(self, factor: f32, cursor: Point, min: f32, max: f32) -> Self {
        let new_scale = (self.scale * factor).clamp(min, max);
        // The **effective** factor after clamping (nil if already at the edge).
        let f = new_scale / self.scale;
        // Pins the point under the cursor: t' = cursor·(1 - f) + f·t.
        Self {
            scale: new_scale,
            tx: cursor.x * (1.0 - f) + f * self.tx,
            ty: cursor.y * (1.0 - f) + f * self.ty,
        }
    }
}

/// A **pannable and zoomable** viewport: its child fills the viewport at scale 1,
/// then the user pans it (by dragging) and zooms it (wheel or pinch) about the
/// cursor. Any content that overflows is **clipped** to the viewport. Ideal for a
/// map, a detailed image, a plan, a diagram.
///
/// Like [`crate::Scroll`], the viewport needs a **bounded size** (otherwise it
/// collapses): a fixed `width`/`height`, or `flex` within a column or row. The
/// scale is bounded by `min_scale` / `max_scale`.
pub struct InteractiveViewer<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    min_scale: f32,
    max_scale: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> InteractiveViewer<Msg> {
    /// An empty interactive viewport (the scale bounded to `0.5×`–`4×` by default).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Length(300.0),
            flex_grow: 0.0,
            min_scale: 0.5,
            max_scale: 4.0,
            children: Vec::new(),
        }
    }

    /// The viewport's fixed width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// The viewport's fixed height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// The **minimum** scale allowed (zooming out).
    pub fn min_scale(mut self, min: f32) -> Self {
        self.min_scale = min;
        self
    }

    /// The **maximum** scale allowed (zooming in).
    pub fn max_scale(mut self, max: f32) -> Self {
        self.max_scale = max;
        self
    }

    /// Sets the pannable and zoomable child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for InteractiveViewer<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for InteractiveViewer<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // The viewport is transparent: only the transformed content is drawn.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn interactive(&self) -> Option<(f32, f32)> {
        Some((self.min_scale, self.max_scale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identity places the content as is: `matrix` is the identity.
    #[test]
    fn default_is_identity() {
        let m = InteractiveView::default().matrix();
        let p = m.apply(Point::new(30.0, 40.0));
        assert!(
            (p.x - 30.0).abs() < 1e-4 && (p.y - 40.0).abs() < 1e-4,
            "identity: {p:?}"
        );
    }

    /// Panning offsets the content by the exact delta.
    #[test]
    fn pan_shifts_the_content() {
        let v = InteractiveView::default().pan(12.0, -5.0);
        let p = v.matrix().apply(Point::new(0.0, 0.0));
        assert!(
            (p.x - 12.0).abs() < 1e-4 && (p.y + 5.0).abs() < 1e-4,
            "offset: {p:?}"
        );
    }

    /// Zooming keeps **the point under the cursor fixed**: that screen point receives
    /// the same point of the content before and after the zoom.
    #[test]
    fn zoom_keeps_the_cursor_point_fixed() {
        let cursor = Point::new(100.0, 60.0);
        let before = InteractiveView::default();
        // The point of the content currently under the cursor (identity → = cursor).
        let content_under = {
            let inv = before.matrix().inverse();
            inv.apply(cursor)
        };
        let after = before.zoom_at(2.0, cursor, 0.5, 4.0);
        assert!((after.scale - 2.0).abs() < 1e-4, "×2: {}", after.scale);
        // That same point of the content must reproject **onto the cursor**.
        let reprojected = after.matrix().apply(content_under);
        assert!(
            (reprojected.x - cursor.x).abs() < 1e-3 && (reprojected.y - cursor.y).abs() < 1e-3,
            "the point under the cursor stays fixed: {reprojected:?}"
        );
    }

    /// Zooming is **clamped**: beyond `max`, the scale saturates and the point under
    /// the cursor stays fixed (a nil effective factor).
    #[test]
    fn zoom_clamps_to_max() {
        let cursor = Point::new(50.0, 50.0);
        let v = InteractiveView {
            scale: 4.0,
            tx: 10.0,
            ty: 20.0,
        };
        let z = v.zoom_at(2.0, cursor, 0.5, 4.0);
        assert!(
            (z.scale - 4.0).abs() < 1e-4,
            "saturated at max: {}",
            z.scale
        );
        assert!(
            (z.tx - 10.0).abs() < 1e-4 && (z.ty - 20.0).abs() < 1e-4,
            "unchanged at the edge"
        );
    }

    /// At scale 1, panning is **cancelled** by the clamping (the content fills the
    /// viewport exactly).
    #[test]
    fn clamp_pins_pan_at_scale_one() {
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        let c = InteractiveView::default().pan(50.0, -30.0).clamped(vp);
        assert!(
            c.tx.abs() < 1e-4 && c.ty.abs() < 1e-4,
            "pan cancelled at scale 1: {c:?}"
        );
    }

    /// Zoomed ×2, panning is clamped so that the content **covers** the viewport: an
    /// excessive pan is brought back to the edge (the ×2 content spans `[t, t+400]`
    /// and must cover `[0, 200]` → `t ∈ [-200, 0]`).
    #[test]
    fn clamp_keeps_zoomed_content_covering() {
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        // A pan too far right (positive t) → brought back to 0 (content's left edge = 0).
        let a = InteractiveView {
            scale: 2.0,
            tx: 500.0,
            ty: 0.0,
        }
        .clamped(vp);
        assert!(
            (a.tx - 0.0).abs() < 1e-4,
            "clamped at the left edge: {}",
            a.tx
        );
        // A pan too far left → brought back to -200 (content's right edge = 200).
        let b = InteractiveView {
            scale: 2.0,
            tx: -900.0,
            ty: 0.0,
        }
        .clamped(vp);
        assert!(
            (b.tx + 200.0).abs() < 1e-4,
            "clamped at the right edge: {}",
            b.tx
        );
        // A moderate pan passes through unchanged.
        let c = InteractiveView {
            scale: 2.0,
            tx: -100.0,
            ty: -50.0,
        }
        .clamped(vp);
        assert!(
            (c.tx + 100.0).abs() < 1e-4 && (c.ty + 50.0).abs() < 1e-4,
            "a valid pan is preserved"
        );
    }

    /// Zoomed out (< 1), content smaller than the viewport is **centred**.
    #[test]
    fn clamp_centers_shrunken_content() {
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        let c = InteractiveView {
            scale: 0.5,
            tx: 999.0,
            ty: -999.0,
        }
        .clamped(vp);
        // Centred: t = (1 − 0.5)·(o + len/2) = 0.5·100 = 50 on each axis.
        assert!(
            (c.tx - 50.0).abs() < 1e-4 && (c.ty - 50.0).abs() < 1e-4,
            "centred: {c:?}"
        );
    }

    /// The walk wraps the child in **a layer transformed and clipped to the
    /// viewport**: a `Primitive::Layer` carrying a matrix and a clip = the viewport.
    #[test]
    fn walk_emits_a_transformed_clipped_layer() {
        use crate::Container;
        use frus_core::{Color, Primitive, Size};
        let root = InteractiveViewer::<()>::new()
            .width(200.0)
            .height(200.0)
            .child(Container::new().color(Color::rgb(1.0, 0.0, 0.0)));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        let (has_xform, clip) = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer {
                    transform, clip, ..
                } => Some((transform.is_some(), *clip)),
                _ => None,
            })
            .expect("un calque interactif");
        assert!(has_xform, "le calque porte la matrice de transformation");
        assert!(
            clip.width <= 200.5 && clip.height <= 200.5,
            "clipped to the viewport: {clip:?}"
        );
    }

    /// A regression: a sibling placed **after** an interactive viewport keeps its
    /// layout place. The viewport is a layout **leaf** (its subtree is laid out
    /// separately); without that, the rectangle index desynchronises and everything
    /// after it overlaps. Here the viewport is 150 tall → the sibling follows at y=150.
    #[test]
    fn sibling_after_viewer_keeps_its_layout_position() {
        use crate::{Container, Flex};
        use frus_core::{Color, Primitive, Size};
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = Flex::<()>::column()
            .width(300.0)
            .child(
                InteractiveViewer::new()
                    .width(300.0)
                    .height(150.0)
                    .child(Container::new().color(Color::rgb(0.2, 0.2, 0.2))),
            )
            .child(Container::new().width(300.0).height(20.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(300.0, 600.0), &rt, &theme);
        let marker_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("le marqueur vert");
        assert!(
            (marker_y - 150.0).abs() < 0.5,
            "the sibling follows the viewport (150 tall), not overlapping: y = {marker_y}"
        );
    }

    /// Hit-testing **goes through the transformation**: after a pan of +50 in x, a
    /// click at the moved position reaches the child, and its former position misses.
    #[test]
    fn walk_pan_shifts_the_hit_test() {
        use crate::interaction::WidgetId;
        use crate::Container;
        use frus_core::{Color, Size};
        let root = InteractiveViewer::<i32>::new()
            .width(200.0)
            .height(200.0)
            .child(
                Container::new()
                    .width(200.0)
                    .height(200.0)
                    .color(Color::rgb(1.0, 0.0, 0.0))
                    .on_click(9),
            );
        let theme = crate::Theme::dark();

        // Identity: the left edge (x = 10) reaches the child.
        let rt = crate::runtime::Runtime::default();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        assert!(
            ui.hit(Point::new(10.0, 100.0)).is_some(),
            "identity: the left edge is reached"
        );

        // After a pan of +50 in x: the content is pushed right; x = 10 falls outside
        // the content (M⁻¹ = -40), but x = 60 lands back in it (M⁻¹ = 10).
        let mut rt = crate::runtime::Runtime::default();
        rt.interactive
            .insert(WidgetId::ROOT, InteractiveView::default().pan(50.0, 0.0));
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        assert!(
            ui.hit(Point::new(10.0, 100.0)).is_none(),
            "pan: the former position misses"
        );
        assert!(
            ui.hit(Point::new(60.0, 100.0)).is_some(),
            "pan: the moved position is reached"
        );
    }
}
