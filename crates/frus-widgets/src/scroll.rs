//! [`SingleChildScrollView`]: a vertically scrollable container.
//!
//! It occupies a fixed-size viewport; its single child is laid out at free height by
//! the driver, then clipped to the viewport and translated by the scroll offset, which
//! the runtime retains, keyed by identity.

use frus_core::{Insets, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::physics::ScrollPhysics;
use crate::theme::Theme;
use crate::widget::Widget;

/// A [`SingleChildScrollView`]'s scrolling axis, or axes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
    /// Both, which is a **convenience** rather than a third kind of area: a scrollable
    /// elsewhere has one axis and one only, and two of them means one nested inside the
    /// other. This is that pair collapsed into a single node, and a gesture reads it as
    /// the pair: one drag moves one axis, decided by the direction the finger went in and
    /// held to the end of the gesture. A page that scrolls down does not drift sideways
    /// while it does it.
    Both,
}

impl Axis {
    pub(crate) fn free_x(self) -> bool {
        matches!(self, Axis::Horizontal | Axis::Both)
    }
    pub(crate) fn free_y(self) -> bool {
        matches!(self, Axis::Vertical | Axis::Both)
    }
}

/// A scrollable container.
pub struct SingleChildScrollView<Msg> {
    width: Dimension,
    height: Dimension,
    /// Was the width **set** explicitly? If not, in flex mode the width must not
    /// serve as a basis; see [`SingleChildScrollView::style`].
    width_explicit: bool,
    /// Was the height **set** explicitly? The same question for the vertical axis.
    height_explicit: bool,
    flex_grow: f32,
    axis: Axis,
    /// Edge and fling behaviour, when this area wants one of its own; `None`
    /// follows the application, which follows the platform.
    physics: Option<ScrollPhysics>,
    reverse: bool,
    /// Room around the content, inside the viewport; see [`SingleChildScrollView::padding`].
    padding: Insets,
    content: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> SingleChildScrollView<Msg> {
    /// Creates a scrollable area, its viewport 200 px tall by default.
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            width_explicit: false,
            height_explicit: false,
            flex_grow: 0.0,
            axis: Axis::Vertical,
            physics: None,
            reverse: false,
            padding: Insets::ZERO,
            content: Vec::new(),
        }
    }

    /// Chooses the scrolling axis or axes; vertical by default.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Scrolls **from the far end**: the content is anchored to the bottom of a
    /// vertical viewport (the right of a horizontal one) and the area starts there.
    ///
    /// It is what a conversation wants. Two things follow from it that nothing else
    /// gives you:
    ///
    /// - content **shorter** than the viewport sits at the bottom rather than the top;
    /// - the area **stays** at the end when content arrives, because offsets are
    ///   measured from the end. A view resting at the newest message goes on resting
    ///   there, however many messages are appended.
    ///
    /// Nothing changes for the user's hand: a finger pushes the content the way it
    /// moves, in either direction, and the scrollbar's thumb rests where the content is.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Overrides how this area behaves at its edges and after a fling.
    ///
    /// Left unset, it follows the application's choice, which itself defaults to
    /// what the running platform does — so setting this is for the cases where the
    /// content wants a particular feel, not for making an app feel native.
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    /// Sets the viewport width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self.width_explicit = true;
        self
    }

    /// Sets the viewport height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self.height_explicit = true;
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Insets the content, **inside** the viewport: the padding scrolls with what it
    /// surrounds rather than shrinking the window onto it.
    ///
    /// That is the whole distinction, and it is the reference's: a scroll area padded at
    /// the bottom has that room *at the end of its content*, reachable only by scrolling
    /// to it, which is what a floating button hovering over the last row needs. Room
    /// taken out of the viewport instead would sit there permanently and the last row
    /// would still slide under the button.
    ///
    /// Along the cross axis it simply insets the content, which stays the width of the
    /// viewport less the two sides.
    ///
    /// ```
    /// # use frus_widgets::SingleChildScrollView;
    /// # let feed = frus_widgets::Container::<()>::new();
    /// SingleChildScrollView::<()>::new().padding_each(0.0, 16.0, 88.0, 16.0).child(feed);
    /// ```
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// The same, one side at a time.
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// Sets the scrollable content.
    pub fn child(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.content.clear();
        self.content.push(Box::new(content));
        self
    }
}

impl<Msg> Default for SingleChildScrollView<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for SingleChildScrollView<Msg> {
    fn style(&self) -> Style {
        // In **flex** mode (fill then scroll), an axis dimension that was not set
        // explicitly must not serve as a basis: the default height of 200 would
        // otherwise stay a flexible basis and the viewport would not fill the parent's
        // remaining space — it would take `+200` of free room to grow. So we set it to
        // `Auto`, a basis of 0, and let `flex_grow` **fill**.
        let filling = self.flex_grow > 0.0;
        let height = if filling && !self.height_explicit && self.axis.free_y() {
            Dimension::Auto
        } else {
            self.height
        };
        let width = if filling && !self.width_explicit && self.axis.free_x() {
            Dimension::Auto
        } else {
            self.width
        };
        Style {
            width,
            height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // The viewport itself is transparent: only the content is drawn.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        self.content.first().map(|child| child.as_ref())
    }

    fn scroll_axis(&self) -> Axis {
        self.axis
    }

    fn scroll_physics(&self) -> Option<ScrollPhysics> {
        self.physics
    }

    fn scroll_reverse(&self) -> bool {
        self.reverse
    }

    fn scroll_padding(&self) -> Insets {
        self.padding
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    const MARK: Color = Color::rgb(1.0, 0.0, 0.0);

    /// A scroll 100 tall holding `content_h` of content, at `offset`; returns the
    /// rectangle of the marker drawn at the very end of that content.
    fn last_mark(scroll: SingleChildScrollView<()>, offset: f32) -> Rect {
        let mut runtime = Runtime::default();
        let ui = build_ui(
            &scroll,
            Size::new(100.0, 100.0),
            &runtime,
            &Theme::default(),
        );
        let id = ui.scroll_regions()[0].id;
        runtime.scroll.insert(id, (0.0, offset));
        let ui = build_ui(
            &scroll,
            Size::new(100.0, 100.0),
            &runtime,
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, rect, .. } if *color == MARK => Some(*rect),
                _ => None,
            })
            .next_back()
            .expect("the marker is painted")
    }

    /// A column of `n` 40 px rows, the last one marked.
    fn content(n: usize) -> Container<()> {
        let mut column = crate::Flex::<()>::column();
        for i in 0..n {
            let colour = if i + 1 == n {
                MARK
            } else {
                Color::rgb(0.0, 0.0, 1.0)
            };
            column = column.child(Container::<()>::new().height(40.0).color(colour));
        }
        Container::<()>::new().child(column)
    }

    /// Content shorter than the viewport sits at the **top** normally and at the
    /// **bottom** reversed — the first of the two things reversing is for.
    #[test]
    fn short_content_sits_at_the_end_when_reversed() {
        let plain = SingleChildScrollView::<()>::new()
            .width(100.0)
            .height(100.0)
            .child(content(1));
        assert_eq!(last_mark(plain, 0.0).y, 0.0, "at the top");

        let reversed = SingleChildScrollView::<()>::new()
            .width(100.0)
            .height(100.0)
            .reverse()
            .child(content(1));
        let r = last_mark(reversed, 0.0);
        assert_eq!(r.y + r.height, 100.0, "against the bottom: {r:?}");
    }

    /// Offset 0 is the **end** of the content when reversed, and it stays the end
    /// however much content there is — which is what keeps a conversation resting on
    /// its newest message as messages arrive.
    #[test]
    fn offset_zero_is_the_end_and_stays_there() {
        for rows in [4, 10, 40] {
            let reversed = SingleChildScrollView::<()>::new()
                .width(100.0)
                .height(100.0)
                .reverse()
                .child(content(rows));
            let r = last_mark(reversed, 0.0);
            assert_eq!(
                r.y + r.height,
                100.0,
                "{rows} rows, still resting at the end: {r:?}"
            );
        }
    }

    /// Scrolling away from zero walks **back** through the content, which is up the
    /// screen for the newest-last layout a reversed view has.
    #[test]
    fn a_positive_offset_moves_back_through_the_content() {
        let scroll = || {
            SingleChildScrollView::<()>::new()
                .width(100.0)
                .height(100.0)
                .reverse()
                .child(content(10))
        };
        let at_rest = last_mark(scroll(), 0.0);
        let scrolled = last_mark(scroll(), 60.0);
        assert_eq!(
            scrolled.y - at_rest.y,
            60.0,
            "the end has moved down and out of view"
        );
    }

    /// The end of the content is the end of the *axis*, so a refusal there glows at the
    /// bottom of a reversed area rather than the top — where a conversation's oldest
    /// message is, and where the user is actually pressing.
    #[test]
    fn the_glow_follows_the_axis_and_not_the_screen() {
        use frus_core::Rect as R;
        let area = |reverse: bool| crate::Scrollable {
            id: crate::WidgetId::ROOT,
            viewport: R::new(0.0, 0.0, 100.0, 100.0),
            max_x: 0.0,
            max_y: 300.0,
            physics: None,
            refresh: None,
            page: None,
            reverse_x: false,
            reverse_y: reverse,
            host: None,
            keep_visible: None,
        };
        // A refusal towards the axis's start (a negative offset).
        assert_eq!(area(false).refused_edge(true, -1.0), crate::GlowEdge::Top);
        assert_eq!(
            area(true).refused_edge(true, -1.0),
            crate::GlowEdge::Bottom,
            "the start of a reversed axis is the bottom of the screen"
        );
        // And a pull-to-refresh listens at whichever of the two that is.
        assert_eq!(area(true).start_edge(true), crate::GlowEdge::Bottom);
    }

    /// The one thing that is *not* reversed: what the user's hand does. A push in a
    /// direction moves the content that way in both, and only the arithmetic between
    /// the push and the number changes sign.
    #[test]
    fn a_push_moves_the_content_the_way_it_pushes() {
        let region = |reverse: bool| {
            let mut s = SingleChildScrollView::<()>::new()
                .width(100.0)
                .height(100.0);
            if reverse {
                s = s.reverse();
            }
            let s = s.child(content(10));
            let ui = build_ui(
                &s,
                Size::new(100.0, 100.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scroll_regions()[0]
        };
        // A finger pushing the content **down** by 10 px.
        assert_eq!(region(false).offset_delta((0.0, 10.0)).1, -10.0);
        assert_eq!(region(true).offset_delta((0.0, 10.0)).1, 10.0);
        // Opposite numbers, and in both cases "the content went down": one counts from
        // the top and the other from the bottom.
    }
}

#[cfg(test)]
mod padding_tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    const MARK: Color = Color::rgb(1.0, 0.0, 0.0);

    /// The box the content painted.
    fn content_rect(scroll: &SingleChildScrollView<()>) -> Rect {
        build_ui(
            scroll,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .iter()
        .find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if *color == MARK => Some(*rect),
            _ => None,
        })
        .expect("the content is painted")
    }

    fn area(height: f32) -> SingleChildScrollView<()> {
        SingleChildScrollView::<()>::new()
            .width(100.0)
            .height(100.0)
            .child(Container::<()>::new().height(height).color(MARK))
    }

    /// The leading insets move the content in; the sides come off its width, which is
    /// otherwise the viewport's.
    #[test]
    fn the_content_starts_inside_the_insets() {
        let bare = content_rect(&area(50.0));
        assert_eq!((bare.x, bare.y, bare.width), (0.0, 0.0, 100.0));

        let padded = content_rect(&area(50.0).padding_each(12.0, 8.0, 24.0, 8.0));
        assert_eq!((padded.x, padded.y), (8.0, 12.0));
        assert_eq!(padded.width, 84.0, "the sides come off the content");
    }

    /// The room is inside the viewport and scrolls with the content: it joins what there
    /// is to scroll rather than being taken out of the window.
    #[test]
    fn the_far_inset_is_reachable_rather_than_lost() {
        let region = |scroll: &SingleChildScrollView<()>| {
            build_ui(
                scroll,
                Size::new(100.0, 100.0),
                &Runtime::default(),
                &Theme::default(),
            )
            .scroll_regions()[0]
                .max_y
        };
        assert_eq!(
            region(&area(100.0)),
            0.0,
            "exactly fills: nothing to scroll"
        );
        assert_eq!(
            region(&area(100.0).padding_each(12.0, 0.0, 24.0, 0.0)),
            36.0,
            "both insets joined the content"
        );
    }

    /// A reversed area rests at the end, and the end is one bottom inset up.
    #[test]
    fn a_reversed_area_rests_above_its_bottom_inset() {
        let padded = content_rect(&area(50.0).reverse().padding_each(12.0, 0.0, 24.0, 0.0));
        assert_eq!(
            padded.y + padded.height,
            76.0,
            "the content ends one bottom inset above the viewport's edge"
        );
    }
}
