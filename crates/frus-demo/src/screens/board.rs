//! The Kanban board screen.

use crate::prelude::*;
use frus_widgets::{column, row};

/// A **rich card** of the Kanban (milestone 249): the label on the left, a **×** delete button
/// on the right (`KanbanDelete(col, pos)`).
pub(crate) fn rich_card(label: &str, col: usize, pos: usize) -> Box<dyn Widget<Msg>> {
    Box::new(
        row![
            text(label).size(14.0),
            Flex::row().flex(1.0),
            button("×", Msg::KanbanDelete(col, pos))
                .variant(Variant::Danger)
                .size(12.0),
        ]
        .align(Align::Center)
        .gap(8.0),
    )
}

/// The **Kanban** screen: columns of **rich cards** (a label + a × to delete), with per-column
/// adding (milestone 249) and drag-and-drop between columns (milestone 247). The app holds the
/// cards; the widget emits `KanbanMove`/`KanbanAdd`/`KanbanDelete` and the reducer applies them.
pub(crate) fn board_screen(app: &TodoApp, theme: &Theme) -> Box<dyn Widget<Msg>> {
    // The window this screen fills, read from the surface description in force:
    // nothing hands it down any more.
    let Size { width, height } = surface();
    let cols = app.kanban_cols();
    // Per-column vertical scrolling **with no explicit height** (milestone 266): the columns fill
    // the board's height (laid out in an ancestor with a defined height — here the bounded screen
    // and the horizontal SingleChildScrollView below) and each column scrolls its cards through `flex(1)`. No
    // height has to be computed any more (the old `card_area_height` stopgap, milestone 264).
    let mut board = Kanban::new(Msg::KanbanMove)
        .on_add(Msg::KanbanAdd)
        .scrollable_columns();
    for (c, title) in KANBAN_TITLES.iter().enumerate() {
        let cards = cols.get(c).cloned().unwrap_or_default();
        let factories: Vec<CellFn<Msg>> = cards
            .iter()
            .enumerate()
            .map(|(pos, label)| {
                let label = label.clone();
                Box::new(move || rich_card(&label, c, pos)) as CellFn<Msg>
            })
            .collect();
        board = board.column_widgets(*title, factories);
    }
    // The hint **wraps** within the width (otherwise the line runs off the right of the screen).
    let hint = text("Add cards with + Add card; remove with ×; drag a card to move it.")
        .size(13.0)
        .color(theme.muted)
        .wrap();
    // The board (a row of fixed-width columns) is wider than the screen, so it is made scrollable
    // **horizontally** — a **deliberate** axis: the row of columns is a horizontal scroller, not a
    // 2D pan. The cards' **vertical** scrolling belongs to each column (milestone 266,
    // `scrollable_columns`). Dragging a card reorders; dragging empty space scrolls.
    //
    // A plain `Container` with padding is enough for the margin: since milestone 269
    // `compute_scroll` **fills the constrained axis**, so this `Container` takes the viewport's
    // height (it used to collapse, hence the old `Flex` `flex(1)` workaround) and the board
    // follows.
    let board_area = SingleChildScrollView::new()
        .axis(Axis::Horizontal)
        .width(width)
        .flex(1.0)
        .child(Container::new().padding(24.0).child(board));
    let hint_bar = Container::new().width(width).padding(24.0).child(hint);
    let screen = column![
        NavigationBar::new("Kanban board").on_back(Msg::Pop),
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
