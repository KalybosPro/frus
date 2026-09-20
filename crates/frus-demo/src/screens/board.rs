//! The Kanban board screen.

use crate::prelude::*;
use frus_widgets::{column, row, ReorderAxis};

/// The width of one label of the strip.
const LABEL_WIDTH: f32 = 96.0;

/// The **Kanban** screen: a strip of labels to reorder across (milestone 527), then columns of
/// **rich cards** (a label + a × to delete), with per-column adding (milestone 249) and
/// drag-and-drop between columns (milestone 247). The screen holds the cards; the widget reports
/// a move, an add or a delete and the state applies it.
pub(crate) struct BoardPage;

/// What the board keeps.
#[derive(Default)]
pub(crate) struct BoardState {
    /// The Kanban cards, per column (milestone 247); `None` = the `KANBAN_SEED` starting set.
    /// `Some` as soon as a card is moved. See [`BoardState::columns`].
    pub(crate) cards: Option<Vec<Vec<String>>>,
    /// The order of the board's label strip, as indices into `BOARD_LABELS` (milestone 527);
    /// `None` = the order they are declared in. See [`BoardState::labels`].
    pub(crate) label_order: Option<Vec<usize>>,
}

impl BoardState {
    /// The Kanban cards per column: the ones held in the state if any have moved, otherwise the
    /// `KANBAN_SEED` starting set (milestone 247).
    pub(crate) fn columns(&self) -> Vec<Vec<String>> {
        match &self.cards {
            Some(cols) => cols.clone(),
            None => KANBAN_SEED
                .iter()
                .map(|col| col.iter().map(|s| s.to_string()).collect())
                .collect(),
        }
    }

    /// The board's labels in the order the strip shows them, as indices into `BOARD_LABELS`:
    /// the order held in the state once a label has moved, otherwise the declared one
    /// (milestone 527).
    pub(crate) fn labels(&self) -> Vec<usize> {
        self.label_order
            .clone()
            .unwrap_or_else(|| (0..BOARD_LABELS.len()).collect())
    }

    /// Moves a card: `(from_col, from_pos, to_col, to_pos)` (milestone 247).
    pub(crate) fn move_card(
        &mut self,
        from_col: usize,
        from_pos: usize,
        to_col: usize,
        to_pos: usize,
    ) {
        let mut cols = self.columns();
        if from_col < cols.len() && from_pos < cols[from_col].len() && to_col < cols.len() {
            let card = cols[from_col].remove(from_pos);
            // After the removal, a target further down the **same** column shifts by one.
            let mut at = to_pos;
            if from_col == to_col && from_pos < at {
                at -= 1;
            }
            at = at.min(cols[to_col].len());
            cols[to_col].insert(at, card);
            self.cards = Some(cols);
        }
    }

    /// Adds a card at the bottom of a column (milestone 249).
    pub(crate) fn add_card(&mut self, col: usize) {
        let mut cols = self.columns();
        if col < cols.len() {
            cols[col].push("New card".to_string());
            self.cards = Some(cols);
        }
    }

    /// Deletes the card `(col, pos)` (milestone 249).
    pub(crate) fn delete_card(&mut self, col: usize, pos: usize) {
        let mut cols = self.columns();
        if col < cols.len() && pos < cols[col].len() {
            cols[col].remove(pos);
            self.cards = Some(cols);
        }
    }

    /// A label of the strip carried to a new place: `(from, to)`, `to` being the index it ends
    /// up at (milestone 527). `to` already counts the label as gone from where it was, so it is
    /// removed first and put back at `to`.
    pub(crate) fn move_label(&mut self, from: usize, to: usize) {
        let mut order = self.labels();
        if from < order.len() && to < order.len() {
            let label = order.remove(from);
            order.insert(to, label);
            self.label_order = Some(order);
        }
    }
}

impl StatefulWidget for BoardPage {
    type State = BoardState;

    fn create_state(&self) -> BoardState {
        BoardState::default()
    }
}

impl State for BoardState {
    type Widget = BoardPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let theme = &theme;
        // The window this screen fills, read from the surface description in force:
        // nothing hands it down any more.
        let Size { width, height } = surface();
        let cols = self.columns();
        // Per-column vertical scrolling **with no explicit height** (milestone 266): the columns
        // fill the board's height (laid out in an ancestor with a defined height — here the
        // bounded screen and the horizontal SingleChildScrollView below) and each column
        // scrolls its cards through `flex(1)`. No height has to be computed any more (the old
        // `card_area_height` stopgap, milestone 264).
        let handle = cx.handle();
        let mut board = Kanban::new({
            let handle = handle.clone();
            move |from_col: usize, from_pos: usize, to_col: usize, to_pos: usize| {
                let handle = handle.clone();
                Callback::new(move || {
                    handle.set_state(|s| s.move_card(from_col, from_pos, to_col, to_pos))
                })
            }
        })
        .on_add(cx.handler(|s, col: usize| s.add_card(col)))
        .scrollable_columns();
        for (c, title) in KANBAN_TITLES.iter().enumerate() {
            let cards = cols.get(c).cloned().unwrap_or_default();
            let factories: Vec<CellFn> = cards
                .iter()
                .enumerate()
                .map(|(pos, label)| {
                    let label = label.clone();
                    let handle = handle.clone();
                    Box::new(move || rich_card(&handle, &label, c, pos)) as CellFn
                })
                .collect();
            board = board.column_widgets(*title, factories);
        }
        // The hint **wraps** within the width (otherwise the line runs off the right of the
        // screen).
        let hint =
            text("Add cards with + Add card; remove with ×; drag a card or a label to move it.")
                .size(13.0)
                .color(theme.muted)
                .wrap();
        // The board (a row of fixed-width columns) is wider than the screen, so it is made
        // scrollable **horizontally** — a **deliberate** axis: the row of columns is a
        // horizontal scroller, not a 2D pan. The cards' **vertical** scrolling belongs to each
        // column (milestone 266, `scrollable_columns`). Dragging a card reorders; dragging
        // empty space scrolls.
        //
        // A plain `Container` with padding is enough for the margin: since milestone 269
        // `compute_scroll` **fills the constrained axis**, so this `Container` takes the
        // viewport's height (it used to collapse, hence the old `Flex` `flex(1)` workaround)
        // and the board follows.
        let board_area = SingleChildScrollView::new()
            .axis(Axis::Horizontal)
            .width(width)
            .flex(1.0)
            .child(Container::new().padding(24.0).child(board));
        let hint_bar = Container::new().width(width).padding(24.0).child(hint);
        let router = cx.router();
        let screen = column![
            NavigationBar::new("Kanban board").on_back(on(move || {
                router.pop();
            })),
            self.label_strip(cx, theme, width),
            board_area,
            hint_bar
        ]
        .flex(1.0);
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                // The background runs **under** the bars; the content does not. `SafeArea`
                // reads the intrusions from the surface description, so a screen with no
                // `Scaffold` to do it for it still keeps clear of the notch.
                .child(SafeArea::new(screen)),
        )
    }
}

impl BoardState {
    /// The board's **label strip** (milestone 527): a `ReorderableList` whose rows run across,
    /// wider than a phone so that it scrolls, and so that a label carried to either edge scrolls
    /// it. The screen holds the order; the list reports `(from, to)`.
    fn label_strip(
        &self,
        cx: &StateContext<Self>,
        theme: &Theme,
        width: f32,
    ) -> SingleChildScrollView {
        let handle = cx.handle();
        let mut strip = ReorderableList::new(move |from: usize, to: usize| {
            let handle = handle.clone();
            Callback::new(move || handle.set_state(|s| s.move_label(from, to)))
        })
        .axis(ReorderAxis::Horizontal)
        .gap(8.0);
        for index in self.labels() {
            let label = Container::new()
                .width(LABEL_WIDTH)
                .padding(12.0)
                .radius(10.0)
                .color(theme.surface)
                .child(
                    row![text(BOARD_LABELS[index]).size(14.0).color(theme.on_surface)]
                        .justify(Justify::Center),
                );
            // Keyed by the label, not its place: the place is what changes.
            strip = strip.keyed_row(index as u64, label);
        }
        SingleChildScrollView::new()
            .axis(Axis::Horizontal)
            .width(width)
            .child(
                Container::new()
                    .padding_each(12.0, 24.0, 0.0, 24.0)
                    .child(strip),
            )
    }
}

/// A **rich card** of the Kanban (milestone 249): the label on the left, a **×** delete button
/// on the right.
fn rich_card(
    handle: &frus_widgets::StateHandle<BoardState>,
    label: &str,
    col: usize,
    pos: usize,
) -> Box<dyn Widget> {
    let handle = handle.clone();
    Box::new(
        row![
            text(label).size(14.0),
            Flex::row().flex(1.0),
            button(
                "×",
                on(move || handle.set_state(|s| s.delete_card(col, pos)))
            )
            .variant(Variant::Danger)
            .size(12.0),
        ]
        .align(Align::Center)
        .gap(8.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(cols: &[Vec<String>]) -> Vec<Vec<&str>> {
        cols.iter()
            .map(|c| c.iter().map(String::as_str).collect())
            .collect()
    }

    #[test]
    fn kanban_move_relocates_a_card() {
        let mut s = BoardState::default();
        assert_eq!(
            titles(&s.columns()),
            [
                vec!["Design API", "Write spec", "Triage bugs"],
                vec!["Build widget"],
                vec!["Kickoff", "Research"]
            ]
        );
        // Across columns.
        s.move_card(0, 0, 1, 0);
        assert_eq!(titles(&s.columns())[1], ["Design API", "Build widget"]);
        // Within a column, further down: the target shifts by one after the removal.
        s.move_card(0, 0, 0, 2);
        assert_eq!(titles(&s.columns())[0], ["Triage bugs", "Write spec"]);
        // Out of range changes nothing.
        let before = s.columns();
        s.move_card(9, 0, 0, 0);
        s.move_card(0, 9, 0, 0);
        s.move_card(0, 0, 9, 0);
        assert_eq!(s.columns(), before);
    }

    #[test]
    fn cards_are_added_at_the_bottom_and_deleted_by_place() {
        let mut s = BoardState::default();
        s.add_card(2);
        assert_eq!(titles(&s.columns())[2], ["Kickoff", "Research", "New card"]);
        s.delete_card(2, 0);
        assert_eq!(titles(&s.columns())[2], ["Research", "New card"]);
        s.add_card(9);
        s.delete_card(0, 9);
        assert_eq!(s.columns()[0].len(), 3);
    }

    #[test]
    fn a_label_carried_to_a_new_place_reorders_the_strip() {
        let mut s = BoardState::default();
        assert_eq!(s.labels(), (0..10).collect::<Vec<_>>());
        s.move_label(0, 3);
        assert_eq!(s.labels(), [1, 2, 3, 0, 4, 5, 6, 7, 8, 9]);
        s.move_label(9, 0);
        assert_eq!(s.labels()[0], 9);
        let before = s.labels();
        s.move_label(0, 10);
        assert_eq!(s.labels(), before, "off the end is refused");
    }
}
