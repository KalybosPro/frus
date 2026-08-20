//! The data-table screen: sorting, selection, paging.

use crate::prelude::*;
use crate::screens::todo::*;
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

/// The **data table** screen: a read-only `DataTable` that **sorts its own rows** (milestone
/// 232) and **paginates** with a page-size selector (milestones 233/236). The app only keeps
/// the `(sort, page, size)` state — the display sort is **not** duplicated in the reducer (a
/// deliberate contrast with the editable grid next door). — milestone 237.
pub(crate) fn data_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Box<dyn Widget<Msg>> {
    let people = app.data_rows();
    let rows: Vec<Vec<String>> = people
        .iter()
        .map(|(n, r, s, l)| vec![n.clone(), r.clone(), s.to_string(), l.clone()])
        .collect();
    // `0` (the derived default) = sensible starting values.
    let per = if app.data_page_size == 0 {
        5
    } else {
        app.data_page_size
    };
    let page = app.data_page.max(1);
    let mut table: DataTable<Msg> = DataTable::new(["Name", "Role", "Score", "Level"], rows)
        .column_widths(&[200.0, 170.0, 90.0, 110.0])
        // The "Level" column: a **custom** sort key (Low < Medium < High), since a text sort
        // would order it alphabetically (milestone 240).
        .sort_with(3, |a, b| level_rank(a).cmp(&level_rank(b)))
        // Search: filters the source rows (every column) before sorting/pagination (milestone 242).
        .searchable(app.data_query.as_str(), Msg::DataSearch)
        // An overridden empty state (milestone 244): a message when the filter/the data show nothing.
        .empty_text("No people match your search")
        .on_sort(Msg::DataSort)
        .on_select_row(Msg::DataSelectRow)
        // Multi-selection (milestone 241): checkboxes for a bulk selection, on top of the row
        // click (focus). The checked rows drive the highlighting through `selected`.
        .checkboxes(Msg::DataCheck, Msg::DataCheckAll)
        .selected(&app.data_checked)
        // The bulk-actions bar (milestone 243): visible when rows are checked.
        .bulk_actions(|| {
            vec![
                Box::new(
                    button("Clear", Msg::DataClearChecked)
                        .variant(Variant::Outlined)
                        .size(14.0),
                ) as Box<dyn Widget<Msg>>,
                Box::new(
                    button("Delete", Msg::DataAskDelete)
                        .variant(Variant::Danger)
                        .size(14.0),
                ),
            ]
        })
        .paginated(page, per, Msg::DataPage)
        .page_sizes(&[5, 10], Msg::DataPageSize);
    if let Some((col, asc)) = app.data_sort {
        table = table.sorted(col, asc);
    }
    let hint =
        text("Check rows for bulk actions; click a row to focus it; click a header to sort.")
            .size(13.0)
            .color(theme.muted)
            .wrap();
    // Detail of the **focused** row (a click on the row's body): read from the current data.
    let detail = match app.data_selected.and_then(|i| people.get(i)) {
        Some((n, r, s, l)) => text(format!("Focused: {n} — {r} (score {s}, {l} priority)"))
            .size(15.0)
            .color(theme.on_surface)
            .wrap(),
        None => text("No row focused.").size(15.0).color(theme.muted).wrap(),
    };
    // A summary of the **bulk** selection (the checked boxes).
    let summary = text(format!("{} checked", app.data_checked.len()))
        .size(13.0)
        .color(theme.muted);
    // The table (fixed columns, ~610 px) is wider than a phone: a bounded **scrollable** region
    // (columns in X, rows in Y) — not a page pan, a scrollable table.
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
    let screen = column![NavigationBar::new("Data table").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    let content = Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen);
    // A confirmation before the bulk delete (milestone 245): a centred modal, dismissible by an
    // outside click or Escape (`dismiss`), laid over the screen.
    if app.data_confirm_delete {
        Box::new(
            OverlayPortal::new(content)
                .overlay(
                    data_confirm_content(app.data_checked.len()),
                    Placement::Center,
                )
                .dismiss(Msg::DataCancelDelete),
        )
    } else {
        Box::new(content)
    }
}
