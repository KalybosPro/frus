//! [`ListWheel`]: the scrolling cylinder of values you spin to pick one.
//!
//! The selected row is in the middle and the rest curve away above and below, packing
//! together and shrinking as they go. It is the control behind every date, time and
//! duration picker that is **not** a calendar — and the one an application reaches for
//! when it needs a picker of its own: a quantity, a font size, a unit.
//!
//! ## What a cylinder is, and what this one is
//!
//! A true cylinder needs a rotation about a **horizontal** axis and a perspective divide.
//! The paint here transforms a subtree by an [`frus_core::Affine`], and an affine maps
//! parallel lines to parallel lines — so it cannot produce the trapezoid a tipped row
//! actually is, its near edge wider than its far one. **A real cylinder is not
//! expressible**, and saying it is would be the sort of claim that is found out on a
//! screen rather than in a test.
//!
//! What is expressible is every part of a cylinder that a reader notices:
//!
//! - rows **pack together** towards the ends, their centres at `r·sin θ` rather than at
//!   the flat distance the strip would put them;
//! - rows **squash** vertically by `cos θ`, which is foreshortening exactly;
//! - rows **narrow** slightly as they recede, which is the perspective the affine can
//!   still express — a scale, being uniform across the row, rather than a taper;
//! - rows **fade** with the same `cos θ`, which is where the missing taper's share of the
//!   illusion goes.
//!
//! What is left out is the taper itself, and at the angles a wheel is read at it is the
//! one part nobody looks for. Anything past a **quarter turn** has gone over the horizon
//! and is not drawn at all.
//!
//! ## What it is made of
//!
//! Nothing new: the spin, the fling and the settling-on-a-row are the paged scrollable's
//! ([`crate::PageView`]), with each row a page of its own height. So a wheel inherits the
//! release that springs to the nearest row rather than stopping between two, the
//! virtualised window — a wheel of three thousand minutes costs what a wheel of ten does
//! — and the requested-row-on-the-first-frame rule. What a wheel adds is the geometry
//! below and a way of saying, to a reader who cannot see it, which value is chosen.

use std::f32::consts::FRAC_PI_2;

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::pageview::PagedView;
use crate::physics::ScrollPhysics;
use crate::scroll::Axis;
use crate::theme::Theme;
use crate::widget::Widget;

/// The shape of the cylinder rows are laid on.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WheelGeometry {
    /// The cylinder's diameter as a multiple of the viewport's height. **Larger is
    /// flatter**: at a diameter of twice the viewport only the middle thirty degrees of
    /// the cylinder are ever in front of it, and thirty degrees of a turn is a slope
    /// rather than a wheel.
    pub diameter_ratio: f32,
    /// How much a row narrows per pixel it has receded. `0.0` leaves every row the full
    /// width, which reads as a fold rather than a turn.
    pub perspective: f32,
}

impl Default for WheelGeometry {
    /// A diameter only a little larger than the wheel is tall, which puts the ends of
    /// the visible strip most of the way round to the horizon and is what makes a picker
    /// look like a picker rather than like a list with the ends dimmed.
    ///
    /// These are the reference's own numbers for its picker, and its comment on them is
    /// that they were **eyeballed against a real one**. That is worth repeating rather
    /// than rederiving: the geometry here is deliberately not a cylinder (see the module
    /// documentation), so a figure argued from first principles would be arguing about
    /// the wrong shape.
    fn default() -> Self {
        Self {
            diameter_ratio: 1.07,
            perspective: 0.003,
        }
    }
}

/// Where one row has ended up on the cylinder, and what it looks like there.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RowOnWheel {
    /// How far round the cylinder the row has turned, in radians, signed.
    pub angle: f32,
    /// Where the row's **centre** sits, in pixels from the viewport's centre — always
    /// nearer to it than the flat strip would have put it.
    pub offset: f32,
    /// Horizontal scale: the row receding.
    pub scale_x: f32,
    /// Vertical scale: the row foreshortened, which is `cos θ`.
    pub scale_y: f32,
    /// What is left of the row's opacity.
    pub opacity: f32,
}

impl WheelGeometry {
    /// The cylinder's radius in a viewport `extent` pixels along.
    pub fn radius(&self, extent: f32) -> f32 {
        (extent * self.diameter_ratio / 2.0).max(1.0)
    }

    /// Where a row goes, given the viewport's extent and where the row's centre would
    /// sit on an **unrolled** strip, measured from the viewport's centre.
    ///
    /// `None` once the row has turned past a quarter — it has gone over the horizon, and
    /// a row drawn there would be a line of text lying edge-on.
    pub fn row(&self, extent: f32, flat: f32) -> Option<RowOnWheel> {
        let radius = self.radius(extent);
        // Arc length over radius: the strip is what is wrapped round the cylinder, so the
        // flat distance **is** the arc, and this is the angle it subtends.
        let angle = flat / radius;
        if angle.abs() >= FRAC_PI_2 {
            return None;
        }
        let cos = angle.cos();
        // How far the row has receded from the plane of the viewport.
        let z = radius * (1.0 - cos);
        Some(RowOnWheel {
            angle,
            offset: radius * angle.sin(),
            scale_x: 1.0 / (1.0 + self.perspective * z),
            scale_y: cos,
            opacity: cos,
        })
    }
}

/// A scrolling cylinder of rows, resting on one of them. See the module documentation.
///
/// ```
/// use frus_widgets::{text, ListWheel};
///
/// let _minutes: ListWheel<usize> = ListWheel::new(60, 34.0, |i| text(format!("{i:02}")))
///     .height(180.0)
///     .selected(15)
///     .on_selected(|i| i)
///     .label(|i| format!("{i} minutes"));
/// ```
pub struct ListWheel<Msg> {
    count: usize,
    extent: f32,
    geometry: WheelGeometry,
    selected: usize,
    build: Box<dyn Fn(usize) -> Box<dyn Widget<Msg>>>,
    on_selected: Option<Box<dyn Fn(usize) -> Msg>>,
    label: Option<Box<dyn Fn(usize) -> String>>,
    physics: ScrollPhysics,
    width: Dimension,
    height: Dimension,
    flex: f32,
}

impl<Msg> ListWheel<Msg> {
    /// A wheel of `count` rows, each `extent` pixels tall, built on demand.
    pub fn new<W: Widget<Msg> + 'static>(
        count: usize,
        extent: f32,
        build: impl Fn(usize) -> W + 'static,
    ) -> Self {
        Self {
            count,
            extent: extent.max(1.0),
            geometry: WheelGeometry::default(),
            selected: 0,
            build: Box::new(move |index| Box::new(build(index)) as Box<dyn Widget<Msg>>),
            on_selected: None,
            label: None,
            physics: ScrollPhysics::default(),
            width: Dimension::Auto,
            height: Dimension::Length(180.0),
            flex: 0.0,
        }
    }

    /// The row the application says is chosen. Applied on the first frame and on every
    /// change, the way [`crate::PageView::page`] is — so a wheel opens on the value it
    /// holds rather than on the first one and then jumping.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// What to send when the wheel comes to rest on a different row.
    pub fn on_selected(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_selected = Some(Box::new(message));
        self
    }

    /// **What each row is called**, for a reader who cannot see it.
    ///
    /// Not optional in spirit: a wheel with no words is a control that announces its
    /// position as a number out of a count and its value as nothing at all. The index is
    /// what the caller has; only the caller knows it means half past three.
    pub fn label(mut self, label: impl Fn(usize) -> String + 'static) -> Self {
        self.label = Some(Box::new(label));
        self
    }

    /// How curved the cylinder is: the diameter as a multiple of the wheel's own height.
    pub fn diameter_ratio(mut self, ratio: f32) -> Self {
        self.geometry.diameter_ratio = ratio.max(0.1);
        self
    }

    /// How much a receding row narrows. `0.0` for none.
    pub fn perspective(mut self, perspective: f32) -> Self {
        self.geometry.perspective = perspective.max(0.0);
        self
    }

    /// The scroll physics of the spin — the platform's by default.
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = physics;
        self
    }

    /// A fixed width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// A fixed height. A wheel needs one: it is a window onto a cylinder, and a window
    /// that hugged its content would be the whole strip.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Grows to fill the space its parent has left along the main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex = grow;
        self
    }

    /// The row a reader is on, clamped to what there is.
    fn chosen(&self) -> usize {
        self.selected.min(self.count.saturating_sub(1))
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ListWheel<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // The rows are the picture, and the walk draws them.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn page_view(&self) -> Option<PagedView<'_, Msg>> {
        Some(PagedView {
            count: self.count,
            axis: Axis::Vertical,
            viewport_fraction: 1.0,
            extent: Some(self.extent),
            requested: self.chosen(),
            // The first row rests **centred**, not against the top edge: the middle of a
            // wheel is where the answer is, so that is where row nought has to be able to
            // get to. This is also what makes the last row reachable at all.
            pad_ends: true,
            snapping: true,
            wheel: Some(self.geometry),
            build: &*self.build,
        })
    }

    fn on_page_changed(&self, index: usize) -> Option<Msg> {
        self.on_selected.as_ref().map(|f| f(index))
    }

    fn scroll_physics(&self) -> Option<ScrollPhysics> {
        Some(self.physics)
    }

    /// **The wheel is the control, and this is how a reader hears it change.**
    ///
    /// Not a list to walk row by row: a wheel is a one-dimensional selector, so it says
    /// so — the role a platform's own picker reports, the chosen value in the caller's
    /// words, and the position in the set as the range. A reader gets "half past three,
    /// 15 of 60", and gets it again, by itself, every time the wheel settles somewhere
    /// else. Rows carry their own annotations underneath; this is the one that speaks
    /// for the whole.
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let chosen = self.chosen();
        let mut props = frus_core::SemanticsProperties::new(frus_core::Role::Slider).range(
            0.0,
            chosen as f32,
            self.count.saturating_sub(1) as f32,
        );
        props.value = Some(match self.label.as_ref() {
            Some(label) => label(chosen),
            // Without words there is still a position, and a position is better than
            // silence — but it is not an answer to "what is this set to".
            None => format!("{} of {}", chosen + 1, self.count),
        });
        Some(props)
    }

    fn debug_name(&self) -> &'static str {
        "ListWheel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::ScrollPhysics;
    use crate::{build_ui, Container, Runtime};
    use frus_core::{Color, Primitive, Size};
    use std::cell::Cell;
    use std::rc::Rc;

    const EXTENT: f32 = 34.0;
    const VIEWPORT: f32 = 200.0;

    fn wheel(count: usize, selected: usize, built: Rc<Cell<usize>>) -> ListWheel<()> {
        ListWheel::new(count, EXTENT, move |index| {
            built.set(built.get() + 1);
            Container::new()
                .width(160.0)
                .height(EXTENT)
                .color(Color::rgb(index as f32 / 60.0, 0.0, 0.0))
        })
        .width(160.0)
        .height(VIEWPORT)
        .selected(selected)
    }

    fn ui_of(widget: &ListWheel<()>, runtime: &Runtime) -> crate::Ui<()> {
        build_ui(
            widget,
            Size::new(160.0, VIEWPORT),
            runtime,
            &Theme::default(),
        )
    }

    /// **The middle of the wheel is where the answer is**, so the row the application
    /// says is chosen has to be able to get there — and has to be there on the first
    /// frame rather than after one that showed row nought.
    #[test]
    fn the_chosen_row_rests_across_the_middle() {
        let wheel = wheel(60, 30, Rc::new(Cell::new(0)));
        let runtime = Runtime::default();
        let ui = ui_of(&wheel, &runtime);
        let area = ui.scroll_regions().first().copied().expect("a region");
        let snap = area.page.expect("a wheel is a paged region");
        assert_eq!(snap.extent, EXTENT, "one row is one page");
        assert_eq!(snap.count, 60);
        assert!(!snap.horizontal, "a wheel turns the short way");
        // The offset a wheel opens at, and the row it puts across the middle.
        let along = snap.offset_of(30);
        assert_eq!(snap.page_at(along), 30);
        // Both ends are padded by half a viewport less half a row, which is what lets
        // row nought and row fifty-nine reach the middle at all.
        let pad = (VIEWPORT - EXTENT) / 2.0;
        assert!(
            (area.max_y - (60.0 * EXTENT + 2.0 * pad - VIEWPORT)).abs() < 0.01,
            "the travel is exactly the last row's own offset: {}",
            area.max_y
        );
        assert!(
            (area.max_y - snap.offset_of(59)).abs() < 0.01,
            "which is to say the last row can be reached: {}",
            area.max_y
        );
    }

    /// **A release settles on a row, never between two.** The wheel does not invent this
    /// — it is the paged scrollable's release — but a wheel that failed to declare
    /// itself paged would fling like a list and stop wherever it ran out, which is the
    /// one thing a picker must not do.
    #[test]
    fn a_spin_comes_to_rest_on_a_row() {
        let wheel = wheel(60, 30, Rc::new(Cell::new(0)));
        let mut runtime = Runtime::default();
        let area = ui_of(&wheel, &runtime)
            .scroll_regions()
            .first()
            .copied()
            .expect("a region");
        let snap = area.page.expect("paged");
        // Let go from exactly halfway between two rows, so nothing but the direction of
        // the release can decide which one it goes to — and ask it both ways, because a
        // wheel that settled on a row and always the same row would pass the first half
        // of this on its own.
        let between = snap.offset_of(30) + EXTENT / 2.0;
        let settle = |runtime: &mut Runtime, velocity: f32| {
            runtime.scroll.insert(area.id, (0.0, between));
            runtime.scroll_ballistic.remove(&area.id);
            assert!(
                runtime.fling_scroll(area, ScrollPhysics::default(), (0.0, velocity)),
                "a release on a paged region is a spring to a row"
            );
            for _ in 0..600 {
                if !runtime.advance_scroll(&[area], ScrollPhysics::default(), 1.0 / 60.0) {
                    break;
                }
            }
            runtime.scroll.get(&area.id).copied().unwrap_or_default().1
        };

        let onwards = settle(&mut runtime, 300.0);
        let row = (onwards / EXTENT).round();
        assert!(
            (onwards - row * EXTENT).abs() < 0.5,
            "settled on a row and not between two: {onwards} is {} rows",
            onwards / EXTENT
        );
        assert_eq!(
            snap.page_at(onwards),
            31,
            "and on the one it was sent towards"
        );

        let back = settle(&mut runtime, -300.0);
        assert!(
            (back - (back / EXTENT).round() * EXTENT).abs() < 0.5,
            "and the other way is a row too: {back}"
        );
        assert_eq!(snap.page_at(back), 30, "the one behind it");
    }

    /// **Only the rows the cylinder shows are built.** A wheel of three thousand minutes
    /// is normal, and one that built three thousand rows a frame would be a picker
    /// nobody could open.
    ///
    /// It is more than a flat strip would show, and that is the point: rows a quarter
    /// turn away are compressed into the last pixels at either end, so their flat
    /// positions are well outside the viewport and the window has to be widened to
    /// reach them. What it must not do is widen without limit.
    #[test]
    fn a_wheel_of_three_thousand_builds_a_dozen() {
        let built = Rc::new(Cell::new(0));
        let wheel = wheel(3000, 1500, built.clone());
        let runtime = Runtime::default();
        ui_of(&wheel, &runtime);
        let flat = (VIEWPORT / EXTENT).ceil() as usize + 1;
        assert!(
            built.get() > flat,
            "a wheel shows more rows than a flat strip: {} vs {flat}",
            built.get()
        );
        assert!(
            built.get() < 3 * flat,
            "and not without limit: {} of three thousand",
            built.get()
        );
    }

    /// **The rows are not where a flat strip would put them.** The one thing that says
    /// the geometry reached the paint rather than merely being computed: the rows either
    /// side of the middle are drawn closer in than their own height, and shorter.
    #[test]
    fn the_paint_puts_the_rows_on_the_cylinder() {
        let wheel = wheel(60, 30, Rc::new(Cell::new(0)));
        let runtime = Runtime::default();
        let ui = ui_of(&wheel, &runtime);
        // Every row's painted band, by the red channel the builder above numbered them
        // with. Layers are transformed at compositing time, so the rects inside carry
        // the flat geometry — what is asked here is the transform the layer holds.
        let layers: Vec<_> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Layer {
                    transform: Some(t), ..
                } => Some(t.affine),
                _ => None,
            })
            .collect();
        assert!(
            layers.len() >= 5,
            "one layer per row on the cylinder: {}",
            layers.len()
        );
        // `m` is `[a, b, c, d, e, f]`: `a` and `d` are the two scales. The middle row is
        // the untouched one — at the centre the cylinder is tangent to the screen, so
        // its transform is the identity to within rounding.
        let flat = layers
            .iter()
            .filter(|m| (m.m[0] - 1.0).abs() < 1e-3 && (m.m[3] - 1.0).abs() < 1e-3)
            .count();
        assert_eq!(flat, 1, "exactly one row is face on");
        // And nothing is stretched: every row is squashed or unchanged, never taller.
        assert!(
            layers.iter().all(|m| m.m[3] <= 1.0 + 1e-3),
            "a cylinder turns rows away from the reader, never towards"
        );
    }

    /// **A reader hears which value is chosen, in the caller's words.**
    ///
    /// A wheel is a one-dimensional selector rather than a list to walk row by row, so
    /// it says so as one: the role a platform's own picker reports, the value spelled
    /// out, and the position in the set as the range. That is also what makes the
    /// change announced — it is the same path a slider's value travels.
    #[test]
    fn the_wheel_says_what_it_is_set_to() {
        let wheel = wheel(60, 30, Rc::new(Cell::new(0))).label(|i| format!("{i} minutes"));
        let runtime = Runtime::default();
        let ui = ui_of(&wheel, &runtime);
        let (_, _, props) = ui
            .semantics()
            .iter()
            .find(|(_, _, s)| s.role == frus_core::Role::Slider)
            .expect("the wheel announces itself as a selector");
        assert_eq!(props.value.as_deref(), Some("30 minutes"));
        assert_eq!(props.range, Some((0.0, 30.0, 59.0)), "30 of 60");

        // And moving it says something else, which is the whole of "announced as it
        // changes": the value a reader is given is a function of where the wheel is.
        let moved = wheel_at(31);
        let ui = ui_of(&moved, &Runtime::default());
        let (_, _, props) = ui
            .semantics()
            .iter()
            .find(|(_, _, s)| s.role == frus_core::Role::Slider)
            .expect("still a selector");
        assert_eq!(props.value.as_deref(), Some("31 minutes"));
    }

    /// A wheel with words on it, at a given row.
    fn wheel_at(selected: usize) -> ListWheel<()> {
        wheel(60, selected, Rc::new(Cell::new(0))).label(|i| format!("{i} minutes"))
    }

    /// Without words, a position is still better than silence — and is still not an
    /// answer to "what is this set to". Said out loud so that nobody reads the fallback
    /// as good enough.
    #[test]
    fn a_wheel_with_no_words_gives_back_a_position() {
        let wheel = wheel(60, 30, Rc::new(Cell::new(0)));
        let ui = ui_of(&wheel, &Runtime::default());
        let (_, _, props) = ui
            .semantics()
            .iter()
            .find(|(_, _, s)| s.role == frus_core::Role::Slider)
            .expect("a selector");
        assert_eq!(props.value.as_deref(), Some("31 of 60"));
    }

    /// The geometry itself, at the three points worth pinning: the middle, a turn, and
    /// the horizon.
    #[test]
    fn the_cylinder_packs_shrinks_and_ends() {
        let g = WheelGeometry::default();
        let radius = g.radius(VIEWPORT);

        // Face on: nothing happens at all, which is what makes the chosen row the one
        // that is drawn as it was written.
        let middle = g.row(VIEWPORT, 0.0).expect("the middle is on the cylinder");
        assert_eq!(middle.angle, 0.0);
        assert_eq!(middle.offset, 0.0);
        assert!((middle.scale_x - 1.0).abs() < 1e-6);
        assert!((middle.scale_y - 1.0).abs() < 1e-6);
        assert!((middle.opacity - 1.0).abs() < 1e-6);

        // A row a third of a radian round: nearer the middle than the flat strip put it,
        // shorter, narrower and fainter — each of those in the same direction.
        let flat = radius / 3.0;
        let turned = g.row(VIEWPORT, flat).expect("still in front");
        assert!((turned.angle - 1.0 / 3.0).abs() < 1e-5);
        assert!(
            turned.offset < flat && turned.offset > 0.0,
            "packed towards the middle: {} of {flat}",
            turned.offset
        );
        assert!(turned.scale_y < 1.0 && turned.scale_y > 0.9);
        assert!(turned.scale_x < 1.0);
        assert!(turned.opacity < 1.0);

        // Symmetric, because a wheel has no top and bottom of its own.
        let mirrored = g.row(VIEWPORT, -flat).expect("still in front");
        assert!((mirrored.offset + turned.offset).abs() < 1e-4);
        assert!((mirrored.scale_y - turned.scale_y).abs() < 1e-6);

        // A quarter turn is the horizon: exactly there and beyond it, a row is a line
        // seen edge-on and is not drawn.
        let horizon = radius * std::f32::consts::FRAC_PI_2;
        assert!(g.row(VIEWPORT, horizon).is_none());
        assert!(g.row(VIEWPORT, horizon * 1.5).is_none());
        assert!(g.row(VIEWPORT, horizon * 0.99).is_some());
    }

    /// **A flatter wheel is a bigger cylinder**, and the numbers have to move that way
    /// round: it is the one parameter a caller will reach for, and one whose sign is
    /// backwards is worse than one that does not exist.
    #[test]
    fn a_larger_diameter_is_a_flatter_wheel() {
        let flat_at = |ratio: f32| {
            WheelGeometry {
                diameter_ratio: ratio,
                ..Default::default()
            }
            .row(VIEWPORT, 60.0)
            .expect("in front")
        };
        let curved = flat_at(1.07);
        let flatter = flat_at(6.0);
        assert!(
            flatter.angle < curved.angle,
            "the same row has turned less: {} vs {}",
            flatter.angle,
            curved.angle
        );
        assert!(
            flatter.scale_y > curved.scale_y,
            "so it is squashed less: {} vs {}",
            flatter.scale_y,
            curved.scale_y
        );
        assert!(
            flatter.offset > curved.offset,
            "and packed towards the middle less: {} vs {}",
            flatter.offset,
            curved.offset
        );
    }
}
