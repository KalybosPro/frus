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

use crate::interaction::WidgetId;
use crate::theme::Theme;
use crate::themed::Themed;
use crate::widget::Widget;

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
        Self {
            area,
            anchor,
            extent,
            ranges: region_ranges(stops, anchor, extent),
        }
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
/// let terms: frus_widgets::Themed<()> = SelectionArea::around(column![
///     Text::new("Terms of service").heading(),
///     Text::new("You may copy any of this."),
///     Text::new("Drag from one line to another."),
/// ]);
/// # let _ = terms;
/// ```
///
/// A text that is itself [`selectable`](crate::Text::selectable) keeps its own selection, and
/// what takes a press — a button, a field, a link — keeps that: a press begins a selection
/// only over words nothing else has a claim on.
///
/// The area does not draw anything and does not change the layout: it is a wrapper that is
/// its child, and says so to the texts inside it.
pub struct SelectionArea;

impl SelectionArea {
    /// Wraps `child` so that the texts in it can be selected together.
    pub fn around<Msg: 'static>(child: impl Widget<Msg> + 'static) -> Themed<Msg> {
        Themed::tweak(
            |theme: &mut Theme| theme.widgets.text.selectable = true,
            child,
        )
    }
}

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
    }
}
