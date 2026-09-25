//! A selection that spans **several texts**: [`SelectionArea`].
//!
//! A field's selection is [`Edit`](crate::Edit) state kept by the runtime under the field's
//! identity, which is the right home for one widget's range and the wrong one for a range that
//! begins in a paragraph and ends three widgets later. This one is kept by the area instead: two
//! ends — a text and a character in it — and, worked out from them, what each text in between
//! has selected. The texts are the ones the last frame registered inside the area
//! ([`TextStop`](crate::TextStop)), in the order they were painted, which is reading order for
//! everything a tree lays out in flow.
//!
//! The **ranges are stored, not derived while painting**: a text painted early in the walk
//! cannot know whether the ends are before or after it, because the texts after it have not
//! been walked yet. Whoever moves an end has the whole frame in hand — the shell — and works
//! the ranges out once.

use std::collections::HashMap;

use std::cell::OnceCell;

use frus_core::Color;

use crate::interaction::WidgetId;
use crate::selectiontoolbar::{SelectionToolbar, ToolbarContext, ToolbarItem};
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgettheme::TextSelectionTheme;

/// One end of a [`RegionSelection`]: a text, and a character boundary in it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RegionPoint {
    /// The text the end is in.
    pub text: WidgetId,
    /// The character boundary in it, in **characters**.
    pub index: usize,
}

/// The selection an area holds: where the press landed, where the pointer is now, and what
/// that makes selected in each text.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionSelection {
    /// The area the selection belongs to: a drag never carries it into another.
    pub area: WidgetId,
    /// Where the press landed.
    pub anchor: RegionPoint,
    /// Where the pointer is now.
    pub extent: RegionPoint,
    /// The `(start, end)` character range selected in each text that has one.
    pub ranges: HashMap<WidgetId, (usize, usize)>,
    /// Whether every text of the area is selected whole — Select all would do nothing.
    pub all: bool,
    /// Whether the **bar** (Copy, Select all) shows over the selection: one made with a finger
    /// does, one made with a mouse does not. Put away while a handle is dragged.
    pub bar: bool,
    /// Whether the selection carries **handles** at its two ends, for a finger to widen or
    /// narrow it: one made with a finger does.
    pub handles: bool,
    /// Where the selection begins, in reading order — whichever of `anchor` and `extent` comes
    /// first — as the first character selected. `None` when nothing is.
    pub start: Option<RegionPoint>,
    /// Where it ends, in reading order, as the boundary after the last character selected.
    pub end: Option<RegionPoint>,
}

impl RegionSelection {
    /// The selection from `anchor` to `extent` over `stops`, the area's texts in order, each
    /// with its length in characters.
    pub fn new(
        area: WidgetId,
        anchor: RegionPoint,
        extent: RegionPoint,
        stops: &[(WidgetId, usize)],
    ) -> Self {
        let ranges = region_ranges(stops, anchor, extent);
        let all = !stops.is_empty()
            && stops
                .iter()
                .all(|(id, len)| *len == 0 || ranges.get(id) == Some(&(0, *len)));
        let covered = stops
            .iter()
            .filter_map(|(id, _)| ranges.get(id).map(|range| (*id, *range)));
        let (mut start, mut end) = (None, None);
        for (id, (low, high)) in covered {
            start.get_or_insert(RegionPoint {
                text: id,
                index: low,
            });
            end = Some(RegionPoint {
                text: id,
                index: high,
            });
        }
        Self {
            area,
            anchor,
            extent,
            ranges,
            all,
            bar: false,
            handles: false,
            start,
            end,
        }
    }

    /// This selection as one made with a finger, with its bar and its handles — or as one made
    /// with a mouse, with neither.
    #[must_use]
    pub fn with_bar(mut self, bar: bool) -> Self {
        self.bar = bar;
        self.handles = bar;
        self
    }

    /// Whether anything is selected: an end that has not moved from the other is a caret,
    /// which is not.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

/// What is selected in each of `stops` — the area's texts in reading order, each with its
/// length in characters — between `anchor` and `extent`, in whichever order they were
/// dragged.
///
/// The first text is selected from its end of the selection to its last character, every text
/// between is selected whole, and the last is selected from its first character to its end of
/// the selection. Both ends in one text is the range between them. An empty range is left out;
/// an end whose text is not among `stops` (it scrolled out of the tree) gives no selection.
pub fn region_ranges(
    stops: &[(WidgetId, usize)],
    anchor: RegionPoint,
    extent: RegionPoint,
) -> HashMap<WidgetId, (usize, usize)> {
    let position = |point: RegionPoint| {
        stops
            .iter()
            .position(|(id, _)| *id == point.text)
            .map(|at| (at, point.index.min(stops[at].1)))
    };
    let (Some(a), Some(b)) = (position(anchor), position(extent)) else {
        return HashMap::new();
    };
    let (start, end) = if a <= b { (a, b) } else { (b, a) };
    let mut ranges = HashMap::new();
    for (at, (id, len)) in stops.iter().enumerate().take(end.0 + 1).skip(start.0) {
        let low = if at == start.0 { start.1 } else { 0 };
        let high = if at == end.0 { end.1 } else { *len };
        if low < high {
            ranges.insert(*id, (low, high));
        }
    }
    ranges
}

/// The words selected, as they are copied: each text's selected slice, in the order of
/// `texts`, **one per line** — a paragraph broken into several texts is copied as those
/// lines, which is what a reader who selected them across sees.
pub fn region_copy(
    texts: &[(WidgetId, &str)],
    ranges: &HashMap<WidgetId, (usize, usize)>,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    for (id, text) in texts {
        if let Some((start, end)) = ranges.get(id) {
            lines.push(text.chars().skip(*start).take(end - start).collect());
        }
    }
    lines.join("\n")
}

/// A text that takes part in a [`SelectionArea`], as one frame laid it out: what the shell
/// hit-tests a press against and works a selection out from.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TextStop {
    /// The text's identity.
    pub id: WidgetId,
    /// The box it was given, in window coordinates.
    pub rect: crate::Rect,
    /// The area it is in.
    pub area: WidgetId,
}

/// A subtree in which **every [`Text`](crate::Text) can be selected, and one drag selects
/// across them**: a paragraph broken into three texts selects as a paragraph, and copying
/// gives those three lines.
///
/// ```
/// use frus_widgets::{column, SelectionArea, Text};
///
/// let terms: SelectionArea<()> = SelectionArea::around(column![
///     Text::new("Terms of service").heading(),
///     Text::new("You may copy any of this."),
///     Text::new("Drag from one line to another."),
/// ]);
/// # let _ = terms;
/// ```
///
/// How it looks and what its bar offers are the application's: the highlight and the handles
/// take [`TextSelectionTheme`](crate::TextSelectionTheme), which
/// [`selection_color`](Self::selection_color) and [`handle_color`](Self::handle_color) set for
/// one area, and [`selection_toolbar`](Self::selection_toolbar) changes the list on the bar.
///
/// A text that is itself [`selectable`](crate::Text::selectable) keeps its own selection, and
/// what takes a press — a button, a field, a link — keeps that: a press begins a selection
/// only over words nothing else has a claim on.
///
/// The area does not draw anything and does not change the layout: it is a wrapper that is
/// its child, and says so to the texts inside it.
pub struct SelectionArea<Msg = crate::callback::Callback> {
    inner: Box<dyn Widget<Msg>>,
    /// The application's say over the bar, if it has said anything.
    toolbar_build: Option<ToolbarBuild<Msg>>,
    /// The bar for each context, built the first time it is asked for and kept: the walk
    /// borrows the widget it floats for a whole frame.
    toolbars: [OnceCell<Option<Box<dyn Widget<Msg>>>>; ToolbarContext::VARIANTS as usize],
    /// How the selection looks here, laid over what the theme says.
    look: TextSelectionTheme,
}

/// The bar for a context, as [`SelectionArea::selection_toolbar`] was told to make it.
type ToolbarBuild<Msg> = Box<dyn Fn(&ToolbarContext) -> Option<Box<dyn Widget<Msg>>>>;

impl<Msg: Clone + 'static> SelectionArea<Msg> {
    /// Wraps `child` so that the texts in it can be selected together.
    pub fn around(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(child),
            toolbar_build: None,
            toolbars: Default::default(),
            look: TextSelectionTheme::default(),
        }
    }

    /// Changes what the bar over the area's selection offers, as
    /// [`TextField::selection_toolbar`](crate::TextField::selection_toolbar) does for a field.
    /// `build` is given the context and the default list — Copy, and Select all unless
    /// everything is selected — and answers with the list to show; an empty one shows no bar.
    ///
    /// ```
    /// use frus_widgets::{SelectionArea, Text, ToolbarItem};
    ///
    /// // Copy only: nothing to select all of on a one-line receipt.
    /// let receipt: SelectionArea<()> = SelectionArea::around(Text::new("ORDER-4417"))
    ///     .selection_toolbar(|_, items| {
    ///         items.into_iter().filter(|item| item.label() == "Copy").collect()
    ///     });
    /// # let _ = receipt;
    /// ```
    #[must_use]
    pub fn selection_toolbar(
        mut self,
        build: impl Fn(&ToolbarContext, Vec<ToolbarItem<Msg>>) -> Vec<ToolbarItem<Msg>> + 'static,
    ) -> Self {
        self.toolbar_build = Some(Box::new(move |context: &ToolbarContext| {
            let mut items = Vec::new();
            if context.has_selection {
                items.push(ToolbarItem::copy());
            }
            if !context.all_selected {
                items.push(ToolbarItem::select_all());
            }
            let items = build(context, items);
            (!items.is_empty())
                .then(|| Box::new(SelectionToolbar::new(items)) as Box<dyn Widget<Msg>>)
        }));
        self.toolbars = Default::default();
        self
    }

    /// The highlight under the selected words in this area. Unset, the theme's
    /// [`TextSelectionTheme::selection_color`].
    #[must_use]
    pub fn selection_color(mut self, color: Color) -> Self {
        self.look.selection_color = Some(color);
        self
    }

    /// The handles under a touch selection in this area. Unset, the theme's
    /// [`TextSelectionTheme::handle_color`].
    #[must_use]
    pub fn handle_color(mut self, color: Color) -> Self {
        self.look.handle_color = Some(color);
        self
    }
}

impl<Msg> SelectionArea<Msg> {
    /// An area is not a box: its child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(SelectionArea {
    /// What makes the texts below selectable together, and how their selection looks —
    /// laid over the inherited theme, then whatever the child says about the theme.
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        let mut mine = inherited.clone();
        mine.widgets.text.selectable = true;
        let look = &mut mine.widgets.text_selection;
        look.selection_color = self.look.selection_color.or(look.selection_color);
        look.handle_color = self.look.handle_color.or(look.handle_color);
        Some(
            self.inner
                .theme_override(&mine)
                .unwrap_or_else(|| Box::new(mine)),
        )
    }

    /// The bar the application asked for; `None` when it asked for nothing, so that the
    /// texts offer theirs.
    fn area_toolbar(&self, context: ToolbarContext) -> Option<Option<&dyn Widget<Msg>>> {
        let build = self.toolbar_build.as_ref()?;
        let bar = self.toolbars[usize::from(context.variant())].get_or_init(|| build(&context));
        Some(bar.as_deref())
    }

    /// Forwarded: an area is not an identity, a place, a surface nor a form.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
    fn autofill_group(&self) -> bool {
        self.inner.autofill_group()
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> WidgetId {
        WidgetId::from_u64(n)
    }

    fn at(text: u64, index: usize) -> RegionPoint {
        RegionPoint {
            text: id(text),
            index,
        }
    }

    /// Three texts of 5, 10 and 4 characters.
    fn stops() -> Vec<(WidgetId, usize)> {
        vec![(id(1), 5), (id(2), 10), (id(3), 4)]
    }

    /// **A selection across texts takes the tail of the first, all of those between and the
    /// head of the last.**
    #[test]
    fn a_selection_takes_the_tail_the_middle_and_the_head() {
        let ranges = region_ranges(&stops(), at(1, 2), at(3, 3));
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[&id(1)], (2, 5), "from the press to the end");
        assert_eq!(ranges[&id(2)], (0, 10), "the whole of the one between");
        assert_eq!(ranges[&id(3)], (0, 3), "from the start to the pointer");
    }

    /// **Dragged upward it is the same selection**: the ends are ordered by where they are, not
    /// by which was pressed.
    #[test]
    fn dragging_upward_selects_the_same_words() {
        let down = region_ranges(&stops(), at(1, 2), at(3, 3));
        let up = region_ranges(&stops(), at(3, 3), at(1, 2));
        assert_eq!(down, up);
    }

    /// Both ends in one text is the range between them, either way round; an end that has not
    /// moved selects nothing.
    #[test]
    fn both_ends_in_one_text_are_the_range_between_them() {
        assert_eq!(region_ranges(&stops(), at(2, 7), at(2, 3))[&id(2)], (3, 7));
        assert_eq!(region_ranges(&stops(), at(2, 3), at(2, 7))[&id(2)], (3, 7));
        assert!(region_ranges(&stops(), at(2, 4), at(2, 4)).is_empty());
    }

    /// An end at the very start of the first text, or the very end of the last, leaves that
    /// text out when nothing of it is selected — no empty range is stored.
    #[test]
    fn a_text_with_nothing_selected_has_no_range() {
        let ranges = region_ranges(&stops(), at(1, 5), at(3, 0));
        assert_eq!(ranges.len(), 1, "only the one between: {ranges:?}");
        assert_eq!(ranges[&id(2)], (0, 10));
    }

    /// An end past the last character is the end, and an end whose text is not there gives no
    /// selection rather than a wrong one.
    #[test]
    fn an_end_is_clamped_and_a_missing_one_selects_nothing() {
        assert_eq!(region_ranges(&stops(), at(1, 0), at(1, 99))[&id(1)], (0, 5));
        assert!(region_ranges(&stops(), at(1, 0), at(9, 1)).is_empty());
    }

    /// **What is copied is one line per text**, in reading order and by character.
    #[test]
    fn the_copy_is_one_line_per_text() {
        let texts = [(id(1), "héllo"), (id(2), "second line"), (id(3), "tail")];
        let ranges = region_ranges(&stops_for(&texts), at(1, 1), at(3, 2));
        assert_eq!(region_copy(&texts, &ranges), "éllo\nsecond line\nta");
        let none = region_ranges(&stops_for(&texts), at(2, 3), at(2, 3));
        assert_eq!(region_copy(&texts, &none), "");
    }

    /// **Everything selected is `all`**, and anything less is not — which is what takes Select
    /// all off the bar.
    #[test]
    fn a_selection_knows_when_it_holds_everything() {
        let every = RegionSelection::new(id(9), at(1, 0), at(3, 4), &stops());
        assert!(every.all);
        let less = RegionSelection::new(id(9), at(1, 1), at(3, 4), &stops());
        assert!(!less.all);
        assert!(!RegionSelection::new(id(9), at(1, 0), at(3, 4), &stops()).bar);
        assert!(
            RegionSelection::new(id(9), at(1, 0), at(3, 4), &stops())
                .with_bar(true)
                .bar
        );
    }

    /// **The two ends are in reading order**, whichever way the selection was made: the start
    /// is the first character selected, the end the boundary after the last.
    #[test]
    fn the_ends_are_in_reading_order() {
        for (a, b) in [(at(1, 2), at(3, 3)), (at(3, 3), at(1, 2))] {
            let selection = RegionSelection::new(id(9), a, b, &stops());
            assert_eq!(selection.start, Some(at(1, 2)));
            assert_eq!(selection.end, Some(at(3, 3)));
        }
        // An end at the very end of a text leaves it out; the next one starts the selection.
        let skipped = RegionSelection::new(id(9), at(1, 5), at(3, 2), &stops());
        assert_eq!(skipped.start, Some(at(2, 0)));
        let nothing = RegionSelection::new(id(9), at(2, 4), at(2, 4), &stops());
        assert_eq!((nothing.start, nothing.end), (None, None));
    }

    fn stops_for(texts: &[(WidgetId, &str)]) -> Vec<(WidgetId, usize)> {
        texts
            .iter()
            .map(|(id, t)| (*id, t.chars().count()))
            .collect()
    }

    /// The area's texts as a frame lays them out.
    mod frame {
        use super::*;
        use crate::{build_ui, Container, Flex, Point, Rect, Runtime, Size, Text, Theme};

        fn tree() -> impl Widget<()> {
            Flex::column()
                .width(300.0)
                .height(400.0)
                .child(Text::new("outside").no_wrap())
                .child(SelectionArea::around(
                    Flex::column()
                        .child(Text::new("alpha").no_wrap())
                        .child(Text::new("beta").no_wrap())
                        .child(Text::new("").no_wrap()),
                ))
                .child(SelectionArea::around(
                    Flex::column().child(Text::new("gamma").no_wrap()),
                ))
        }

        fn frame(rt: &Runtime) -> crate::Ui<()> {
            build_ui(&tree(), Size::new(300.0, 400.0), rt, &Theme::default())
        }

        /// **Every text in an area is registered, in painted order, under the area's identity** —
        /// and not the text outside it, an empty one, or one in another area under the first's.
        #[test]
        fn the_texts_of_an_area_are_registered_in_order_under_its_identity() {
            let ui = frame(&Runtime::default());
            let stops = ui.text_stops();
            assert_eq!(stops.len(), 3, "alpha, beta, gamma: {stops:?}");
            assert_eq!(stops[0].area, stops[1].area, "one area holds the first two");
            assert_ne!(stops[1].area, stops[2].area, "and another holds the third");
            assert!(stops[0].rect.y < stops[1].rect.y, "in painted order");
            let first: Vec<_> = ui.text_stops_in(stops[0].area).map(|s| s.id).collect();
            assert_eq!(first, vec![stops[0].id, stops[1].id]);
            // Not the words outside, which sit above the areas.
            assert!(stops.iter().all(|s| s.rect.y > 10.0), "{stops:?}");
        }

        /// **A press finds the text under it, and a drag that left the words finds the nearest** —
        /// of its own area, never of the other.
        #[test]
        fn a_press_finds_its_text_and_a_stray_drag_the_nearest() {
            let ui = frame(&Runtime::default());
            let stops: Vec<TextStop> = ui.text_stops().to_vec();
            let inside = Point::new(stops[0].rect.x + 2.0, stops[0].rect.y + 2.0);
            assert_eq!(ui.text_stop_at(inside).map(|s| s.id), Some(stops[0].id));
            assert_eq!(
                ui.text_stop_at(Point::new(290.0, 390.0)),
                None,
                "empty space"
            );
            // Far below everything: the last text of the area asked about, not the other area.
            let far = Point::new(5.0, 399.0);
            assert_eq!(
                ui.nearest_text_stop(stops[0].area, far).map(|s| s.id),
                Some(stops[1].id)
            );
            assert_eq!(
                ui.nearest_text_stop(stops[2].area, far).map(|s| s.id),
                Some(stops[2].id)
            );
            // Beside a line, it is that line's, not the one above.
            let beside = Point::new(250.0, stops[1].rect.y + 3.0);
            assert_eq!(
                ui.nearest_text_stop(stops[0].area, beside).map(|s| s.id),
                Some(stops[1].id)
            );
        }

        /// **A repaint boundary that replays from the cache still registers its texts**: the
        /// registry is captured with the rest of what a subtree adds, or a selection could
        /// not begin in any text whose paint was reused.
        #[test]
        fn a_replayed_boundary_still_registers_its_texts() {
            let tree = SelectionArea::around(
                Container::new()
                    .repaint_boundary()
                    .child(Flex::column().child(Text::new("cached").no_wrap())),
            );
            let rt = Runtime::default();
            let theme = Theme::default();
            let size = Size::new(200.0, 100.0);
            let first = build_ui::<()>(&tree, size, &rt, &theme);
            let second = build_ui::<()>(&tree, size, &rt, &theme);
            assert_eq!(
                rt.paint_cache.borrow().last_frame_stats(),
                (1, 0),
                "the boundary was replayed"
            );
            assert_eq!(first.text_stops().len(), 1);
            assert_eq!(second.text_stops(), first.text_stops());
        }

        /// **A highlight repaints a cached subtree**: the selection is part of what the cache's
        /// fingerprint is made of, so a boundary whose text is newly selected is painted again
        /// and not replayed as it was.
        #[test]
        fn a_selection_repaints_a_boundary_that_was_cached() {
            let tree = SelectionArea::around(
                Container::new()
                    .repaint_boundary()
                    .child(Flex::column().child(Text::new("cached").no_wrap())),
            );
            let mut rt = Runtime::default();
            let theme = Theme::default();
            let size = Size::new(200.0, 100.0);
            let painted = |ui: &crate::Ui<()>| {
                ui.scene()
                    .primitives()
                    .iter()
                    .filter(|p| matches!(p, crate::Primitive::Rect { color, .. } if *color == theme.selection))
                    .count()
            };
            let bare = build_ui::<()>(&tree, size, &rt, &theme);
            assert_eq!(painted(&bare), 0);
            let id = bare.text_stops()[0].id;
            let area = bare.text_stops()[0].area;
            let point = RegionPoint { text: id, index: 0 };
            let end = RegionPoint { text: id, index: 3 };
            rt.region = Some(RegionSelection::new(area, point, end, &[(id, 6)]));
            let selected = build_ui::<()>(&tree, size, &rt, &theme);
            assert_eq!(
                painted(&selected),
                1,
                "the cached subtree was painted again"
            );
            let _: Rect = selected.text_stops()[0].rect;
        }

        /// **An area's colours paint its selection** (milestone 578): the highlight in the area's
        /// selection colour and the two handles in its handle colour — and with neither set, the
        /// theme's `selection` and `primary`, as before.
        #[test]
        fn an_areas_colours_paint_its_selection() {
            let (red, blue) = (
                frus_core::Color::rgb(1.0, 0.0, 0.0),
                frus_core::Color::rgb(0.0, 0.0, 1.0),
            );
            let words = || Flex::column().child(Text::new("colour").no_wrap());
            let theme = Theme::default();
            let size = Size::new(200.0, 100.0);
            let count = |ui: &crate::Ui<()>| {
                let (mut rects, mut paths) = (Vec::new(), Vec::new());
                for p in ui.scene().primitives() {
                    match p {
                        crate::Primitive::Rect { color, .. } => rects.push(*color),
                        crate::Primitive::Path {
                            fill: Some(color), ..
                        } => paths.push(*color),
                        _ => {}
                    }
                }
                (rects, paths)
            };
            let selected = |tree: &dyn Widget<()>| {
                let mut rt = Runtime::default();
                let bare = build_ui::<()>(tree, size, &rt, &theme);
                let stop = bare.text_stops()[0];
                let (from, to) = (
                    RegionPoint {
                        text: stop.id,
                        index: 1,
                    },
                    RegionPoint {
                        text: stop.id,
                        index: 4,
                    },
                );
                rt.region =
                    Some(RegionSelection::new(stop.area, from, to, &[(stop.id, 6)]).with_bar(true));
                count(&build_ui::<()>(tree, size, &rt, &theme))
            };

            let tailored = SelectionArea::around(words())
                .selection_color(red)
                .handle_color(blue);
            let (rects, paths) = selected(&tailored);
            assert!(rects.contains(&red), "the highlight: {rects:?}");
            assert!(!rects.contains(&theme.selection));
            assert_eq!(paths.iter().filter(|c| **c == blue).count(), 2, "{paths:?}");

            let plain = SelectionArea::around(words());
            let (rects, paths) = selected(&plain);
            assert!(rects.contains(&theme.selection));
            let primary = theme.scheme.primary;
            assert_eq!(paths.iter().filter(|c| **c == primary).count(), 2);
        }

        /// **What the application says about the bar is the area's answer**, through any wrapper
        /// around it — nothing said, it says nothing and the texts offer theirs.
        #[test]
        fn the_area_answers_for_its_bar() {
            let context = crate::ToolbarContext {
                has_selection: true,
                can_paste: false,
                all_selected: false,
            };
            let plain = SelectionArea::<()>::around(Text::new("x"));
            assert!(plain.area_toolbar(context).is_none());
            let copy_only =
                SelectionArea::<()>::around(Text::new("x")).selection_toolbar(|_, mut items| {
                    items.truncate(1);
                    items
                });
            assert!(matches!(copy_only.area_toolbar(context), Some(Some(_))));
            let wrapped = crate::Keyed::new(7u64, copy_only);
            assert!(matches!(wrapped.area_toolbar(context), Some(Some(_))));
            let none =
                SelectionArea::<()>::around(Text::new("x")).selection_toolbar(|_, _| Vec::new());
            assert!(matches!(none.area_toolbar(context), Some(None)));
            // And it is still an area: its texts take part.
            let theme = Theme::default();
            let scoped = Widget::<()>::theme_override(&none, &theme).expect("a theme");
            assert!(scoped.widgets.text.selectable);
        }
    }
}
