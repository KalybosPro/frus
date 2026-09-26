//! [`Aligned`] and [`Center`]: a child placed somewhere in the room it is given.

use frus_core::{AlignmentGeometry, Rect, Scene};
use frus_layout::Style;

use crate::container::Container;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{FillAxes, Widget};

/// **Places its child** at `alignment` in the room it is given, at the child's own size.
///
/// The box takes all the room on offer, and the child sits at the anchor inside it: centred, in
/// a corner, against an edge, or at any fraction between. The anchor can be
/// [`Alignment`](frus_core::Alignment) or
/// [`AlignmentDirectional`](frus_core::AlignmentDirectional), which mirrors in a right-to-left
/// script.
///
/// ```
/// use frus_core::Alignment;
/// use frus_widgets::{Aligned, Text};
///
/// let _signature: Aligned<()> = Aligned::new(Alignment::BOTTOM_RIGHT, Text::new("— the team"));
/// ```
///
/// **How much room it takes.** All of what its parent hands down: a whole page, a whole card,
/// the whole cell of a grid. The exception is along a row's or a column's own direction when the
/// line holds other children. The line is shared there, so the box is only as long as its child,
/// and the child is centred across the line only. Wrap it in
/// [`Expanded`](crate::Expanded) to give it the rest of the line.
///
/// To animate a change of anchor, use [`AnimatedAlign`](crate::AnimatedAlign).
///
/// (Named for what it does to its child, as [`Positioned`](crate::Positioned) and
/// [`Expanded`](crate::Expanded) are: `Align` is the cross-axis alignment of a row or a column.)
pub struct Aligned<Msg = crate::callback::Callback> {
    inner: Container<Msg>,
}

impl<Msg: Clone + 'static> Aligned<Msg> {
    /// Places `child` at `alignment`.
    pub fn new(alignment: impl Into<AlignmentGeometry>, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Container::new().alignment(alignment).child(child),
        }
    }
}

/// **Centres its child** in the room it is given — an [`Aligned`] at
/// [`Alignment::CENTER`](frus_core::Alignment::CENTER), and the same room.
///
/// ```
/// use frus_widgets::{Center, Text};
///
/// let _empty: Center<()> = Center::new(Text::new("Nothing here yet"));
/// ```
pub struct Center<Msg = crate::callback::Callback> {
    inner: Aligned<Msg>,
}

impl<Msg: Clone + 'static> Center<Msg> {
    /// Centres `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Aligned::new(frus_core::Alignment::CENTER, child),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Aligned<Msg> {
    fn style(&self) -> Style {
        self.inner.style()
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.inner.style_themed(theme)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.inner.children()
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure layout widget: it draws nothing of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn alignment_geometry(&self) -> Option<AlignmentGeometry> {
        self.inner.alignment_geometry()
    }

    /// All the room on offer, on both axes. The layout gives a box that asks for this the
    /// reference's answer: the whole of a bounded axis, and the child's length along a line it
    /// shares with others.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes::BOTH
    }

    fn debug_name(&self) -> &'static str {
        "Aligned"
    }
}

impl<Msg: Clone> Widget<Msg> for Center<Msg> {
    fn style(&self) -> Style {
        self.inner.style()
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.inner.style_themed(theme)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.inner.children()
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn alignment_geometry(&self) -> Option<AlignmentGeometry> {
        self.inner.alignment_geometry()
    }

    fn fill_axes(&self, theme: &Theme) -> FillAxes {
        self.inner.fill_axes(theme)
    }

    fn debug_name(&self) -> &'static str {
        "Center"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Expanded, Flex, Runtime, Text};
    use frus_core::{Alignment, Color, Primitive, Size};

    const RED: Color = Color::rgb(1.0, 0.0, 0.0);

    /// A 20×20 red square.
    fn square() -> Container<()> {
        Container::new().width(20.0).height(20.0).color(RED)
    }

    /// Where the red square landed.
    fn red(root: &dyn Widget<()>, size: Size) -> Rect {
        let ui = build_ui(root, size, &Runtime::default(), &Theme::dark());
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == RED => Some(*rect),
                _ => None,
            })
            .expect("the red square")
    }

    fn near(rect: Rect, x: f32, y: f32) -> bool {
        (rect.x - x).abs() < 1.0 && (rect.y - y).abs() < 1.0
    }

    /// **Alone on a page, it centres on both axes**: the whole page is the room it is given,
    /// without a size said anywhere.
    #[test]
    fn a_center_alone_centres_on_the_page() {
        let at = red(&Center::new(square()), Size::new(200.0, 100.0));
        assert!(near(at, 90.0, 40.0), "{at:?}");
    }

    /// **Inside a box with a size**, the same: the room is the box's.
    #[test]
    fn a_center_in_a_box_centres_in_the_box() {
        let root = Container::<()>::new()
            .width(100.0)
            .height(60.0)
            .child(Center::new(square()));
        let at = red(&root, Size::new(300.0, 300.0));
        assert!(near(at, 40.0, 20.0), "{at:?}");
    }

    /// **In a column beside other children**, the column's own direction is shared: the centre
    /// takes its child's height and sits after the text, centred across the column only. Wrapped
    /// in `Expanded`, it takes the rest of the column and centres in that.
    #[test]
    fn in_a_column_it_shares_the_line_unless_expanded() {
        let size = Size::new(200.0, 200.0);
        let column = Flex::<()>::column()
            .child(Text::new("title").no_wrap())
            .child(Center::new(square()));
        let at = red(&column, size);
        assert!((at.x - 90.0).abs() < 1.0, "centred across: {at:?}");
        assert!(
            at.y < 40.0,
            "just below the title, not in the middle: {at:?}"
        );

        let column = Flex::<()>::column()
            .height(200.0)
            .child(Text::new("title").no_wrap())
            .child(Expanded::new(Center::new(square())));
        let at = red(&column, size);
        assert!((at.x - 90.0).abs() < 1.0, "{at:?}");
        assert!(at.y > 90.0, "centred in what the title leaves: {at:?}");
    }

    /// **In a row beside other children**, the other way round: centred down the row's height,
    /// and as wide as its child, after the label.
    #[test]
    fn in_a_row_it_centres_across_the_row() {
        let row = Flex::<()>::row()
            .height(100.0)
            .child(Text::new("label").no_wrap())
            .child(Center::new(square()));
        let at = red(&row, Size::new(300.0, 100.0));
        assert!((at.y - 40.0).abs() < 1.0, "centred down the row: {at:?}");
        assert!(at.x < 80.0, "just after the label: {at:?}");
    }

    /// **The body of a page**: the commonest place for it — an empty state, a spinner — centres
    /// in what the bar leaves.
    #[test]
    fn a_center_as_a_page_body_centres_under_the_bar() {
        let size = Size::new(200.0, 400.0);
        let page = crate::MediaQuery::new(size).scope(|| {
            crate::Scaffold::<()>::new()
                .app_bar(
                    crate::AppBar::<()>::new()
                        .title(Text::new("Inbox"))
                        .height(56.0)
                        .build(),
                )
                .body(Center::new(square()))
                .build()
        });
        let at = red(page.as_ref(), size);
        assert!((at.x - 90.0).abs() < 1.0, "{at:?}");
        assert!(
            at.y > 190.0 && at.y < 230.0,
            "centred below the bar: {at:?}"
        );
    }

    /// **In a scroll**, which has no height to hand down, it is as tall as its child — the
    /// reference's rule for an axis with no bound — and still centred across.
    #[test]
    fn in_a_scroll_it_is_as_tall_as_its_child() {
        let scroll = crate::SingleChildScrollView::<()>::new()
            .width(200.0)
            .height(300.0)
            .child(Center::new(square()));
        let at = red(&scroll, Size::new(200.0, 300.0));
        assert!((at.x - 90.0).abs() < 1.0, "{at:?}");
        assert!(at.y < 5.0, "at the top: {at:?}");
    }

    /// **Any anchor**, physical or directional: `Aligned` in a corner, and at a fraction.
    #[test]
    fn aligned_places_at_any_anchor() {
        let size = Size::new(100.0, 100.0);
        let at = red(&Aligned::new(Alignment::BOTTOM_RIGHT, square()), size);
        assert!(near(at, 80.0, 80.0), "{at:?}");
        let at = red(&Aligned::new(Alignment::new(0.5, -0.5), square()), size);
        assert!(near(at, 60.0, 20.0), "{at:?}");
        let at = red(
            &Aligned::new(frus_core::AlignmentDirectional::CENTER_START, square()),
            size,
        );
        assert!(near(at, 0.0, 40.0), "{at:?}");
    }
}
