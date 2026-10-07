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
/// **A size of its own.** [`Aligned::width_factor`] and [`Aligned::height_factor`] make the
/// box a multiple of its child on that axis instead: `2.0` is twice as tall as the child, with
/// the child at the anchor inside; `0.5` is half as tall, and the child spills past the box,
/// which does not cut it. On an axis with no factor, the box still takes the room.
///
/// To animate a change of anchor, use [`AnimatedAlign`](crate::AnimatedAlign).
///
/// (Named for what it does to its child, as [`Positioned`](crate::Positioned) and
/// [`Expanded`](crate::Expanded) are: `Align` is the cross-axis alignment of a row or a column.)
pub struct Aligned<Msg = crate::callback::Callback> {
    inner: Container<Msg>,
    alignment: AlignmentGeometry,
    /// `(across, down)`: a multiple of the child on that axis, or the room.
    factors: (Option<f32>, Option<f32>),
    /// Once a factor is given, `inner` moves here and is laid out apart: the box is then
    /// measured from it, as the boxes that transform the space on offer are.
    factored: Vec<Box<dyn Widget<Msg>>>,
    /// How a change of anchor moves, for [`AnimatedAlign`](crate::AnimatedAlign)
    /// (milestone 600).
    anim: Option<(f32, frus_core::animation::Curve)>,
}

impl<Msg: Clone + 'static> Aligned<Msg> {
    /// Places `child` at `alignment`.
    pub fn new(alignment: impl Into<AlignmentGeometry>, child: impl Widget<Msg> + 'static) -> Self {
        let alignment = alignment.into();
        Self {
            inner: Container::new().hugging().alignment(alignment).child(child),
            alignment,
            factors: (None, None),
            factored: Vec::new(),
            anim: None,
        }
    }

    /// Places `child` at `alignment`, moving there over `duration` when the anchor changes.
    ///
    /// The container that places the child moves its anchor too: without a factor it is this
    /// box's own node, and with one it places the child on the axes without a factor, while
    /// the walk moves it on the others (milestone 600).
    pub(crate) fn animated(
        alignment: impl Into<AlignmentGeometry>,
        duration: f32,
        curve: frus_core::animation::Curve,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        let alignment = alignment.into();
        Self {
            inner: Container::new()
                .hugging()
                .animated_alignment(alignment, duration, curve.clone())
                .child(child),
            alignment,
            factors: (None, None),
            factored: Vec::new(),
            anim: Some((duration, curve)),
        }
    }

    /// The box is `factor` times as wide as its child, rather than as wide as the room.
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.factors.0 = Some(factor.max(0.0));
        self.factor()
    }

    /// The box is `factor` times as tall as its child, rather than as tall as the room.
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.factors.1 = Some(factor.max(0.0));
        self.factor()
    }

    fn factor(mut self) -> Self {
        if self.factored.is_empty() {
            let inner = std::mem::take(&mut self.inner);
            self.factored.push(Box::new(inner));
        }
        self
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

    /// The box is `factor` times as wide as its child. See [`Aligned::width_factor`].
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.inner = self.inner.width_factor(factor);
        self
    }

    /// The box is `factor` times as tall as its child. See [`Aligned::height_factor`].
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.inner = self.inner.height_factor(factor);
        self
    }
}

impl<Msg: Clone> Aligned<Msg> {
    fn is_factored(&self) -> bool {
        !self.factored.is_empty()
    }
}

impl<Msg: Clone> Widget<Msg> for Aligned<Msg> {
    fn style(&self) -> Style {
        match self.is_factored() {
            true => Style::default(),
            false => self.inner.style(),
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        match self.is_factored() {
            true => Style::default(),
            false => self.inner.style_themed(theme),
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        match self.is_factored() {
            true => &self.factored,
            false => self.inner.children(),
        }
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // A pure layout widget: it draws nothing of its own.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn alignment_geometry(&self) -> Option<AlignmentGeometry> {
        Some(self.alignment)
    }

    fn anim_offset(&self) -> Option<(f32, f32)> {
        self.anim.as_ref().map(|_| self.alignment.fractions())
    }

    fn anim_duration(&self) -> f32 {
        self.anim.as_ref().map_or(0.0, |(duration, _)| *duration)
    }

    fn anim_curve(&self) -> frus_core::animation::Curve {
        self.anim
            .as_ref()
            .map_or(frus_core::animation::Curve::Linear, |(_, curve)| {
                curve.clone()
            })
    }

    /// With a factor, the child is laid out apart: allowed the room, as the most it may take,
    /// on the axes with a factor, and handed it on the others, where the container it sits in
    /// places it. The box is then the child's size times the factors. Where the child sits on
    /// an axis with a factor is the walk's to say, from the anchor and the reading direction.
    fn constraints_transform(&self) -> Option<crate::constraints::ConstraintsTransform> {
        use crate::constraints::AxisConstraint::{AsGiven, Loose};
        if !self.is_factored() {
            return None;
        }
        let rule = |factor: Option<f32>| match factor {
            Some(_) => Loose,
            None => AsGiven,
        };
        Some(crate::constraints::ConstraintsTransform {
            width: rule(self.factors.0),
            height: rule(self.factors.1),
            alignment: self.alignment.resolve(frus_core::TextDirection::Ltr),
            report: false,
        })
    }

    fn size_factor(&self) -> Option<(f32, f32)> {
        self.is_factored()
            .then(|| (self.factors.0.unwrap_or(1.0), self.factors.1.unwrap_or(1.0)))
    }

    /// All the room on offer, on both axes. The layout gives a box that asks for this the
    /// reference's answer: the whole of a bounded axis, and the child's length along a line it
    /// shares with others. An axis with a factor asks for nothing: the child decides it.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes {
            horizontal: self.factors.0.is_none(),
            vertical: self.factors.1.is_none(),
        }
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

    fn constraints_transform(&self) -> Option<crate::constraints::ConstraintsTransform> {
        self.inner.constraints_transform()
    }

    fn size_factor(&self) -> Option<(f32, f32)> {
        self.inner.size_factor()
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

    const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

    /// Every rect of `color` painted, with the clip it was painted under.
    fn painted(
        root: &dyn Widget<()>,
        size: Size,
        theme: &Theme,
        color: Color,
    ) -> Vec<(Rect, Rect)> {
        let ui = build_ui(root, size, &Runtime::default(), theme);
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    clip,
                    color: c,
                    ..
                } if *c == color => Some((*rect, *clip)),
                _ => None,
            })
            .collect()
    }

    /// A column: the factored box, then a blue bar that starts where the box ends.
    fn over_a_bar(anchor: impl Widget<()> + 'static) -> Flex<()> {
        Flex::<()>::column()
            .child(anchor)
            .child(Container::new().width(10.0).height(10.0).color(BLUE))
    }

    fn bar_top(root: &dyn Widget<()>, size: Size) -> f32 {
        painted(root, size, &Theme::dark(), BLUE)[0].0.y
    }

    /// **A height factor**: the box is that many times as tall as its child, with the child at
    /// the anchor inside, and what follows starts below it. Across, the box is still the room,
    /// and the child is centred in it — even a child that has no width of its own.
    #[test]
    fn a_height_factor_makes_the_box_a_multiple_of_the_child() {
        let size = Size::new(200.0, 200.0);
        let root = over_a_bar(Center::new(Container::new().child(square())).height_factor(3.0));
        let at = red(&root, size);
        assert!(near(at, 90.0, 20.0), "{at:?}");
        assert!((bar_top(&root, size) - 60.0).abs() < 0.5);

        let root = over_a_bar(Aligned::new(Alignment::BOTTOM_RIGHT, square()).height_factor(2.0));
        let at = red(&root, size);
        assert!(near(at, 180.0, 20.0), "{at:?}");
        assert!((bar_top(&root, size) - 40.0).abs() < 0.5);

        // Alone on the page, the height is on offer; the factor still asks the child, and the
        // box is 40 tall at the top, not the page's height.
        let at = red(&Center::new(square()).height_factor(2.0), size);
        assert!(near(at, 90.0, 10.0), "{at:?}");
    }

    /// **A width factor**: across, the same, and down the box is still the room.
    #[test]
    fn a_width_factor_makes_the_box_a_multiple_across() {
        let root = Flex::<()>::row()
            .height(100.0)
            .child(Center::new(square()).width_factor(3.0))
            .child(Container::new().width(10.0).height(10.0).color(BLUE));
        let size = Size::new(300.0, 100.0);
        let at = red(&root, size);
        assert!(near(at, 20.0, 40.0), "{at:?}");
        let bar = painted(&root, size, &Theme::dark(), BLUE)[0].0;
        assert!((bar.x - 60.0).abs() < 0.5, "{bar:?}");
    }

    /// **Both factors at one**: the box is exactly its child, wherever it is — the usual way to
    /// stop a centre taking the page.
    #[test]
    fn factors_of_one_hug_the_child() {
        let root = Container::<()>::new().child(over_a_bar(
            Center::new(square()).width_factor(1.0).height_factor(1.0),
        ));
        let size = Size::new(200.0, 200.0);
        let at = red(&root, size);
        assert!(near(at, 0.0, 0.0), "{at:?}");
        assert!((bar_top(&root, size) - 20.0).abs() < 0.5);
    }

    /// **Below one**, the box is smaller than the child, and the child is not cut: it spills,
    /// as the reference's does.
    #[test]
    fn a_factor_below_one_lets_the_child_spill() {
        let size = Size::new(200.0, 200.0);
        let root = over_a_bar(Aligned::new(Alignment::TOP_CENTER, square()).height_factor(0.5));
        let (at, clip) = painted(&root, size, &Theme::dark(), RED)[0];
        assert!(near(at, 90.0, 0.0), "{at:?}");
        assert!(clip.height >= 20.0, "not cut to the box: {clip:?}");
        assert!((bar_top(&root, size) - 10.0).abs() < 0.5);
    }

    /// **A directional anchor** on an axis with a factor follows the script: the start is the
    /// right in right-to-left.
    #[test]
    fn a_start_anchor_with_a_factor_follows_the_script() {
        let size = Size::new(200.0, 100.0);
        let root = Flex::<()>::row().width(200.0).height(100.0).child(
            Aligned::new(frus_core::AlignmentDirectional::CENTER_START, square()).width_factor(4.0),
        );
        let ltr = painted(&root, size, &Theme::dark(), RED)[0].0;
        assert!(near(ltr, 0.0, 40.0), "{ltr:?}");
        let rtl = painted(&root, size, &Theme::dark().rtl(), RED)[0].0;
        assert!(near(rtl, 180.0, 40.0), "{rtl:?}");
    }

    /// **A new factor is a new box**, on a frame that reuses the last one's layout.
    #[test]
    fn a_new_factor_is_a_new_box() {
        let runtime = Runtime::default();
        let size = Size::new(200.0, 200.0);
        let tops: Vec<f32> = [2.0, 3.0]
            .into_iter()
            .map(|factor| {
                let root = over_a_bar(Center::new(square()).height_factor(factor));
                let ui = build_ui(&root, size, &runtime, &Theme::dark());
                ui.scene()
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Rect { rect, color, .. } if *color == BLUE => Some(rect.y),
                        _ => None,
                    })
                    .expect("the bar")
            })
            .collect();
        assert_eq!(tops, vec![40.0, 60.0]);
    }

    /// **A paragraph under a width factor wraps at the room**, as the reference's does, and the
    /// box is its longest line: the room is the most it may take, not nothing (milestone 598).
    #[test]
    fn a_paragraph_under_a_width_factor_wraps_at_the_room() {
        let size = Size::new(120.0, 400.0);
        let words = "one two three four five six seven eight nine ten eleven twelve";
        let height = |text: &str| {
            bar_top(
                &over_a_bar(Aligned::new(Alignment::TOP_LEFT, Text::new(text)).width_factor(1.0)),
                size,
            )
        };
        let line = height("one");
        let paragraph = height(words);
        assert!(
            paragraph > 2.5 * line,
            "wrapped: {paragraph} against {line}"
        );
    }

    /// **Below one**, a paragraph is still laid out at the room it was measured in, not at the
    /// smaller box: it wraps at 120 and spills past a 60-px box.
    #[test]
    fn a_paragraph_under_a_small_factor_keeps_its_lines() {
        let words = "one two three four five six seven eight nine ten eleven twelve";
        let root =
            over_a_bar(Aligned::new(Alignment::TOP_LEFT, Text::new(words)).width_factor(0.5));
        let ui = build_ui(
            &root,
            Size::new(120.0, 400.0),
            &Runtime::default(),
            &Theme::dark(),
        );
        let wraps_at = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Text { max_width, .. } => Some(*max_width),
                _ => None,
            })
            .expect("the paragraph");
        assert_eq!(wraps_at, Some(120.0));
    }
}
