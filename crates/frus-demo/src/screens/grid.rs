//! The editable grid screen: cell validation, and moving between the faults.

use crate::prelude::*;
use frus_widgets::{column, row};

/// The **editable grid** screen: a `Table` whose every cell is an always-editable `TextField`.
/// Tab / Shift+Tab moves from cell to cell (the shell's focusables), Enter moves down one row
/// (milestone 201). The headers sort (milestone 204, `on_sort`), invalid cells show an error,
/// and Enter on the last row creates a new one.
pub(crate) fn grid_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Box<dyn Widget<Msg>> {
    const COL_W: [f32; 3] = [190.0, 170.0, 240.0];
    let muted = theme.muted;
    let mut table = Table::new(4)
        .header(&["Name", "Role", "Email", ""])
        .column_widths(&[COL_W[0], COL_W[1], COL_W[2], 44.0])
        .on_sort(Msg::GridSort);
    // The sort indicator (a header arrow) on the sorted column.
    if let Some((col, asc)) = app.grid_sort {
        table = table.sorted(col, asc);
    }
    for (r, row) in app.grid.iter().enumerate() {
        let mut cells: Vec<CellFn<Msg>> = (0..3)
            .map(|c| {
                let value = row[c].clone();
                let w = COL_W[c] - 14.0;
                let err = grid_cell_error(c, &value);
                let factory: CellFn<Msg> = Box::new(move || {
                    let mut input = TextField::new(value.clone())
                        .width(w)
                        .size(15.0)
                        // A cell editor lives inside a row: dense is what that is for.
                        .dense(true)
                        .on_input(move |v| Msg::GridInput(r, c, v))
                        .on_submit(Msg::GridEnter(r, c));
                    if let Some(e) = err {
                        input = input.error(e);
                    }
                    Box::new(keyed(("grid", r, c), input)) as Box<dyn Widget<Msg>>
                });
                factory
            })
            .collect();
        // The row's delete button (a non-focusable Container: Tab skips it).
        cells.push(Box::new(move || {
            Box::new(
                Container::<Msg>::new()
                    .padding(6.0)
                    .child(Icon::new(Icons::Close).size(16.0).color(muted))
                    .on_click(Msg::GridDeleteRow(r)),
            ) as Box<dyn Widget<Msg>>
        }));
        table = table.widget_row(cells);
    }
    let hint = text("Click a header to sort, Tab between cells, Enter for the next row.")
        .size(13.0)
        .color(theme.muted)
        .wrap();
    // The validation status bar: green when everything is valid, otherwise the error count.
    let errors = grid_error_count(&app.grid);
    let status = if errors == 0 {
        text("All cells valid").size(13.0).color(theme.primary)
    } else {
        let label = if errors == 1 {
            "1 error".to_string()
        } else {
            format!("{errors} errors")
        };
        text(label).size(13.0).color(theme.error)
    };
    let add = button("Add row", Msg::GridAddRow);
    // `Save` is disabled (not clickable) for as long as a cell is invalid (milestone 210).
    let save = button("Save", Msg::GridSave).enabled(errors == 0);
    let mut actions = row![add, save].gap(12.0).align(Align::Center);
    // A shortcut that cycles through the faulty cells, shown only when there are errors.
    if errors > 0 {
        actions = actions.child(button("Next error", Msg::GridFocusError));
    }
    actions = actions.child(status);
    // The editable table (fixed columns, ~644 px): a bounded **scrollable** region (columns in X, rows in Y).
    let table_area = SingleChildScrollView::new()
        .axis(Axis::Both)
        .flex(1.0)
        .child(table);
    let body = column![table_area, actions, hint]
        .gap(16.0)
        .padding(24.0)
        .flex(1.0);
    let screen = column![NavigationBar::new("Editable grid").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Box::new(
        Container::new()
            .width(width)
            .height(height)
            .color(theme.background)
            .child(screen),
    )
}
