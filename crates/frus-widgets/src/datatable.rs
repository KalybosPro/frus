//! [`DataTable`]: a [`Table`](crate::Table) that **sorts its own data**.
//!
//! The base `Table` is purely controlled: it only emits the column clicked (`on_sort`) and
//! shows the indicator for the `sorted` state it is handed — it is the application that
//! reorders its rows. `DataTable` encapsulates that **display** reordering: it is given the
//! raw rows and the sort state `(column, direction)`, and rebuilds a `Table` with the rows
//! already **sorted** (numeric-aware, case-insensitive) and the indicator. The model stays
//! controlled — the sort state lives in the app — but the sorting logic is no longer copied
//! out by hand.

use std::cmp::Ordering;
use std::rc::Rc;

use frus_core::{Rect, Scene};
use frus_layout::{Align, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::pagination::Pagination;
use crate::segmented::SegmentedButton;
use crate::table::Table;

/// An application's own ordering for one column — dates, amounts, priorities — where
/// comparing the rendered strings would sort them wrongly.
type Comparator = Rc<dyn Fn(&str, &str) -> Ordering>;

/// The buttons of the bulk-action bar, built on demand while a selection stands.
type BulkActions<Msg> = Rc<dyn Fn() -> Vec<Box<dyn Widget<Msg>>>>;
use crate::text::Text;
use crate::textinput::TextField;
use crate::theme::Theme;
use crate::widget::Widget;

/// Compares two text cells: **numerically** when both read as numbers, otherwise lexically
/// and **case-insensitively**. The basis of a [`DataTable`]'s sorting.
pub fn compare_cells(a: &str, b: &str) -> Ordering {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

/// `true` when a cell of `row` **contains** `query` (a **case-insensitive** substring). An
/// empty or blank query lets everything through. The basis of a [`DataTable`]'s filter;
/// reusable outside the widget (a reducer can filter its data the same way).
pub fn row_matches(row: &[String], query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    row.iter().any(|cell| cell.to_lowercase().contains(&q))
}

/// Returns a **copy** of `rows` sorted by column `col` (`ascending` = increasing) through
/// [`compare_cells`]. A missing cell (a row too short) counts as empty. Reusable outside the
/// widget: a reducer can sort its data in exactly the same way.
pub fn sort_rows(rows: &[Vec<String>], col: usize, ascending: bool) -> Vec<Vec<String>> {
    let empty = String::new();
    let mut out = rows.to_vec();
    out.sort_by(|a, b| {
        let ord = compare_cells(a.get(col).unwrap_or(&empty), b.get(col).unwrap_or(&empty));
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
    out
}

/// The number of pages for `len` rows cut into slices of `per_page` (at least **1**).
pub fn page_count(len: usize, per_page: usize) -> usize {
    let per = per_page.max(1);
    (len.div_ceil(per)).max(1)
}

/// The **slice** of rows of page `current` (1-indexed) with a size of `per_page`. The page is
/// brought back into `[1, page_count]` if it overflows. Reusable outside the widget.
pub fn page_rows(rows: &[Vec<String>], current: usize, per_page: usize) -> Vec<Vec<String>> {
    let per = per_page.max(1);
    let current = current.clamp(1, page_count(rows.len(), per));
    let start = (current - 1) * per;
    let end = (start + per).min(rows.len());
    rows[start..end].to_vec()
}

/// The "N–M of T" label of the current slice (`0 of 0` when empty) — milestone 236. Reusable.
pub fn page_range_label(current: usize, per_page: usize, total: usize) -> String {
    if total == 0 {
        return "0 of 0".to_string();
    }
    let per = per_page.max(1);
    let current = current.clamp(1, page_count(total, per));
    let start = (current - 1) * per + 1;
    let end = (current * per).min(total);
    format!("{start}\u{2013}{end} of {total}")
}

/// A **text** data table that sorts its own rows according to the sort state supplied, then
/// delegates the rendering to a [`Table`](crate::Table).
///
/// ```
/// use frus_widgets::DataTable;
/// let rows = vec![vec!["Bob".to_string(), "9".to_string()], vec!["Ada".to_string(), "10".to_string()]];
/// let table: DataTable = DataTable::new(["Name", "Score"], rows).sorted(1, true);
/// ```
pub struct DataTable<Msg = ()> {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    widths: Vec<f32>,
    sort: Option<(usize, bool)>,
    on_sort: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// Pagination: `(current page, 1-indexed; page size)` — milestone 233.
    page: Option<(usize, usize)>,
    on_page: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// The page sizes offered + the callback on change (the footer's selector) — milestone 236.
    page_sizes: Vec<usize>,
    on_page_size: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// Row selection (milestone 239): a callback on click plus the highlighted rows. The indices
    /// are those of the **source rows** (before sorting and pagination) — the `DataTable` does
    /// the translation against the displayed slice, as it already does for sorting and paging.
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    selected: Vec<usize>,
    /// A **custom** comparator per column (milestone 240): `None` = the default sort
    /// ([`compare_cells`]). It allows ordering cells the default sorts badly — formatted
    /// dates, amounts ("$1.2M"), priorities ("High"/"Medium"/"Low").
    comparators: Vec<Option<Comparator>>,
    /// **Multiple** selection (milestone 241): a column of checkboxes. `on_check(source_row)`
    /// toggles one row, and `on_check_all` toggles the header box. The checked state follows
    /// [`selected`](Self::selected), with the same source indices.
    on_check: Option<Rc<dyn Fn(usize) -> Msg>>,
    on_check_all: Option<Msg>,
    /// Search (milestone 242): the current query + a callback on typing. When `on_query` is set,
    /// a search field caps the table and the source rows are **filtered** ([`row_matches`])
    /// before sorting and pagination — all in source indices (selection and boxes unchanged).
    query: Option<String>,
    on_query: Option<Rc<dyn Fn(String) -> Msg>>,
    /// The **bulk actions** bar (milestone 243): a factory of action widgets (buttons…), called
    /// again on every rebuild. Rendered **above** the table **only** when rows are selected,
    /// preceded by an "N selected". The app wires up its buttons (variants, messages).
    bulk_actions: Option<BulkActions<Msg>>,
    /// The **empty state**'s text (milestone 244): shown centred, under the header, when no row
    /// is visible (empty data, or a filter with no result). Overridable through
    /// [`empty_text`](DataTable::empty_text).
    empty_text: String,
    /// The rendering: the `Table` (sorted and paginated rows), optionally capped by a footer
    /// (the slice label + `Pagination` + the size selector) beneath it.
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> DataTable<Msg> {
    /// Creates a table from its **headers** and its **rows** (text). Without a sort state, the
    /// rows are shown in the order supplied.
    pub fn new(
        headers: impl IntoIterator<Item = impl Into<String>>,
        rows: impl IntoIterator<Item = Vec<String>>,
    ) -> Self {
        let headers: Vec<String> = headers.into_iter().map(Into::into).collect();
        let cols = headers.len().max(1);
        let mut me = Self {
            headers,
            rows: rows.into_iter().collect(),
            widths: vec![0.0; cols],
            sort: None,
            on_sort: None,
            page: None,
            on_page: None,
            page_sizes: Vec::new(),
            on_page_size: None,
            on_select: None,
            selected: Vec::new(),
            comparators: Vec::new(),
            on_check: None,
            on_check_all: None,
            query: None,
            on_query: None,
            bulk_actions: None,
            empty_text: "No results".to_string(),
            inner: Box::new(Flex::<Msg>::column()),
        };
        me.rebuild();
        me
    }

    /// The **fixed** width of each column, in pixels (`0` or less = a flexible column).
    pub fn column_widths(mut self, widths: &[f32]) -> Self {
        for (i, w) in widths.iter().enumerate().take(self.widths.len()) {
            self.widths[i] = *w;
        }
        self.rebuild();
        self
    }

    /// The sorted column and its direction (`true` = ascending): it **sorts** the rows for
    /// display and shows the direction indicator.
    pub fn sorted(mut self, column: usize, ascending: bool) -> Self {
        self.sort = Some((column, ascending));
        self.rebuild();
        self
    }

    /// Makes the headers **clickable**: `on_sort(column)` on click (the application then flips
    /// the direction and passes `sorted(...)` back).
    pub fn on_sort(mut self, on_sort: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_sort = Some(Rc::new(on_sort));
        self.rebuild();
        self
    }

    /// **Paginates** the table: it shows only the slice of page `current` (1-indexed) with a
    /// size of `per_page`, and places a [`Pagination`](crate::Pagination) selector beneath it.
    /// `on_page(page)` when a page is clicked (the application updates `current`). The slicing
    /// follows the **sort**.
    pub fn paginated(
        mut self,
        current: usize,
        per_page: usize,
        on_page: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        self.page = Some((current.max(1), per_page.max(1)));
        self.on_page = Some(Rc::new(on_page));
        self.rebuild();
        self
    }

    /// Adds a **page size selector** (a `SegmentedButton` of the `sizes` offered) to the
    /// footer. `on_page_size(size)` on change (the app updates the size and, usually, returns
    /// to page 1). It has no effect if the table is not paginated.
    pub fn page_sizes(
        mut self,
        sizes: &[usize],
        on_page_size: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        self.page_sizes = sizes.iter().copied().filter(|&s| s > 0).collect();
        self.on_page_size = Some(Rc::new(on_page_size));
        self.rebuild();
        self
    }

    /// Makes the rows **clickable**: `on_select_row(source_row)` when a row is clicked. The
    /// index passed is that of the **source row** (before sorting and pagination) — the
    /// `DataTable` translates the displayed position into the original index.
    pub fn on_select_row(mut self, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self.rebuild();
        self
    }

    /// The **selected** rows (highlighted, or checked), designated by their **source row**
    /// index. The `DataTable` only highlights (and checks) those visible in the current
    /// slice.
    pub fn selected(mut self, rows: &[usize]) -> Self {
        self.selected = rows.to_vec();
        self.rebuild();
        self
    }

    /// Enables **multiple selection**: a column of checkboxes capped by a "check all".
    /// `on_check(source_row)` toggles one row (the **source row** index, translated from the
    /// displayed position), and `on_check_all` toggles the header box. The checked state
    /// reflects [`selected`](Self::selected). It combines with
    /// [`on_select_row`](Self::on_select_row): the box handles group selection, and a click
    /// on the row's body stays a row click.
    pub fn checkboxes(
        mut self,
        on_check: impl Fn(usize) -> Msg + 'static,
        on_check_all: Msg,
    ) -> Self {
        self.on_check = Some(Rc::new(on_check));
        self.on_check_all = Some(on_check_all);
        self.rebuild();
        self
    }

    /// Makes the table **searchable**: a search field (with value `query`) caps the table, and
    /// the source rows are **filtered** ([`row_matches`], a case-insensitive substring across
    /// every column) before sorting and pagination. `on_query(text)` on each keystroke (the
    /// application updates `query` and usually returns to page 1). The filter acts upstream
    /// of the sort, the page **and** the selection: boxes and highlighting stay in source
    /// indices, over the visible subset.
    pub fn searchable(
        mut self,
        query: impl Into<String>,
        on_query: impl Fn(String) -> Msg + 'static,
    ) -> Self {
        self.query = Some(query.into());
        self.on_query = Some(Rc::new(on_query));
        self.rebuild();
        self
    }

    /// Adds a **bulk actions bar** above the table, visible **only** when rows are
    /// [`selected`](Self::selected). The `make` factory produces the action widgets (typically
    /// [`Button`](crate::Button)s — the app chooses variants and messages); the bar precedes
    /// them with an "N selected" label. Called again on every rebuild (fresh widgets). The
    /// number shown is that of the **selected** rows, across all pages.
    pub fn bulk_actions(mut self, make: impl Fn() -> Vec<Box<dyn Widget<Msg>>> + 'static) -> Self {
        self.bulk_actions = Some(Rc::new(make));
        self.rebuild();
        self
    }

    /// Overrides the **empty state**'s text (default "No results") — "No people match your
    /// search", say. Shown centred under the header when no row is visible (milestone 244).
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self.rebuild();
        self
    }

    /// Gives column `col` a **custom comparator**: `cmp(a, b)` orders two of its (text) cells.
    /// It replaces the default sort ([`compare_cells`]) for that column — useful when the
    /// values sort badly as they stand: formatted dates ("Mar 2024"), amounts ("$1.2M"),
    /// priorities ("High"/"Medium"/"Low"). The direction (`sorted(_, ascending)`) applies on
    /// top (the comparator defines the **ascending** order).
    pub fn sort_with(mut self, col: usize, cmp: impl Fn(&str, &str) -> Ordering + 'static) -> Self {
        if self.comparators.len() <= col {
            self.comparators.resize_with(col + 1, || None);
        }
        self.comparators[col] = Some(Rc::new(cmp));
        self.rebuild();
        self
    }

    /// The order of the **source row indices** after **filtering** (search) then sorting
    /// (stable); the filtered identity when there is no sort. The sort uses the column's
    /// **custom** comparator when it has one, otherwise [`compare_cells`].
    fn sorted_order(&self) -> Vec<usize> {
        // The search filter, upstream: it keeps only the matching source rows.
        let mut order: Vec<usize> = (0..self.rows.len())
            .filter(|&i| match &self.query {
                Some(q) => row_matches(&self.rows[i], q),
                None => true,
            })
            .collect();
        if let Some((col, asc)) = self.sort {
            let empty = String::new();
            let custom = self.comparators.get(col).and_then(|c| c.as_ref());
            order.sort_by(|&a, &b| {
                let (x, y) = (
                    self.rows[a].get(col).unwrap_or(&empty),
                    self.rows[b].get(col).unwrap_or(&empty),
                );
                let ord = match custom {
                    Some(cmp) => cmp(x, y),
                    None => compare_cells(x, y),
                };
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }
        order
    }

    /// (Re)builds the rendering: rows sorted according to `sort`, cut to page `page` where
    /// applicable, in a `Table` (headers, widths, indicator) optionally capped by a
    /// `Pagination` beneath.
    fn rebuild(&mut self) {
        // The reasoning is in terms of **source row indices** (sorted, then sliced): that
        // preserves each row's original identity through the sort and the pagination, so the
        // selection can be translated (displayed index ↔ source index) both ways.
        let order = self.sorted_order();
        let total = order.len();
        let page_indices: Vec<usize> = match self.page {
            Some((current, per)) => {
                let per = per.max(1);
                let current = current.clamp(1, page_count(total, per));
                let start = (current - 1) * per;
                let end = (start + per).min(total);
                order[start..end].to_vec()
            }
            None => order.clone(),
        };
        let hrefs: Vec<&str> = self.headers.iter().map(|s| s.as_str()).collect();
        let mut t = Table::new(self.headers.len().max(1)).header(&hrefs);
        for &i in &page_indices {
            let refs: Vec<&str> = self.rows[i].iter().map(|s| s.as_str()).collect();
            t = t.row(&refs);
        }
        if self.widths.iter().any(|w| *w > 0.0) {
            t = t.column_widths(&self.widths);
        }
        if let Some(f) = &self.on_sort {
            let f = f.clone();
            t = t.on_sort(move |c| f(c));
        }
        if let Some((col, asc)) = self.sort {
            t = t.sorted(col, asc);
        }
        // Selection: `Table` reasons in **displayed positions** (0..slice). The click is wired
        // to return the **source** index, and the selected rows are translated source → position.
        if let Some(f) = &self.on_select {
            let f = f.clone();
            let indices = page_indices.clone();
            t = t.on_select_row(move |d| f(indices.get(d).copied().unwrap_or(d)));
        }
        // Multiple selection: the same displayed position → source index translation for the boxes.
        if let (Some(f), Some(all)) = (&self.on_check, &self.on_check_all) {
            let f = f.clone();
            let indices = page_indices.clone();
            let all = all.clone();
            t = t.checkboxes(move |d| f(indices.get(d).copied().unwrap_or(d)), all);
        }
        if !self.selected.is_empty() {
            let display_sel: Vec<usize> = page_indices
                .iter()
                .enumerate()
                .filter(|(_, orig)| self.selected.contains(orig))
                .map(|(d, _)| d)
                .collect();
            if !display_sel.is_empty() {
                t = t.selected(&display_sel);
            }
        }
        // The empty state (empty data, or a filter with no result): header + a centred message,
        // **without** a footer (a "0 of 0" pager under an empty body adds nothing). The
        // message is overridable.
        let block: Box<dyn Widget<Msg>> = if total == 0 {
            let message = Text::new(self.empty_text.clone()).size(15.0);
            let empty = Flex::column()
                .align(Align::Center)
                .padding(24.0)
                .child(message);
            Box::new(Flex::column().gap(8.0).child(t).child(empty))
        } else {
            match (self.page, &self.on_page) {
                (Some((current, per)), Some(on_page)) => {
                    let pages = page_count(total, per);
                    let current = current.clamp(1, pages);
                    // The "N–M of T" label of the current slice (milestone 236).
                    let label = Text::new(page_range_label(current, per, total)).size(13.0);
                    let on_page = on_page.clone();
                    let pager = Pagination::new(current, pages, move |p| on_page(p));
                    let mut footer = Flex::row()
                        .align(Align::Center)
                        .gap(12.0)
                        .child(label)
                        .child(Flex::row().flex(1.0))
                        .child(pager);
                    // The page size selector, when offered (milestone 236).
                    if let (Some(on_size), false) = (&self.on_page_size, self.page_sizes.is_empty())
                    {
                        let sizes = self.page_sizes.clone();
                        let sel = sizes.iter().position(|&s| s == per).unwrap_or(0);
                        let on_size = on_size.clone();
                        // A row of numbers in a table's footer: the checkmark would take
                        // more room than the digits it sits beside, and which one is chosen
                        // is already plain from the fill.
                        let mut seg = SegmentedButton::new(sel, move |i| on_size(sizes[i]))
                            .show_selected_icon(false)
                            .padding(10.0);
                        for s in &self.page_sizes {
                            seg = seg.segment(s.to_string());
                        }
                        footer = footer.child(seg);
                    }
                    Box::new(Flex::column().gap(12.0).child(t).child(footer))
                }
                _ => Box::new(t),
            }
        };
        // Bulk actions: a bar above the table, only when a selection exists.
        let mut block = block;
        if let Some(make) = &self.bulk_actions {
            if !self.selected.is_empty() {
                let label = Text::new(format!("{} selected", self.selected.len())).size(14.0);
                let mut bar = Flex::row()
                    .align(Align::Center)
                    .gap(8.0)
                    .child(label)
                    .child(Flex::row().flex(1.0));
                for w in make() {
                    bar = bar.child(w);
                }
                block = Box::new(Flex::column().gap(12.0).child(bar).child(block));
            }
        }
        // Search: it caps the table with a field (otherwise the block is kept as is).
        self.inner = if let Some(on_query) = &self.on_query {
            let on_query = on_query.clone();
            let field = TextField::new(self.query.clone().unwrap_or_default())
                .placeholder("Search")
                .width(240.0)
                .on_input(move |s| on_query(s));
            Box::new(Flex::column().gap(12.0).child(field).child(block))
        } else {
            block
        };
    }
}

impl<Msg: Clone> Widget<Msg> for DataTable<Msg> {
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

    fn stack(&self) -> bool {
        self.inner.stack()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Vec<String>> {
        vec![
            vec!["Bob".to_string(), "9".to_string()],
            vec!["alice".to_string(), "10".to_string()],
            vec!["Carol".to_string(), "2".to_string()],
        ]
    }

    #[test]
    fn sort_rows_is_numeric_aware_and_case_insensitive() {
        let rows = sample();
        // Column 1 is **numeric**: 2 < 9 < 10 (not the lexical order "10" < "2" < "9").
        let by_num = sort_rows(&rows, 1, true);
        assert_eq!(
            by_num.iter().map(|r| r[1].as_str()).collect::<Vec<_>>(),
            ["2", "9", "10"]
        );
        // Column 0 is **text**, case-insensitive: alice < Bob < Carol.
        let by_name = sort_rows(&rows, 0, true);
        assert_eq!(
            by_name.iter().map(|r| r[0].as_str()).collect::<Vec<_>>(),
            ["alice", "Bob", "Carol"]
        );
        // **Descending** direction: the order is reversed.
        let desc = sort_rows(&rows, 1, false);
        assert_eq!(
            desc.iter().map(|r| r[1].as_str()).collect::<Vec<_>>(),
            ["10", "9", "2"]
        );
    }

    #[test]
    fn compare_cells_prefers_numbers_then_text() {
        assert_eq!(compare_cells("2", "10"), Ordering::Less, "numeric: 2 < 10");
        assert_eq!(
            compare_cells("Bob", "alice"),
            Ordering::Greater,
            "text: b > a (case-insensitive)"
        );
    }

    #[test]
    fn data_table_builds_a_non_empty_tree() {
        let dt = DataTable::<()>::new(["Name", "Score"], sample())
            .column_widths(&[120.0, 80.0])
            .sorted(1, true)
            .on_sort(|_| ());
        // Header + rows → a non-empty render tree.
        assert!(
            !Widget::<()>::children(&dt).is_empty(),
            "the DataTable produces a tree"
        );
    }

    #[test]
    fn pagination_slices_rows_and_counts_pages() {
        let rows: Vec<Vec<String>> = (1..=7).map(|i| vec![i.to_string()]).collect();
        assert_eq!(page_count(7, 3), 3, "7 rows / 3 = 3 pages");
        assert_eq!(page_count(0, 3), 1, "at least one page");
        let col0 = |rs: Vec<Vec<String>>| rs.iter().map(|r| r[0].clone()).collect::<Vec<_>>();
        // Page 1: 1,2,3; page 3: 7 (the last, partial one).
        assert_eq!(col0(page_rows(&rows, 1, 3)), ["1", "2", "3"]);
        assert_eq!(col0(page_rows(&rows, 3, 3)), ["7"]);
        // An out-of-bounds page is brought back into the interval.
        assert_eq!(col0(page_rows(&rows, 99, 3)), ["7"]);
    }

    #[test]
    fn data_table_with_pagination_builds_table_and_pager() {
        let rows: Vec<Vec<String>> = (1..=10)
            .map(|i| vec![format!("R{i}"), i.to_string()])
            .collect();
        let dt = DataTable::<()>::new(["Name", "Score"], rows)
            .sorted(1, true)
            .paginated(1, 4, |_| ());
        // inner = the column [table, footer] → two children.
        assert_eq!(
            Widget::<()>::children(&dt).len(),
            2,
            "table + pied de pagination"
        );
    }

    #[test]
    fn page_range_label_describes_the_slice() {
        assert_eq!(page_range_label(1, 3, 7), "1\u{2013}3 of 7");
        assert_eq!(page_range_label(2, 3, 7), "4\u{2013}6 of 7");
        assert_eq!(
            page_range_label(3, 3, 7),
            "7\u{2013}7 of 7",
            "the last, partial page"
        );
        assert_eq!(page_range_label(1, 3, 0), "0 of 0", "empty");
        assert_eq!(
            page_range_label(99, 3, 7),
            "7\u{2013}7 of 7",
            "an out-of-bounds page is brought back"
        );
    }

    /// Collects, in tree order, every non-null `on_click` message of a subtree.
    fn collect_clicks(w: &dyn Widget<usize>, out: &mut Vec<usize>) {
        if let Some(m) = w.on_click() {
            out.push(m);
        }
        for c in w.children() {
            collect_clicks(c.as_ref(), out);
        }
    }

    #[test]
    fn selection_click_reports_the_source_row_through_sort_and_page() {
        // Three rows, with a numeric key in column 1. Ascending sort → source order [1, 2, 0].
        let rows = vec![
            vec!["A".to_string(), "3".to_string()],
            vec!["B".to_string(), "1".to_string()],
            vec!["C".to_string(), "2".to_string()],
        ];
        let make = |page: usize| -> DataTable<usize> {
            DataTable::new(["N", "K"], rows.clone())
                .sorted(1, true)
                .paginated(page, 2, |_| 0)
                .on_select_row(|i| i)
        };
        // `children()[0]` = the Table; `[1]` = the footer (the pager), which is ignored.
        let clicks_of = |dt: &DataTable<usize>| {
            let mut v = Vec::new();
            collect_clicks(Widget::<usize>::children(dt)[0].as_ref(), &mut v);
            v.dedup(); // one clickable cell per column → consecutive repeats
            v
        };
        // Page 1 (size 2) of the sorted rows [1, 2, 0]: the click returns **source** index 1, then 2.
        assert_eq!(
            clicks_of(&make(1)),
            vec![1, 2],
            "the click returns the source row's index"
        );
        // Page 2: the last sorted row, source index 0 — pagination does not alter identity.
        assert_eq!(
            clicks_of(&make(2)),
            vec![0],
            "the translation survives pagination"
        );
    }

    #[test]
    fn empty_filter_drops_rows_and_pager() {
        // Two rows, with a "zzz" filter that matches nothing → the empty state: no clickable row,
        // and the **pagination footer is removed** (otherwise its single page button would
        // emit `0`).
        let rows = vec![vec!["Ada".to_string()], vec!["Bob".to_string()]];
        let dt: DataTable<usize> = DataTable::new(["N"], rows)
            .searchable("zzz", |_| 0)
            .paginated(1, 5, |_| 0)
            .on_select_row(|i| i);
        let mut clicks = Vec::new();
        for c in Widget::<usize>::children(&dt) {
            collect_clicks(c.as_ref(), &mut clicks);
        }
        assert!(
            clicks.is_empty(),
            "neither a clickable row nor a pager when the filter matches nothing"
        );
        // The tree stays non-empty: the header + the empty-state message.
        assert!(
            !Widget::<usize>::children(&dt).is_empty(),
            "header + message rendered"
        );
    }

    #[test]
    fn bulk_actions_bar_shows_only_with_a_selection() {
        use crate::button::Button;
        let rows = vec![vec!["A".to_string()], vec!["B".to_string()]];
        // `777` = the message of one bar action (a sentinel).
        let make = |sel: &[usize]| -> DataTable<usize> {
            DataTable::new(["N"], rows.clone())
                .checkboxes(|i| i, 900usize)
                .selected(sel)
                .bulk_actions(|| vec![Box::new(Button::new("Clear").on_press(777usize)) as Box<dyn Widget<usize>>])
        };
        let has_action = |dt: &DataTable<usize>| {
            let mut v = Vec::new();
            for c in Widget::<usize>::children(dt) {
                collect_clicks(c.as_ref(), &mut v);
            }
            v.contains(&777)
        };
        assert!(!has_action(&make(&[])), "no bar without a selection");
        assert!(
            has_action(&make(&[0])),
            "the action bar appears as soon as a row is selected"
        );
    }

    #[test]
    fn row_matches_is_case_insensitive_substring_over_all_cells() {
        let row = vec!["Ada Lovelace".to_string(), "Engineer".to_string()];
        assert!(
            row_matches(&row, ""),
            "an empty query lets everything through"
        );
        assert!(
            row_matches(&row, "  "),
            "a blank query lets everything through"
        );
        assert!(row_matches(&row, "ENGIN"), "case-insensitive, a substring");
        assert!(row_matches(&row, "love"), "another column");
        assert!(!row_matches(&row, "zzz"), "no match");
    }

    #[test]
    fn search_filters_rows_before_sort_and_keeps_source_indices() {
        // Four rows; searching for "a" → only Ada and Cal match (source indices 0 and 2).
        let rows = vec![
            vec!["Ada".to_string(), "3".to_string()],
            vec!["Bob".to_string(), "1".to_string()],
            vec!["Cal".to_string(), "2".to_string()],
            vec!["Eve".to_string(), "4".to_string()],
        ];
        // An ascending sort by key among {Ada(3), Cal(2)} → Cal(2), Ada(3) = source indices [2, 0].
        let dt: DataTable<usize> = DataTable::new(["N", "K"], rows)
            .searchable("a", |_| 0)
            .sorted(1, true)
            .on_select_row(|i| i);
        let mut clicks = Vec::new();
        for c in Widget::<usize>::children(&dt) {
            collect_clicks(c.as_ref(), &mut clicks);
        }
        clicks.dedup();
        assert_eq!(
            clicks,
            vec![2, 0],
            "filtered then sorted, in source indices"
        );
    }

    #[test]
    fn checkbox_click_reports_the_source_row_through_sort_and_page() {
        // The same data as the single selection: ascending sort → source order [1, 2, 0].
        let rows = vec![
            vec!["A".to_string(), "3".to_string()],
            vec!["B".to_string(), "1".to_string()],
            vec!["C".to_string(), "2".to_string()],
        ];
        // `999` = the "check all" box's message (a sentinel, filtered out).
        let dt: DataTable<usize> = DataTable::new(["N", "K"], rows)
            .sorted(1, true)
            .paginated(2, 2, |_| 0)
            .checkboxes(|i| i, 999);
        let mut v = Vec::new();
        // `children()[0]` = the Table; `[1]` = the footer (the pager), ignored.
        collect_clicks(Widget::<usize>::children(&dt)[0].as_ref(), &mut v);
        v.retain(|&m| m != 999); // removes the header box
        v.dedup();
        // Page 2 (size 2) of the sorted rows [1, 2, 0] → the box returns **source** index 0.
        assert_eq!(
            v,
            vec![0],
            "the box returns the source row's index, pagination included"
        );
    }

    #[test]
    fn custom_comparator_orders_a_column_semantically() {
        // A **priority** column that a text sort would order alphabetically
        // (High < Low < Medium) — semantically wrong. A home-made comparator imposes
        // Low < Medium < High.
        let rows = vec![
            vec!["A".to_string(), "High".to_string()],
            vec!["B".to_string(), "Low".to_string()],
            vec!["C".to_string(), "Medium".to_string()],
        ];
        let rank = |s: &str| match s {
            "Low" => 0,
            "Medium" => 1,
            "High" => 2,
            _ => 3,
        };
        let dt: DataTable<usize> = DataTable::new(["N", "Prio"], rows)
            .sorted(1, true)
            .sort_with(1, move |a, b| rank(a).cmp(&rank(b)))
            .on_select_row(|i| i);
        let mut clicks = Vec::new();
        for c in Widget::<usize>::children(&dt) {
            collect_clicks(c.as_ref(), &mut clicks);
        }
        clicks.dedup();
        // The semantic ascending order Low(1) < Medium(2) < High(0) → source indices [1, 2, 0],
        // not the alphabetical order [High(0), Low(1), Medium(2)] of the default sort.
        assert_eq!(
            clicks,
            vec![1, 2, 0],
            "the custom comparator orders by priority"
        );
    }

    #[test]
    fn page_size_selector_appears_in_the_footer() {
        let rows: Vec<Vec<String>> = (1..=7).map(|i| vec![i.to_string()]).collect();
        // inner = [table, footer]; the footer is a Flex row [label, spacer, pager, (selector)].
        let footer_len = |dt: &DataTable<()>| Widget::<()>::children(dt)[1].children().len();
        let base = DataTable::<()>::new(["N"], rows.clone()).paginated(1, 3, |_| ());
        let sized = DataTable::<()>::new(["N"], rows)
            .paginated(1, 3, |_| ())
            .page_sizes(&[3, 5], |_| ());
        assert_eq!(footer_len(&base), 3, "label + spacer + pager");
        assert_eq!(footer_len(&sized), 4, "+ the size selector");
    }
}
