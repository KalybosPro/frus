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

use frus_core::{Primitive, Rect};

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
///   are drawn separately as a ghost).
///
/// The **slot** threshold is the card's height. A block **taller** than `1.5×` that slot is a
/// column or page background (not a card): left in place — the vertical counterpart of the
/// `max_cell` guard. Each primitive slides according to the **centre** of its bounds; since
/// insertion lines land on card **edges** (never mid-centre), a card never shears.
pub fn reflow_reorder_cards(
    prims: &[Primitive],
    src: Rect,
    line: Option<Rect>,
    lifted: &HashSet<u64>,
) -> Vec<Primitive> {
    let slot = src.height;
    // Beyond this: a block covers more than a card (a column or page background) — left in place.
    let max_card = src.height * OVERSIZE_FACTOR;
    let in_band = |cx: f32, x0: f32, w: f32| cx >= x0 && cx <= x0 + w;

    prims
        .iter()
        .filter_map(|p| {
            // The lifted card: removed from the preview (it floats as a ghost).
            if lifted.contains(&p.owner()) {
                return None;
            }
            let b = p.bounds();
            // A large background (column or page): immobile.
            if b.height >= max_card {
                return Some(p.clone());
            }
            let cx = b.x + b.width * 0.5;
            let cy = b.y + b.height * 0.5;
            let mut dy = 0.0;
            // The **target** column: whatever is at or below the line moves down (the gap opens).
            if let Some(line) = line {
                if in_band(cx, line.x, line.width) && cy >= line.y {
                    dy += slot;
                }
            }
            // The **source** column: whatever follows the lifted card moves up (the gap closes).
            if in_band(cx, src.x, src.width) && cy > src.y {
                dy -= slot;
            }
            Some(p.translated(0.0, dy))
        })
        .collect()
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
}
