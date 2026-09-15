//! **Geometric** reflow of columns for a table's reorder preview: while a header is
//! being dragged, the neighbouring columns slide to **open the drop slot** and to
//! **close** the gap left by the lifted column — without the shell having to know
//! which widgets belong to which column.
//!
//! The primitives are grouped by **owner** (one cell = one `owner`): each cell
//! slides **as a block** (never shearing between its background and its text), by a
//! **continuous** amount that follows the cursor's position — so the slide **follows
//! the finger** instead of jumping from one column to the next. Blocks wider than a
//! column (page or row backgrounds) are left in place.

use std::collections::{HashMap, HashSet};

use frus_core::{Point, Primitive, Rect};

use crate::widget::ReorderAxis;

/// A box's `(start, extent)` **along** the axis a list is reordered on.
fn along(axis: ReorderAxis, r: Rect) -> (f32, f32) {
    match axis {
        ReorderAxis::Vertical => (r.y, r.height),
        ReorderAxis::Horizontal => (r.x, r.width),
    }
}

/// And **across** it: the band a list's slots share.
fn across(axis: ReorderAxis, r: Rect) -> (f32, f32) {
    match axis {
        ReorderAxis::Vertical => (r.x, r.width),
        ReorderAxis::Horizontal => (r.y, r.height),
    }
}

/// The factor of the "**background** vs cell/card" guard **shared** by both reflows: a block
/// whose **extent along the reorder axis** (width for horizontal columns, height for vertical
/// cards) exceeds `OVERSIZE_FACTOR × slot` is a page or column background, not a cell or card
/// — and is left in place. The two functions below are the **same idea on transposed axes**;
/// their bodies differ because the interaction model differs (columns: a **continuous** slide
/// following the cursor; cards: a **binary** shift according to the insertion line), so they
/// are not merged — only the constant is shared.
const OVERSIZE_FACTOR: f32 = 1.5;

/// Reflows the `prims` primitives for the preview, according to the cursor's abscissa
/// `cursor_x`. The **source** column (`src`, lifted, with `lifted_owner` its header) is
/// removed; the columns on either side slide by one slot (the source's width),
/// **progressively** as the cursor passes them, to fill the gap and open the drop slot.
pub fn reflow_reorder_columns(
    prims: &[Primitive],
    src: Rect,
    cursor_x: f32,
    lifted_owner: u64,
) -> Vec<Primitive> {
    let slot = src.width;
    // Beyond this width, a block covers more than a cell (a page or row background):
    // left in place, so as not to move an entire background.
    let max_cell = src.width * OVERSIZE_FACTOR;

    // The bounding box per owner (grouping a cell's background + text + icon).
    let mut bounds: HashMap<u64, Rect> = HashMap::new();
    for p in prims {
        let b = p.bounds();
        bounds
            .entry(p.owner())
            .and_modify(|r| *r = r.union(b))
            .or_insert(b);
    }

    // An owner's shift: `None` = removed (the source column), `Some(dx)` = translated.
    let shift_of = |owner: u64| -> Option<f32> {
        let b = bounds[&owner];
        let cx = b.x + b.width * 0.5;
        if b.width >= max_cell {
            return Some(0.0); // a wide background: left in place
        }
        // The source column: removed (it floats as a ghost).
        if owner == lifted_owner || (cx > src.x && cx < src.x + src.width) {
            return None;
        }
        // The transition width: the cell's, or one slot by default (cells with no
        // background, reduced to their text), for a smooth slide rather than a jump.
        let w = if b.width > 1.0 { b.width } else { slot };
        if cx >= src.x + src.width {
            // A right-hand neighbour: it slides left as the cursor passes it.
            let t = ((cursor_x - (cx - w * 0.5)) / w).clamp(0.0, 1.0);
            Some(-slot * t)
        } else {
            // A left-hand neighbour: it slides right as the cursor passes it.
            let t = (((cx + w * 0.5) - cursor_x) / w).clamp(0.0, 1.0);
            Some(slot * t)
        }
    };

    prims
        .iter()
        .filter_map(|p| shift_of(p.owner()).map(|dx| p.translated(dx, 0.0)))
        .collect()
}

/// **Vertical** reflow for the reorder preview of a Kanban's **cards**: while a card is
/// being dragged, those below it in its **source** column **move up** (the lifted card's gap
/// closes), and those in the **target** column at or below the **insertion line** **move
/// down** (the drop slot opens).
///
/// Like [`reflow_reorder_columns`], purely **geometric** — with no knowledge of the tree:
/// - `src`: the lifted card's bounds (the **source column**'s x band and one slot's height);
/// - `line`: the **insertion line** (the **target column**'s x band and the insertion y) —
///   `None` if the cursor is over no target (only the source gap closes);
/// - `lifted`: the owners of the lifted card's **subtree** — removed from the preview (they
///   are drawn separately as a ghost);
/// - `movable`: the owners of everything that can be reordered — cards, rows, drop zones, and
///   what each of them paints. **Nothing else moves.** The reflow is geometric, and a button
///   floating over a list, or the navigation bar under it, sits in the same band as the cards
///   without being one of them.
///
/// The **slot** threshold is the card's height. A block **taller** than `1.5×` that slot is a
/// column or page background (not a card): left in place — the vertical counterpart of the
/// `max_cell` guard. Each primitive slides according to the **centre** of its bounds; since
/// insertion lines land on card **edges** (never mid-centre), a card never shears.
///
/// **On the horizontal `axis`** — a list whose rows run along x — it is the same reflow
/// transposed: what follows the lifted row along x moves back by its width, what is at or
/// past the line moves on by it, and the band is the row's height. Without the background
/// guard, which is a board's: there is no horizontal board, and in a row of chips of
/// different widths a neighbour wider than one and a half of the lifted one is a chip, not a
/// background — kept still, its label would slide out of it. The geometry does not care
/// which way the list reads: in a mirrored layout the line is simply on the other edge.
pub fn reflow_reorder_cards(
    prims: &[Primitive],
    src: Rect,
    line: Option<Rect>,
    lifted: &HashSet<u64>,
    movable: &HashSet<u64>,
    axis: ReorderAxis,
) -> Vec<Primitive> {
    let (src_start, slot) = along(axis, src);
    let (src_band, src_band_extent) = across(axis, src);
    // Beyond this: a block covers more than a card (a column or page background) — left in place.
    let max_card = slot * OVERSIZE_FACTOR;
    let in_band = |c: f32, start: f32, extent: f32| c >= start && c <= start + extent;

    prims
        .iter()
        .filter_map(|p| {
            // The lifted card: removed from the preview (it floats as a ghost).
            if lifted.contains(&p.owner()) {
                return None;
            }
            // Not part of anything that can be reordered: it stays where it is drawn.
            if !movable.contains(&p.owner()) {
                return Some(p.clone());
            }
            let b = p.bounds();
            let (b_start, b_extent) = along(axis, b);
            // A large background (column or page): immobile. Only a board has one.
            if axis == ReorderAxis::Vertical && b_extent >= max_card {
                return Some(p.clone());
            }
            let (b_band, b_band_extent) = across(axis, b);
            let c_band = b_band + b_band_extent * 0.5;
            let c_along = b_start + b_extent * 0.5;
            let mut shift = 0.0;
            // The **target** column: whatever is at or past the line moves on (the gap opens).
            if let Some(line) = line {
                let (line_band, line_band_extent) = across(axis, line);
                if in_band(c_band, line_band, line_band_extent) && c_along >= along(axis, line).0 {
                    shift += slot;
                }
            }
            // The **source** column: whatever follows the lifted card moves back (the gap closes).
            if in_band(c_band, src_band, src_band_extent) && c_along > src_start {
                shift -= slot;
            }
            Some(match axis {
                ReorderAxis::Vertical => p.translated(0.0, shift),
                ReorderAxis::Horizontal => p.translated(shift, 0.0),
            })
        })
        .collect()
}

/// Which of a list's `slots` a carried item lands on when the pointer is over **none of
/// them** — the index into `slots`, or `None`. The list runs along `axis`.
///
/// A list is rarely all that is on its page. A finger that carries a row down to the bottom
/// edge is over whatever follows the list — a footer, a bar — and a finger that carries it
/// between two rows is over the gap. Neither is a row, and a drop that asked only *what is
/// under the pointer* put the row back where it came from: the gesture the reference answers
/// by landing at the nearest end. So the answer here is **the slot nearest along the axis**:
/// below the last one is the last, above the first is the first, and a gap belongs to the
/// closer of its two rows. Which half of it the pointer is on then decides before or after,
/// as it does over a row.
///
/// Only while the pointer is **across the list** — within the extent of its slots on the other
/// axis, the width of a vertical list or the height of a horizontal one. Beside it is
/// somewhere else: another column of a board, whose own targets answer for it.
///
/// *Past the end* is a place, not a direction: a horizontal list laid out right to left ends
/// at its left, and the slot nearest a pointer out there is still its last.
pub fn nearest_reorder_slot(point: Point, slots: &[Rect], axis: ReorderAxis) -> Option<usize> {
    let (point_along, point_band) = match axis {
        ReorderAxis::Vertical => (point.y, point.x),
        ReorderAxis::Horizontal => (point.x, point.y),
    };
    let near = slots.iter().map(|r| across(axis, *r).0).reduce(f32::min)?;
    let far = slots
        .iter()
        .map(|r| {
            let (start, extent) = across(axis, *r);
            start + extent
        })
        .reduce(f32::max)?;
    if point_band < near || point_band > far {
        return None;
    }
    let distance = |r: &Rect| {
        let (start, extent) = along(axis, *r);
        (start - point_along)
            .max(point_along - (start + extent))
            .max(0.0)
    };
    slots
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))
        .map(|(index, _)| index)
}

/// Is a carried item dropped **after** the `slot` the pointer is over, rather than before it?
///
/// The half the pointer is in decides, along the list's `axis`: the lower half of a row in a
/// vertical list, the right half of one in a horizontal list. **Except when the horizontal
/// list is laid out right to left** (`rtl`): what follows a row there is on its left, so the
/// left half is the one that means *after*. A vertical list reads down in either direction.
///
/// The insertion line and the release both come through here — the line on the edge of the
/// half, the release at the index after it — so the two cannot disagree about which side of a
/// row a drop is on.
pub fn reorder_drop_after(point: Point, slot: Rect, axis: ReorderAxis, rtl: bool) -> bool {
    match axis {
        ReorderAxis::Vertical => slot.height > 0.0 && point.y > slot.y + slot.height * 0.5,
        ReorderAxis::Horizontal => slot.width > 0.0 && (point.x > slot.x + slot.width * 0.5) != rtl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Scene};

    /// Three cells side by side (100 px, owners 1..3), plus a wide page background.
    fn scene() -> Scene {
        let mut s = Scene::new();
        s.fill_rect(Rect::new(0.0, 0.0, 300.0, 40.0), Color::WHITE); // fond (large)
        for i in 0..3 {
            s.set_owner((i + 1) as u64);
            s.fill_rect(Rect::new(i as f32 * 100.0, 0.0, 100.0, 40.0), Color::BLACK);
        }
        s
    }

    fn rect_x_of_owner(prims: &[Primitive], owner: u64) -> Option<f32> {
        prims.iter().find_map(|p| match p {
            Primitive::Rect { rect, owner: o, .. } if *o == owner && rect.width < 150.0 => {
                Some(rect.x)
            }
            _ => None,
        })
    }

    #[test]
    fn dragging_far_right_lifts_source_and_slides_middle_fully() {
        let base = scene();
        // Source = column 0 (owner 1); the cursor far right → the neighbours fully slid.
        let out = reflow_reorder_columns(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 40.0),
            1000.0,
            1,
        );
        assert_eq!(
            rect_x_of_owner(&out, 1),
            None,
            "the source column is removed"
        );
        assert!(
            out.iter()
                .any(|p| matches!(p, Primitive::Rect { rect, .. } if rect.width > 150.0)),
            "the background is kept"
        );
        assert_eq!(
            rect_x_of_owner(&out, 2),
            Some(0.0),
            "col 1 → 0 (slid by one slot)"
        );
        assert_eq!(
            rect_x_of_owner(&out, 3),
            Some(100.0),
            "col 2 → 100 (a slot opened on the right)"
        );
    }

    #[test]
    fn slide_is_partial_and_follows_the_cursor() {
        let base = scene();
        // The cursor at the **centre** of column 1 (owner 2, [100,200], centre 150).
        let out = reflow_reorder_columns(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 40.0),
            150.0,
            1,
        );
        // t = clamp((150 - (150 - 50)) / 100) = 0.5 → a half-way slide (−50).
        assert_eq!(
            rect_x_of_owner(&out, 2),
            Some(50.0),
            "col 1 half-way through its slide"
        );
        // Column 2 not yet reached by the cursor → immobile.
        assert_eq!(rect_x_of_owner(&out, 3), Some(200.0), "col 2 immobile");
    }

    #[test]
    fn dragging_left_slides_middle_right() {
        let base = scene();
        // Source = column 2 (owner 3); the cursor far left → the neighbours slid by +1 slot.
        let out = reflow_reorder_columns(
            base.primitives(),
            Rect::new(200.0, 0.0, 100.0, 40.0),
            -500.0,
            3,
        );
        assert_eq!(
            rect_x_of_owner(&out, 3),
            None,
            "the source column is removed"
        );
        assert_eq!(rect_x_of_owner(&out, 1), Some(100.0), "col 0 → 100");
        assert_eq!(rect_x_of_owner(&out, 2), Some(200.0), "col 1 → 200");
    }

    /// Two columns (x bands [0,100] and [120,220]) of 3 cards (44 px, slot 52) on a tall
    /// background. Cards: col A owners 1..3, col B owners 4..6; column backgrounds owners
    /// 100 / 200 (tall).
    fn board() -> Scene {
        let mut s = Scene::new();
        s.set_owner(100);
        s.fill_rect(Rect::new(0.0, 0.0, 100.0, 300.0), Color::WHITE);
        s.set_owner(200);
        s.fill_rect(Rect::new(120.0, 0.0, 100.0, 300.0), Color::WHITE);
        for i in 0..3 {
            let y = i as f32 * 52.0;
            s.set_owner((i + 1) as u64);
            s.fill_rect(Rect::new(0.0, y, 100.0, 44.0), Color::BLACK);
            s.set_owner((i + 4) as u64);
            s.fill_rect(Rect::new(120.0, y, 100.0, 44.0), Color::BLACK);
        }
        s
    }

    fn rect_y_of_owner(prims: &[Primitive], owner: u64) -> Option<f32> {
        prims.iter().find_map(|p| match p {
            Primitive::Rect { rect, owner: o, .. } if *o == owner && rect.height < 100.0 => {
                Some(rect.y)
            }
            _ => None,
        })
    }

    #[test]
    fn lifting_a_card_closes_the_source_gap() {
        let base = board();
        // The lifted card = col A, the top card (owner 1); no target (line = None).
        let lifted = HashSet::from([1]);
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 44.0),
            None,
            &lifted,
            &cards(),
            ReorderAxis::Vertical,
        );
        assert_eq!(rect_y_of_owner(&out, 1), None, "the lifted card is removed");
        assert_eq!(
            rect_y_of_owner(&out, 2),
            Some(8.0),
            "the next card moves up one slot (52 − 44)"
        );
        assert_eq!(
            rect_y_of_owner(&out, 3),
            Some(60.0),
            "and so does the last (104 − 44)"
        );
        assert_eq!(
            rect_y_of_owner(&out, 4),
            Some(0.0),
            "the neighbouring column is untouched"
        );
    }

    #[test]
    fn insertion_line_opens_a_hole_in_the_target_column() {
        let base = board();
        // The lifted card = col B, the top card (owner 4); target = col A, inserting before
        // the 2nd card (the line at owner 2's top edge, y=52).
        let lifted = HashSet::from([4]);
        let line = Rect::new(0.0, 52.0, 100.0, 3.0);
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(120.0, 0.0, 100.0, 44.0),
            Some(line),
            &lifted,
            &cards(),
            ReorderAxis::Vertical,
        );
        // The source column (B): the gap closes.
        assert_eq!(rect_y_of_owner(&out, 4), None, "the lifted card is removed");
        assert_eq!(
            rect_y_of_owner(&out, 5),
            Some(8.0),
            "source col: the next card moves up"
        );
        assert_eq!(
            rect_y_of_owner(&out, 6),
            Some(60.0),
            "source col: the last one moves up"
        );
        // The target column (A): the slot opens below the line.
        assert_eq!(
            rect_y_of_owner(&out, 1),
            Some(0.0),
            "above the line: immobile"
        );
        assert_eq!(
            rect_y_of_owner(&out, 2),
            Some(96.0),
            "below the line: moves down one slot (52 + 44)"
        );
        assert_eq!(
            rect_y_of_owner(&out, 3),
            Some(148.0),
            "and so does the next (104 + 44)"
        );
    }

    #[test]
    fn same_column_reflow_lifts_upper_cards_and_holds_the_rest() {
        // A reflow **within the same column** (source == target): lift the **top** card
        // (owner 1) and aim to insert **after** the 2nd card (the line at owner 3's edge, y=104).
        let base = board();
        let lifted = HashSet::from([1]);
        let line = Rect::new(0.0, 104.0, 100.0, 3.0);
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 44.0),
            Some(line),
            &lifted,
            &cards(),
            ReorderAxis::Vertical,
        );
        assert_eq!(rect_y_of_owner(&out, 1), None, "the lifted card is removed");
        // owner 2 (centre 74): below the source (−slot), above the line → **moves up** one slot.
        assert_eq!(
            rect_y_of_owner(&out, 2),
            Some(8.0),
            "the card above the line fills the gap"
        );
        // owner 3 (centre 126): below the source (−slot) **and** below the line (+slot) → a
        // **nil** net shift: it stays put, and the drop slot opens just above it.
        assert_eq!(
            rect_y_of_owner(&out, 3),
            Some(104.0),
            "the card below the line stays (a nil net shift)"
        );
        // The neighbouring column is untouched.
        assert_eq!(rect_y_of_owner(&out, 4), Some(0.0), "column B untouched");
    }

    /// Everything the board paints that can be reordered: the six cards.
    fn cards() -> HashSet<u64> {
        (1..=6).collect()
    }

    /// **Only what can be reordered makes room.** Seen on a phone: the reflow moved every
    /// primitive in the source's band below it, and the demo's floating action button kept
    /// its disc while its `+` slid up a row, and the navigation bar's items left the bar.
    #[test]
    fn only_what_can_be_reordered_makes_room() {
        let mut base = board();
        // A button floating over column A, below the lifted card and inside its band.
        base.set_owner(99);
        base.fill_rect(Rect::new(40.0, 120.0, 20.0, 20.0), Color::BLACK);
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 44.0),
            None,
            &HashSet::from([1]),
            &cards(),
            ReorderAxis::Vertical,
        );
        assert_eq!(
            rect_y_of_owner(&out, 2),
            Some(8.0),
            "a card below the lifted one still closes the gap"
        );
        assert_eq!(
            rect_y_of_owner(&out, 99),
            Some(120.0),
            "the floating button is not a card, and stays where it is"
        );
    }

    #[test]
    fn tall_backgrounds_stay_put() {
        let base = board();
        let lifted = HashSet::from([1]);
        let line = Rect::new(0.0, 52.0, 100.0, 3.0);
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 44.0),
            Some(line),
            &lifted,
            &cards(),
            ReorderAxis::Vertical,
        );
        // Both column backgrounds (height 300 > 1.5×44) stay at y = 0.
        let bgs: Vec<f32> = out
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } if rect.height > 150.0 => Some(rect.y),
                _ => None,
            })
            .collect();
        assert_eq!(bgs, vec![0.0, 0.0], "the column backgrounds are immobile");
    }

    /// Three rows 40 px tall with 8 px between them, in a list 16 px from the page's edge.
    fn rows() -> Vec<Rect> {
        (0..3)
            .map(|i| Rect::new(16.0, 100.0 + i as f32 * 48.0, 300.0, 40.0))
            .collect()
    }

    /// **Past either end of a list is that end.** Seen on a phone: a row carried to the
    /// bottom edge was over the footer that follows the list, and the release put it back.
    #[test]
    fn past_the_end_of_a_list_is_its_last_row_and_before_it_its_first() {
        let rows = rows();
        // Below the last row (100 + 2 x 48 + 40 = 236), however far.
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 260.0), &rows, ReorderAxis::Vertical),
            Some(2)
        );
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 900.0), &rows, ReorderAxis::Vertical),
            Some(2)
        );
        // Above the first.
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 20.0), &rows, ReorderAxis::Vertical),
            Some(0)
        );
    }

    /// A gap between two rows belongs to the nearer of them — and over a row, to that row.
    #[test]
    fn a_gap_between_rows_belongs_to_the_nearer_one() {
        let rows = rows();
        // The gap between the first and second rows runs from 140 to 148.
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 141.0), &rows, ReorderAxis::Vertical),
            Some(0)
        );
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 147.0), &rows, ReorderAxis::Vertical),
            Some(1)
        );
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 170.0), &rows, ReorderAxis::Vertical),
            Some(1)
        );
    }

    /// Beside the list is not in it: another column of a board answers for itself.
    #[test]
    fn beside_a_list_is_no_slot_of_it() {
        let rows = rows();
        assert_eq!(
            nearest_reorder_slot(Point::new(8.0, 260.0), &rows, ReorderAxis::Vertical),
            None
        );
        assert_eq!(
            nearest_reorder_slot(Point::new(330.0, 120.0), &rows, ReorderAxis::Vertical),
            None
        );
        // Its edges still are.
        assert_eq!(
            nearest_reorder_slot(Point::new(16.0, 260.0), &rows, ReorderAxis::Vertical),
            Some(2)
        );
        assert_eq!(
            nearest_reorder_slot(Point::new(316.0, 260.0), &rows, ReorderAxis::Vertical),
            Some(2)
        );
        assert_eq!(
            nearest_reorder_slot(Point::new(160.0, 260.0), &[], ReorderAxis::Vertical),
            None
        );
    }

    /// Three chips 44 px wide with 8 px between them, and a fourth more than twice as wide —
    /// owners 1..4, all in the band from y = 0 to 40 — plus a movable card below the strip
    /// (owner 5) that is not in its band.
    fn strip() -> Scene {
        let mut s = Scene::new();
        for i in 0..3 {
            s.set_owner((i + 1) as u64);
            s.fill_rect(Rect::new(i as f32 * 52.0, 0.0, 44.0, 40.0), Color::BLACK);
        }
        s.set_owner(4);
        s.fill_rect(Rect::new(156.0, 0.0, 100.0, 40.0), Color::BLACK);
        s.set_owner(5);
        s.fill_rect(Rect::new(52.0, 100.0, 44.0, 40.0), Color::BLACK);
        s
    }

    /// **A horizontal list closes the gap along x**, the vertical reflow transposed — and a
    /// wide chip is a chip: it makes room with the rest instead of standing still like a
    /// board's background.
    #[test]
    fn a_horizontal_reflow_closes_the_gap_along_x() {
        let base = strip();
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(0.0, 0.0, 44.0, 40.0),
            None,
            &HashSet::from([1]),
            &(1..=5).collect(),
            ReorderAxis::Horizontal,
        );
        assert_eq!(rect_x_of_owner(&out, 1), None, "the lifted chip is removed");
        assert_eq!(
            rect_x_of_owner(&out, 2),
            Some(8.0),
            "the next moves back (52 - 44)"
        );
        assert_eq!(
            rect_x_of_owner(&out, 3),
            Some(60.0),
            "and the one after (104 - 44)"
        );
        assert_eq!(
            rect_x_of_owner(&out, 4),
            Some(112.0),
            "the wide chip too (156 - 44): only a board has backgrounds"
        );
        assert_eq!(
            rect_x_of_owner(&out, 5),
            Some(52.0),
            "a card outside the strip's band stays"
        );
        assert_eq!(
            rect_y_of_owner(&out, 2),
            Some(0.0),
            "and nothing moves across the list"
        );
    }

    /// The line opens the slot along x: carried after the second chip, the second fills the
    /// gap and the third, past the line, has nowhere to go.
    #[test]
    fn a_horizontal_line_opens_the_slot_along_x() {
        let base = strip();
        let out = reflow_reorder_cards(
            base.primitives(),
            Rect::new(0.0, 0.0, 44.0, 40.0),
            Some(Rect::new(104.0, 0.0, 3.0, 40.0)),
            &HashSet::from([1]),
            &(1..=5).collect(),
            ReorderAxis::Horizontal,
        );
        assert_eq!(
            rect_x_of_owner(&out, 2),
            Some(8.0),
            "before the line: fills the gap"
        );
        assert_eq!(
            rect_x_of_owner(&out, 3),
            Some(104.0),
            "past it: a nil net shift"
        );
    }

    /// **Right to left, the geometry holds.** The strip mirrored: the first chip on the right
    /// (x = 104), the second in the middle, the third on the left. Carried after the second —
    /// whose *after* edge is its left one, x = 52 — the second takes the first's place and the
    /// slot opens between it and the third.
    #[test]
    fn a_mirrored_strip_opens_the_slot_on_the_left() {
        let mut s = Scene::new();
        for (owner, x) in [(1, 104.0), (2, 52.0), (3, 0.0)] {
            s.set_owner(owner);
            s.fill_rect(Rect::new(x, 0.0, 44.0, 40.0), Color::BLACK);
        }
        let out = reflow_reorder_cards(
            s.primitives(),
            Rect::new(104.0, 0.0, 44.0, 40.0),
            Some(Rect::new(50.5, 0.0, 3.0, 40.0)),
            &HashSet::from([1]),
            &(1..=3).collect(),
            ReorderAxis::Horizontal,
        );
        assert_eq!(
            rect_x_of_owner(&out, 2),
            Some(96.0),
            "on past the line (52 + 44)"
        );
        assert_eq!(
            rect_x_of_owner(&out, 3),
            Some(0.0),
            "before it, in reading order: stays"
        );
    }

    /// The three rows of [`rows`], turned on their side: chips 40 px wide, 8 px apart, 300
    /// tall, 16 px from the top of the page.
    fn chips() -> Vec<Rect> {
        (0..3)
            .map(|i| Rect::new(100.0 + i as f32 * 48.0, 16.0, 40.0, 300.0))
            .collect()
    }

    /// **Past either end of a horizontal list is that end**, along x.
    #[test]
    fn past_either_end_of_a_horizontal_list_is_that_end() {
        let chips = chips();
        let at = |x, y| nearest_reorder_slot(Point::new(x, y), &chips, ReorderAxis::Horizontal);
        // Right of the last chip (100 + 2 x 48 + 40 = 236), however far; left of the first.
        assert_eq!(at(260.0, 160.0), Some(2));
        assert_eq!(at(900.0, 160.0), Some(2));
        assert_eq!(at(20.0, 160.0), Some(0));
        // The gap between the first two runs from 140 to 148.
        assert_eq!(at(141.0, 160.0), Some(0));
        assert_eq!(at(147.0, 160.0), Some(1));
        // Above or below the strip is not in it; its edges still are.
        assert_eq!(at(260.0, 8.0), None);
        assert_eq!(at(120.0, 330.0), None);
        assert_eq!(at(260.0, 16.0), Some(2));
        assert_eq!(at(260.0, 316.0), Some(2));
        // The same point read as a vertical list is beside it: the axis is what decides.
        assert_eq!(
            nearest_reorder_slot(Point::new(260.0, 160.0), &chips, ReorderAxis::Vertical),
            None
        );
    }

    /// **Which half means after.** The lower half down a list, the right half across one —
    /// and the left half across one that reads right to left.
    #[test]
    fn the_half_that_means_after_follows_the_axis_and_the_reading_direction() {
        let slot = Rect::new(100.0, 100.0, 80.0, 40.0);
        let upper_left = Point::new(110.0, 110.0);
        let lower_right = Point::new(170.0, 130.0);
        let vertical = |p, rtl| reorder_drop_after(p, slot, ReorderAxis::Vertical, rtl);
        let horizontal = |p, rtl| reorder_drop_after(p, slot, ReorderAxis::Horizontal, rtl);
        assert!(!vertical(upper_left, false) && vertical(lower_right, false));
        assert!(
            !vertical(upper_left, true) && vertical(lower_right, true),
            "down a list, the reading direction changes nothing"
        );
        // Across: the right half is after, whichever half of the height.
        assert!(horizontal(Point::new(170.0, 110.0), false));
        assert!(!horizontal(Point::new(110.0, 130.0), false));
        // Right to left: the left half is.
        assert!(horizontal(Point::new(110.0, 130.0), true));
        assert!(!horizontal(Point::new(170.0, 110.0), true));
        // A slot with no extent along the axis has no half to be after.
        let flat = Rect::new(100.0, 100.0, 0.0, 40.0);
        assert!(!reorder_drop_after(
            Point::new(120.0, 120.0),
            flat,
            ReorderAxis::Horizontal,
            false
        ));
    }
}
