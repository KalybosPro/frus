//! The editable grid screen: cell validation, and moving between the faults.

use crate::prelude::*;
use frus_widgets::host;
use frus_widgets::{column, row};

/// The **editable grid** screen: a `Table` whose every cell is an always-editable `TextField`.
/// Tab / Shift+Tab moves from cell to cell (the shell's focusables), Enter moves down one row
/// (milestone 201). The headers sort (milestone 204, `on_sort`), invalid cells show an error,
/// and Enter on the last row creates a new one.
pub(crate) struct GridPage {
    pub(crate) demo: Rc<Demo>,
}

/// What the grid screen keeps.
pub(crate) struct GridState {
    /// The grid's data (rows × columns of text): demonstration data to start with.
    pub(crate) grid: Vec<Vec<String>>,
    /// The grid's current sort: `(column, ascending)`; `None` = the order it was typed in.
    pub(crate) sort: Option<(usize, bool)>,
    /// The last faulty cell "Next error" targeted (milestone 214) — so it can cycle to the next.
    pub(crate) error_cursor: Option<(usize, usize)>,
}

impl Default for GridState {
    fn default() -> Self {
        let row = |a: &str, b: &str, c: &str| vec![a.to_string(), b.to_string(), c.to_string()];
        Self {
            grid: vec![
                row("Ada Lovelace", "Engineer", "ada@example.com"),
                row("Alan Turing", "Cryptographer", "alan@example.com"),
                row("Grace Hopper", "Admiral", "grace@example.com"),
            ],
            sort: None,
            error_cursor: None,
        }
    }
}

impl GridState {
    /// The new value of cell `(row, column)`.
    pub(crate) fn input(&mut self, r: usize, c: usize, value: String) {
        if let Some(cell) = self.grid.get_mut(r).and_then(|row| row.get_mut(c)) {
            *cell = value;
        }
    }

    /// Adds an empty row at the end of the grid.
    pub(crate) fn add_row(&mut self) {
        let cols = self.grid.first().map(|row| row.len()).unwrap_or(3);
        self.grid.push(vec![String::new(); cols]);
    }

    /// Enter pressed in cell `(row, column)`: moves down one row, same column; on the last row,
    /// one is created first. The cell to put the caret in.
    pub(crate) fn enter(&mut self, r: usize, c: usize) -> (usize, usize) {
        if r + 1 >= self.grid.len() {
            self.add_row();
        }
        (r + 1, c)
    }

    /// Deletes row `row`.
    pub(crate) fn delete_row(&mut self, r: usize) {
        if r < self.grid.len() {
            self.grid.remove(r);
        }
    }

    /// Toggles ascending / descending on the clicked column, then sorts the rows.
    pub(crate) fn sort_by_column(&mut self, c: usize) {
        let asc = match self.sort {
            Some((col, asc)) if col == c => !asc,
            _ => true,
        };
        self.sort = Some((c, asc));
        self.grid.sort_by(|a, b| {
            let (x, y) = (
                a.get(c).map(String::as_str).unwrap_or(""),
                b.get(c).map(String::as_str).unwrap_or(""),
            );
            let ord = x.to_lowercase().cmp(&y.to_lowercase());
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    /// The message a submission ends in: it only goes through when every cell is valid.
    pub(crate) fn save_message(&self) -> String {
        match grid_error_count(&self.grid) {
            0 => "Grid saved".to_string(),
            1 => "Fix 1 error before saving".to_string(),
            errors => format!("Fix {errors} errors before saving"),
        }
    }

    /// Cycles to the next faulty cell (wrapping). The cell to focus, if there is one.
    pub(crate) fn next_error(&mut self) -> Option<(usize, usize)> {
        let found = grid_next_error(&self.grid, self.error_cursor);
        self.error_cursor = found;
        found
    }
}

impl StatefulWidget for GridPage {
    type State = GridState;

    fn create_state(&self) -> GridState {
        GridState::default()
    }
}

impl State for GridState {
    type Widget = GridPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let theme = &theme;
        let demo = cx.widget().demo.clone();
        // The window this screen fills, read from the surface description in force:
        // nothing hands it down any more.
        let Size { width, height } = surface();
        const COL_W: [f32; 3] = [190.0, 170.0, 240.0];
        let muted = theme.muted;
        let mut table = Table::new(4)
            .header(&["Name", "Role", "Email", ""])
            .column_widths(&[COL_W[0], COL_W[1], COL_W[2], 44.0])
            .on_sort(cx.handler(|s, c: usize| s.sort_by_column(c)));
        // The sort indicator (a header arrow) on the sorted column.
        if let Some((col, asc)) = self.sort {
            table = table.sorted(col, asc);
        }
        for (r, row) in self.grid.iter().enumerate() {
            let mut cells: Vec<CellFn> = (0..3)
                .map(|c| {
                    let value = row[c].clone();
                    let w = COL_W[c] - 14.0;
                    let err = grid_cell_error(c, &value);
                    let handle = cx.handle();
                    let factory: CellFn = Box::new(move || {
                        let (typed, entered) = (handle.clone(), handle.clone());
                        let mut input = TextField::new(value.clone())
                            .width(w)
                            .size(15.0)
                            // A cell editor lives inside a row: dense is what that is for.
                            .dense(true)
                            .on_input(on_value(move |v: String| {
                                typed.set_state(|s| s.input(r, c, v.clone()))
                            }))
                            .on_submit(on(move || {
                                let mut next = (r + 1, c);
                                entered.set_state(|s| next = s.enter(r, c));
                                host::focus(("grid", next.0, next.1));
                            }));
                        if let Some(e) = err {
                            input = input.error(e);
                        }
                        Box::new(keyed(("grid", r, c), input)) as Box<dyn Widget>
                    });
                    factory
                })
                .collect();
            // The row's delete button (a non-focusable Container: Tab skips it).
            let handle = cx.handle();
            cells.push(Box::new(move || {
                let handle = handle.clone();
                Box::new(
                    Container::<Callback>::new()
                        .padding(6.0)
                        .child(Icon::new(Icons::CLOSE).size(16.0).color(muted))
                        .on_click(on(move || handle.set_state(|s| s.delete_row(r)))),
                ) as Box<dyn Widget>
            }));
            table = table.widget_row(cells);
        }
        let hint = text("Click a header to sort, Tab between cells, Enter for the next row.")
            .size(13.0)
            .color(theme.muted)
            .wrap();
        // The validation status bar: green when everything is valid, otherwise the error count.
        let errors = grid_error_count(&self.grid);
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
        // Focuses the new row's first cell.
        let add = button("Add row", {
            let handle = cx.handle();
            on(move || {
                let mut last = 0;
                handle.set_state(|s| {
                    s.add_row();
                    last = s.grid.len() - 1;
                });
                host::focus(("grid", last, 0));
            })
        });
        // `Save` is disabled (not clickable) for as long as a cell is invalid (milestone 210).
        let save = button("Save", {
            let handle = cx.handle();
            on(move || {
                let message = handle.read(|s| s.save_message());
                demo.toast(&message);
            })
        })
        .enabled(errors == 0);
        let mut actions = row![add, save].gap(12.0).align(Align::Center);
        // A shortcut that cycles through the faulty cells, shown only when there are errors.
        if errors > 0 {
            let handle = cx.handle();
            actions = actions.child(button(
                "Next error",
                on(move || {
                    let mut found = None;
                    handle.set_state(|s| found = s.next_error());
                    if let Some((r, c)) = found {
                        host::focus(("grid", r, c));
                    }
                }),
            ));
        }
        actions = actions.child(status);
        // The editable table (fixed columns, ~644 px): a bounded **scrollable** region (columns
        // in X, rows in Y).
        let table_area = SingleChildScrollView::new()
            .axis(Axis::Both)
            .flex(1.0)
            .child(table);
        let body = column![table_area, actions, hint]
            .gap(16.0)
            .padding(24.0)
            .flex(1.0);
        let router = cx.router();
        let screen = column![
            NavigationBar::new("Editable grid").on_back(on(move || {
                router.pop();
            })),
            body
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

#[cfg(test)]
mod tests {
    use super::*;

    fn names(state: &GridState) -> Vec<&str> {
        state.grid.iter().map(|row| row[0].as_str()).collect()
    }

    #[test]
    fn grid_edit_navigate_and_resize() {
        let mut s = GridState::default();
        assert_eq!(s.grid.len(), 3);
        s.input(0, 0, "Ada".into());
        assert_eq!(s.grid[0][0], "Ada");
        s.input(9, 9, "nowhere".into());
        assert_eq!(s.grid.len(), 3, "an edit outside the grid changes nothing");
        // Enter moves down a row, and on the last row makes one.
        assert_eq!(s.enter(0, 1), (1, 1));
        assert_eq!(s.grid.len(), 3);
        assert_eq!(s.enter(2, 1), (3, 1));
        assert_eq!(s.grid.len(), 4);
        assert_eq!(s.grid[3], ["", "", ""]);
        s.delete_row(3);
        s.delete_row(99);
        assert_eq!(s.grid.len(), 3);
    }

    #[test]
    fn grid_sort_toggles_and_validates() {
        let mut s = GridState::default();
        s.sort_by_column(0);
        assert_eq!(names(&s), ["Ada Lovelace", "Alan Turing", "Grace Hopper"]);
        assert_eq!(s.sort, Some((0, true)));
        s.sort_by_column(0);
        assert_eq!(names(&s), ["Grace Hopper", "Alan Turing", "Ada Lovelace"]);
        assert_eq!(s.sort, Some((0, false)), "again reverses");
        s.sort_by_column(1);
        assert_eq!(s.sort, Some((1, true)), "a new column starts ascending");
    }

    #[test]
    fn grid_save_is_gated_on_cell_errors() {
        let mut s = GridState::default();
        assert_eq!(s.save_message(), "Grid saved");
        s.input(0, 0, String::new());
        assert_eq!(s.save_message(), "Fix 1 error before saving");
        s.input(1, 2, "not an address".into());
        assert_eq!(s.save_message(), "Fix 2 errors before saving");
    }

    #[test]
    fn grid_focus_error_targets_the_first_faulty_cell_and_cycles() {
        let mut s = GridState::default();
        assert_eq!(s.next_error(), None, "nothing is wrong");
        s.input(0, 0, String::new());
        s.input(2, 2, "bad".into());
        assert_eq!(s.next_error(), Some((0, 0)));
        assert_eq!(s.next_error(), Some((2, 2)));
        assert_eq!(s.next_error(), Some((0, 0)), "and wraps");
        s.input(0, 0, "fixed".into());
        s.input(2, 2, "ok@example.com".into());
        assert_eq!(s.next_error(), None);
        assert_eq!(s.error_cursor, None);
    }
}
