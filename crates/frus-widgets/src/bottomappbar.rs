//! [`BottomAppBar`]: a bar of actions along the bottom of a screen, optionally cut
//! with a **notch** that a docked floating action button sits in.
//!
//! It is the other kind of bottom bar. The navigation one
//! ([`Scaffold::nav`](crate::Scaffold::nav)) answers "which section am I in"; this one
//! carries the actions that belong to the screen you are already on, and leaves room
//! for the one action that matters most to float over it.
//!
//! ```ignore
//! Scaffold::new().size(width, height)
//!     .bottom_app_bar(
//!         BottomAppBar::new()
//!             .child(row![menu_button, spacer(), search_button])
//!     )
//!     .fab_location(FabLocation::EndDocked)
//!     .fab(fab_button("+", Msg::Add))
//!     .build()
//! ```
//!
//! **The notch is not the bar's decision.** Where the button sits, and how big it is,
//! are the scaffold's business — it places both — so the scaffold cuts the notch. A
//! bar built by hand and dropped anywhere else is simply a bar.

use frus_core::{Color, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::container::Container;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The bar's height when nothing else is said.
const BAR_HEIGHT: f32 = 64.0;
/// The gap left between the notch and the button it receives.
const NOTCH_MARGIN: f32 = 4.0;
/// How far the notch's shoulders reach along the bar before the curve begins. Taken
/// from the reference, where it is the `s1` of the notch's derivation.
const SHOULDER: f32 = 15.0;
/// The reference's `s2`: how far the control points sit outside the guest circle.
const CLEARANCE: f32 = 1.0;

/// A bottom bar of actions, optionally notched for a docked floating action button.
pub struct BottomAppBar<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    color: Option<Color>,
    height: f32,
    padding: f32,
    notch_margin: f32,
    /// Where the notch goes: the button's centre `x` and its radius, in the bar's own
    /// coordinates. Set by the scaffold, never by the application.
    notch: Option<(f32, f32)>,
}

impl<Msg: Clone + 'static> Default for BottomAppBar<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> BottomAppBar<Msg> {
    /// An empty bar at the conventional height.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            color: None,
            height: BAR_HEIGHT,
            padding: 8.0,
            notch_margin: NOTCH_MARGIN,
            notch: None,
        }
    }

    /// What the bar carries — usually a row of buttons.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// The bar's colour. The theme's surface by default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The bar's height (64 px by default).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// The padding around its content.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// The gap left between the notch and the button it receives (4 px by default).
    pub fn notch_margin(mut self, margin: f32) -> Self {
        self.notch_margin = margin;
        self
    }

    /// Cuts the notch. **The scaffold's to call**, once it knows where it is putting
    /// the button: `centre_x` in the bar's own coordinates, `radius` the button's.
    pub(crate) fn notched_at(mut self, centre_x: f32, radius: f32) -> Self {
        self.notch = Some((centre_x, radius));
        self
    }

    /// The bar's declared height, which the scaffold needs to know where its top edge
    /// is before anything is laid out.
    pub(crate) fn declared_height(&self) -> f32 {
        self.height
    }
}

/// The bar's outline: its rectangle, with a circular notch cut into the top edge
/// around a circle of `radius` centred at `centre_x` on that edge.
///
/// The curve is the reference's: two quadratics joining the top edge to the circle,
/// and an arc between them, so the bar meets the button tangentially instead of at a
/// corner. `guest_dy` is how far the circle's centre sits below the top edge — zero
/// when the button is docked exactly astride it.
pub fn notched_outline(host: Rect, centre_x: f32, radius: f32, guest_dy: f32) -> Path {
    let (left, right) = (host.x, host.x + host.width);
    let (top, bottom) = (host.y, host.y + host.height);
    // A notch whose circle does not reach the bar is no notch at all.
    if radius <= 0.0 || guest_dy - radius >= host.height {
        return Path::rect(host);
    }

    // The reference's derivation, in the notch's own frame: the origin is the guest
    // circle's centre, `b` is how far the host's top edge is above it.
    let r = radius;
    let a = -r - CLEARANCE;
    let b = top - (top + guest_dy);
    let n2 = (b * b * r * r * (a * a + b * b - r * r)).max(0.0).sqrt();
    let denom = a * a + b * b;
    if denom.abs() < 1e-6 {
        return Path::rect(host);
    }
    let p2x_a = ((a * r * r) - n2) / denom;
    let p2x_b = ((a * r * r) + n2) / denom;
    let p2y_a = (r * r - p2x_a * p2x_a).max(0.0).sqrt();
    let p2y_b = (r * r - p2x_b * p2x_b).max(0.0).sqrt();
    let cmp = if b < 0.0 { -1.0 } else { 1.0 };
    let (p2x, p2y) = if cmp * p2y_a > cmp * p2y_b {
        (p2x_a, p2y_a)
    } else {
        (p2x_b, p2y_b)
    };

    // Back into the bar's coordinates. The circle's centre sits on the top edge,
    // pushed down by `guest_dy`.
    let cx = centre_x;
    let cy = top + guest_dy;
    let at = |x: f32, y: f32| Point::new(cx + x, cy + y);
    let p0 = at(a - SHOULDER, b);
    let p1 = at(a, b);
    let p2 = at(p2x, p2y);
    let p3 = at(-p2x, p2y);
    let p4 = at(-a, b);
    let p5 = at(-(a - SHOULDER), b);

    // The arc runs under the circle from p2 to p3, anticlockwise in screen terms.
    let start = (p2.y - cy).atan2(p2.x - cx);
    let end = (p3.y - cy).atan2(p3.x - cx);

    Path::new()
        .move_to(Point::new(left, top))
        .line_to(p0)
        .quad_to(p1, p2)
        .arc_to(Point::new(cx, cy), r, start, end)
        .quad_to(p4, p5)
        .line_to(Point::new(right, top))
        .line_to(Point::new(right, bottom))
        .line_to(Point::new(left, bottom))
        .close()
}

impl<Msg: Clone + 'static> Widget<Msg> for BottomAppBar<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Length(self.height),
            padding: frus_core::Insets::new(self.padding, self.padding, self.padding, self.padding),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, _status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self.color.unwrap_or(theme.surface);
        match self.notch {
            Some((centre_x, radius)) => {
                let path =
                    notched_outline(bounds, bounds.x + centre_x, radius + self.notch_margin, 0.0);
                scene.fill_path(&path, color);
            }
            None => scene.fill_rect(bounds, color),
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A spacer for a bar's row: takes what is left, so what follows goes to the far end.
pub fn bar_spacer<Msg: Clone + 'static>() -> Container<Msg> {
    Container::new().flex(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::PathVerb;

    const HOST: Rect = Rect {
        x: 0.0,
        y: 100.0,
        width: 400.0,
        height: 64.0,
    };

    /// Every node of the outline stays inside the bar, and the notch's own nodes keep
    /// clear of the circle they were cut for — which is the whole point of the shape.
    #[test]
    fn the_notch_clears_the_circle_it_was_cut_for() {
        let (cx, r) = (320.0, 28.0);
        let path = notched_outline(HOST, cx, r, 0.0);
        let centre = Point::new(cx, HOST.y);
        let mut nodes = 0;
        for verb in path.verbs() {
            let p = match verb {
                PathVerb::MoveTo(p) | PathVerb::LineTo(p) => *p,
                PathVerb::QuadTo { to, .. } | PathVerb::CubicTo { to, .. } => *to,
                PathVerb::Close => continue,
            };
            nodes += 1;
            assert!(
                p.y >= HOST.y - 0.01 && p.y <= HOST.y + HOST.height + 0.01,
                "node outside the bar: {p:?}"
            );
            let d = ((p.x - centre.x).powi(2) + (p.y - centre.y).powi(2)).sqrt();
            assert!(
                d >= r - 0.01,
                "node inside the button's circle: {p:?} ({d})"
            );
        }
        assert!(nodes > 6, "the notch adds curves, got {nodes} nodes");
    }

    /// A notch is symmetric about the button's centre.
    #[test]
    fn the_notch_is_centred_on_the_button() {
        let (cx, r) = (200.0, 28.0);
        let path = notched_outline(HOST, cx, r, 0.0);
        let on_top: Vec<f32> = path
            .verbs()
            .iter()
            .filter_map(|v| match v {
                PathVerb::LineTo(p) | PathVerb::QuadTo { to: p, .. }
                    if (p.y - HOST.y).abs() < 0.01 =>
                {
                    Some(p.x)
                }
                _ => None,
            })
            .collect();
        // The two shoulders, and the bar's far corner.
        let left = on_top.iter().cloned().fold(f32::MAX, f32::min);
        let right = on_top
            .iter()
            .cloned()
            .filter(|x| *x < HOST.width)
            .fold(f32::MIN, f32::max);
        assert!(
            ((left + right) / 2.0 - cx).abs() < 0.5,
            "shoulders at {left} and {right} are not centred on {cx}"
        );
    }

    /// The bar paints **behind** what it carries: its own surface goes down first,
    /// and the content on top of it. Painted the other way round, a notched bar would
    /// erase every button on it.
    #[test]
    fn the_bar_paints_behind_its_content() {
        use crate::{build_ui, Container, Runtime, Size, Theme};
        const MARK: Color = Color {
            r: 1.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let bar = BottomAppBar::<()>::new()
            .child(Container::new().width(80.0).height(30.0).color(MARK))
            .notched_at(300.0, 28.0);
        let ui = build_ui(
            &bar,
            Size::new(400.0, 64.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let order: Vec<&str> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Path { .. } => Some("bar"),
                frus_core::Primitive::Rect { color, .. } if *color == MARK => Some("content"),
                _ => None,
            })
            .collect();
        assert_eq!(order, vec!["bar", "content"], "the bar goes down first");
    }

    /// No notch asked for, or one that cannot reach the bar: a plain rectangle.
    #[test]
    fn without_a_notch_it_is_a_rectangle() {
        assert_eq!(notched_outline(HOST, 200.0, 0.0, 0.0), Path::rect(HOST));
        assert_eq!(notched_outline(HOST, 200.0, 10.0, 300.0), Path::rect(HOST));
    }
}
