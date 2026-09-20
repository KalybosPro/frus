//! The data-table screen: sorting, selection, paging.

use crate::prelude::*;
use frus_widgets::column;

/// Semantic rank of a priority level (`Low < Medium < High`) — the **custom** sort key of the
/// data table's "Level" column (a text sort would order it alphabetically).
pub(crate) fn level_rank(s: &str) -> u8 {
    match s {
        "Low" => 0,
        "Medium" => 1,
        "High" => 2,
        _ => 3,
    }
}

/// One row of the table: name, role, score and level.
pub(crate) type Person = (String, String, u32, String);

/// The **data table** screen: a read-only `DataTable` that **sorts its own rows** (milestone
/// 232) and **paginates** with a page-size selector (milestones 233/236). The screen only keeps
/// the `(sort, page, size)` state — the display sort is **not** duplicated in the state (a
/// deliberate contrast with the editable grid next door). — milestone 237.
pub(crate) struct DataPage;

/// What the data table keeps.
#[derive(Default)]
pub(crate) struct DataState {
    /// The data table's sort `(column, ascending)`; `None` = the source order (milestone 237).
    /// The display sort is done by the `DataTable`, **not** duplicated here.
    pub(crate) sort: Option<(usize, bool)>,
    /// The data table's current page (1-indexed); `0` = page 1 (milestone 237).
    pub(crate) page: usize,
    /// The data table's page size; `0` = the default (milestone 237).
    pub(crate) page_size: usize,
    /// The data table's selected **source** row; `None` = none (milestone 239). The `DataTable`
    /// translates that original index into a highlighted position through sorting/pagination.
    pub(crate) selected: Option<usize>,
    /// The data table's checked **source** rows (multi-selection, milestone 241). It drives the
    /// boxes and the highlighting; the screen decides what "check all" covers (here, all 12 rows).
    pub(crate) checked: Vec<usize>,
    /// The data table's search query (milestone 242); the `DataTable` filters the display.
    pub(crate) query: String,
    /// The data table's rows (milestone 243): `None` = the `DATA_PEOPLE` starting set; `Some` as
    /// soon as a bulk action (Delete) changes them. See [`DataState::rows`].
    pub(crate) edited: Option<Vec<Person>>,
    /// Is the data table's bulk-delete confirmation modal open? (milestone 245)
    pub(crate) confirm_delete: bool,
}

impl DataState {
    /// The data table's current rows: the ones held in the state if they have been changed (a
    /// bulk Delete), otherwise the `DATA_PEOPLE` starting set (milestone 243).
    pub(crate) fn rows(&self) -> Vec<Person> {
        match &self.edited {
            Some(rows) => rows.clone(),
            None => DATA_PEOPLE
                .iter()
                .map(|(n, r, s, l)| (n.to_string(), r.to_string(), *s, l.to_string()))
                .collect(),
        }
    }

    /// A click on a header: sorts (or reverses) the column. A new column starts ascending, and
    /// sorting returns to the first page.
    pub(crate) fn sort_by_column(&mut self, c: usize) {
        let asc = match self.sort {
            Some((col, asc)) if col == c => !asc,
            _ => true,
        };
        self.sort = Some((c, asc));
        self.page = 1;
    }

    /// Changing the size returns to the first page.
    pub(crate) fn set_page_size(&mut self, size: usize) {
        self.page_size = size;
        self.page = 1;
    }

    /// Clicking the already-selected row again deselects it (a toggle).
    pub(crate) fn select_row(&mut self, i: usize) {
        self.selected = if self.selected == Some(i) {
            None
        } else {
            Some(i)
        };
    }

    /// Toggles whether the source row belongs to the checked set.
    pub(crate) fn check(&mut self, i: usize) {
        match self.checked.iter().position(|&x| x == i) {
            Some(pos) => {
                self.checked.remove(pos);
            }
            None => self.checked.push(i),
        }
    }

    /// The header box: all checked → clear it; otherwise check every current source row.
    pub(crate) fn check_all(&mut self) {
        let n = self.rows().len();
        self.checked = if self.checked.len() == n {
            Vec::new()
        } else {
            (0..n).collect()
        };
    }

    /// A new search returns to the first page.
    pub(crate) fn search(&mut self, query: String) {
        self.query = query;
        self.page = 1;
    }

    /// Actually deletes the checked rows (by source index, in descending order so the following
    /// ones do not shift), then resets the selection and the modal.
    pub(crate) fn delete_checked(&mut self) {
        self.confirm_delete = false;
        let mut rows = self.rows();
        let mut checked: Vec<usize> = self
            .checked
            .iter()
            .copied()
            .filter(|&i| i < rows.len())
            .collect();
        checked.sort_unstable();
        checked.dedup();
        for &i in checked.iter().rev() {
            rows.remove(i);
        }
        self.edited = Some(rows);
        self.checked.clear();
        self.selected = None;
    }
}

impl StatefulWidget for DataPage {
    type State = DataState;

    fn create_state(&self) -> DataState {
        DataState::default()
    }
}

impl State for DataState {
    type Widget = DataPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let theme = &theme;
        // The confirmation is a modal: it takes the back gesture before the router does.
        cx.block_back(self.confirm_delete);
        // The window this screen fills, read from the surface description in force:
        // nothing hands it down any more.
        let Size { width, height } = surface();
        let people = self.rows();
        let rows: Vec<Vec<String>> = people
            .iter()
            .map(|(n, r, s, l)| vec![n.clone(), r.clone(), s.to_string(), l.clone()])
            .collect();
        // `0` (the derived default) = sensible starting values.
        let per = if self.page_size == 0 {
            5
        } else {
            self.page_size
        };
        let page = self.page.max(1);
        let (clear, ask_delete) = (
            cx.callback(|s| s.checked.clear()),
            cx.callback(|s| s.confirm_delete = true),
        );
        let mut table: DataTable = DataTable::new(["Name", "Role", "Score", "Level"], rows)
            .column_widths(&[200.0, 170.0, 90.0, 110.0])
            // The "Level" column: a **custom** sort key (Low < Medium < High), since a text
            // sort would order it alphabetically (milestone 240).
            .sort_with(3, |a, b| level_rank(a).cmp(&level_rank(b)))
            // Search: filters the source rows (every column) before sorting/pagination
            // (milestone 242).
            .searchable(self.query.as_str(), cx.handler(|s, q: String| s.search(q)))
            // An overridden empty state (milestone 244): a message when the filter/the data
            // show nothing.
            .empty_text("No people match your search")
            .on_sort(cx.handler(|s, c: usize| s.sort_by_column(c)))
            .on_select_row(cx.handler(|s, i: usize| s.select_row(i)))
            // Multi-selection (milestone 241): checkboxes for a bulk selection, on top of the
            // row click (focus). The checked rows drive the highlighting through `selected`.
            .checkboxes(
                cx.handler(|s, i: usize| s.check(i)),
                cx.callback(|s| s.check_all()),
            )
            .selected(&self.checked)
            // The bulk-actions bar (milestone 243): visible when rows are checked.
            .bulk_actions(move || {
                vec![
                    Box::new(
                        button("Clear", clear.clone())
                            .variant(Variant::Outlined)
                            .size(14.0),
                    ) as Box<dyn Widget>,
                    Box::new(
                        button("Delete", ask_delete.clone())
                            .variant(Variant::Danger)
                            .size(14.0),
                    ),
                ]
            })
            .paginated(page, per, cx.handler(|s, p: usize| s.page = p))
            .page_sizes(&[5, 10], cx.handler(|s, size: usize| s.set_page_size(size)));
        if let Some((col, asc)) = self.sort {
            table = table.sorted(col, asc);
        }
        let hint =
            text("Check rows for bulk actions; click a row to focus it; click a header to sort.")
                .size(13.0)
                .color(theme.muted)
                .wrap();
        // Detail of the **focused** row (a click on the row's body): read from the current data.
        let detail = match self.selected.and_then(|i| people.get(i)) {
            Some((n, r, s, l)) => text(format!("Focused: {n} — {r} (score {s}, {l} priority)"))
                .size(15.0)
                .color(theme.on_surface)
                .wrap(),
            None => text("No row focused.").size(15.0).color(theme.muted).wrap(),
        };
        // A summary of the **bulk** selection (the checked boxes).
        let summary = text(format!("{} checked", self.checked.len()))
            .size(13.0)
            .color(theme.muted);
        // The table (fixed columns, ~610 px) is wider than a phone: a bounded **scrollable**
        // region (columns in X, rows in Y) — not a page pan, a scrollable table.
        let table_area = SingleChildScrollView::new()
            .axis(Axis::Both)
            .flex(1.0)
            .child(table);
        // `flex(1.0)`: the body fills the height under the bar so the table region can stretch
        // (otherwise it falls back to its base size and leaves a large gap below).
        let body = column![table_area, detail, summary, hint]
            .gap(16.0)
            .padding(24.0)
            .flex(1.0);
        let router = cx.router();
        let screen = column![
            NavigationBar::new("Data table").on_back(on(move || {
                router.pop();
            })),
            body
        ]
        .flex(1.0);
        let content = Container::new()
            .width(width)
            .height(height)
            .color(theme.background)
            // The background runs **under** the bars; the content does not. `SafeArea` reads
            // the intrusions from the surface description, so a screen with no `Scaffold` to
            // do it for it still keeps clear of the notch.
            .child(SafeArea::new(screen));
        // A confirmation before the bulk delete (milestone 245): a centred modal, dismissible
        // by an outside click or Escape (`dismiss`), laid over the screen.
        if self.confirm_delete {
            Box::new(
                OverlayPortal::new(content)
                    .overlay(self.confirm_content(cx), Placement::Center)
                    .dismiss(cx.callback(|s| s.confirm_delete = false)),
            )
        } else {
            Box::new(content)
        }
    }
}

impl DataState {
    /// Content of the bulk-delete confirmation modal (milestone 245).
    fn confirm_content(&self, cx: &StateContext<Self>) -> Card {
        Card::new().padding(24.0).child(
            column![
                text("Delete selected rows?")
                    .size(22.0)
                    .weight(FontWeight::Medium),
                text(format!("{} row(s) will be removed.", self.checked.len())).size(16.0),
                frus_widgets::row![
                    button("Cancel", cx.callback(|s| s.confirm_delete = false))
                        .variant(Variant::Outlined),
                    button("Delete", cx.callback(|s| s.delete_checked())).variant(Variant::Danger),
                ]
                .justify(Justify::Center)
                .gap(12.0),
            ]
            .gap(16.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_is_ranked_by_meaning_not_by_spelling() {
        assert!(level_rank("Low") < level_rank("Medium"));
        assert!(level_rank("Medium") < level_rank("High"));
    }

    #[test]
    fn sorting_toggles_and_returns_to_the_first_page_without_touching_the_data() {
        let mut s = DataState {
            page: 3,
            ..Default::default()
        };
        s.sort_by_column(2);
        assert_eq!((s.sort, s.page), (Some((2, true)), 1));
        s.sort_by_column(2);
        assert_eq!(s.sort, Some((2, false)));
        s.sort_by_column(0);
        assert_eq!(s.sort, Some((0, true)));
        assert_eq!(s.rows().len(), DATA_PEOPLE.len(), "the source is untouched");
        assert_eq!(s.rows()[0].0, "Ada Lovelace");
    }

    #[test]
    fn a_new_size_or_search_returns_to_the_first_page() {
        let mut s = DataState {
            page: 2,
            ..Default::default()
        };
        s.set_page_size(10);
        assert_eq!((s.page_size, s.page), (10, 1));
        s.page = 2;
        s.search("ada".into());
        assert_eq!((s.query.as_str(), s.page), ("ada", 1));
    }

    #[test]
    fn selection_checks_and_bulk_delete() {
        let mut s = DataState::default();
        s.select_row(4);
        assert_eq!(s.selected, Some(4));
        s.select_row(4);
        assert_eq!(s.selected, None, "the same row again deselects it");
        s.check(0);
        s.check(2);
        s.check(0);
        assert_eq!(s.checked, [2]);
        s.check_all();
        assert_eq!(s.checked.len(), 12, "check all covers every source row");
        s.check_all();
        assert!(s.checked.is_empty(), "and again clears it");
        s.check(1);
        s.check(11);
        s.selected = Some(1);
        s.confirm_delete = true;
        s.delete_checked();
        let rows = s.rows();
        assert_eq!(rows.len(), 10);
        assert!(rows
            .iter()
            .all(|r| r.0 != "Alan Turing" && r.0 != "Vint Cerf"));
        assert!(s.checked.is_empty() && s.selected.is_none() && !s.confirm_delete);
    }
}
