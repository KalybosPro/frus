//! The **state**, and the questions worth asking it.
//!
//! Derived answers (`active_count`, `current_route`) live here rather than in the
//! views that want them: computed next to the state, they cannot drift into three
//! slightly different versions of the same count.

use crate::prelude::*;

/// One task of the list.
pub(crate) struct Todo {
    pub(crate) id: u64,
    pub(crate) text: String,
    pub(crate) done: bool,
}

/// Display filter of the task list.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub(crate) enum Filter {
    #[default]
    All,
    Active,
    Done,
}

/// The application's screens.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    Home,
    Settings,
    Journal,
    /// Multi-step sign-up wizard (an integration demo: Steps + Form + Snackbar).
    Wizard,
    /// **Inline-editable** data grid (an integration demo: Table + one TextInput per cell).
    Grid,
    /// **Chart** dashboard (an integration demo: LineChart + a clickable legend, milestone 218).
    Charts,
    /// **Read-only** data table (an integration demo: a self-sorting, paginated DataTable —
    /// milestone 237).
    Data,
    /// A **Kanban** board: columns of cards, drag-and-drop between columns (milestone 247).
    Board,
    /// A paged **walkthrough**: one panel per swipe, dots, and buttons that drive the
    /// same view the finger does (milestone 283).
    Tour,
    /// One task, on its own screen. Its avatar is a **shared element** with the row it
    /// was opened from, and flies between the two (milestone 286).
    Task(u64),
}

/// The back gesture: the progress follows the finger, then a spring settle (commit/cancel)
/// driven by an [`AnimationController`].
pub(crate) struct BackGesture {
    pub(crate) progress: f32,
    pub(crate) velocity: f32,
    /// `Some` once released: the spring settle in progress (`None` = still following the
    /// finger).
    pub(crate) settle: Option<AnimationController>,
    /// Does the settle, when it ends, commit the back (a pop)?
    pub(crate) commit: bool,
}

/// The to-do application: state + logic. A consumer of the `frus-shell` framework.
#[derive(Default)]
pub(crate) struct TodoApp {
    /// The tasks, in the order they were added.
    pub(crate) todos: Vec<Todo>,
    /// The text currently being typed.
    pub(crate) draft: String,
    /// The current filter.
    pub(crate) filter: Filter,
    /// The next task id.
    pub(crate) next_id: u64,
    /// Is the "clear completed" confirmation modal open?
    pub(crate) confirm_clear: bool,
    /// A light theme (otherwise dark).
    pub(crate) light: bool,
    /// The outgoing theme during a switch fade (`None` = no transition).
    pub(crate) theme_from: Option<Theme>,
    /// Progress of the theme fade (`0 → 1`).
    pub(crate) theme_progress: f32,
    /// Does the log list bounce at its ends rather than stop dead? `false` leaves
    /// it on the platform's own behaviour.
    pub(crate) journal_bounces: bool,
    /// Is the log list reloading? The pull-to-refresh indicator spins for exactly as
    /// long as this is true — the application owns the answer, not the framework.
    pub(crate) journal_reloading: f32,
    /// How many times the log has been reloaded, so a completed pull leaves a trace.
    pub(crate) journal_reloads: usize,
    /// The walkthrough's page. The application owns it: the finger reports its changes
    /// here, and the buttons write to it, so both drive the same one value.
    pub(crate) tour_page: usize,
    /// The screen stack (empty = the home screen).
    pub(crate) routes: Vec<Route>,
    /// The outgoing screen during a transition.
    pub(crate) nav_from: Option<Route>,
    /// Progress of the screen transition (`0 → 1`), driven by a spring.
    pub(crate) nav: AnimationController,
    pub(crate) nav_forward: bool,
    /// The back gesture in progress.
    pub(crate) back: Option<BackGesture>,
    // --- Controls of the Settings screen ---
    pub(crate) notifs: bool,
    pub(crate) volume: f32,
    pub(crate) radio: usize,
    pub(crate) menu_open: bool,
    pub(crate) menu_choice: usize,
    // --- Stopwatch (the timer subscription) ---
    /// Is the stopwatch running? (it drives the `every` subscription).
    pub(crate) running: bool,
    /// Is the app in the **background**? (the lifecycle, milestone 259) — set to `true` on
    /// `Paused`/`Detached` through [`Application::on_lifecycle`], at which point the timer
    /// suspends. `false` by default (the foreground) — which suits `#[derive(Default)]`.
    pub(crate) background: bool,
    /// Seconds elapsed since the stopwatch started.
    pub(crate) elapsed: u32,
    /// The Settings screen's active tab.
    pub(crate) settings_tab: usize,
    /// Is the (header) actions menu open?
    pub(crate) actions_open: bool,
    /// Is the "Advanced options" section expanded?
    pub(crate) advanced_open: bool,
    /// The star rating (Settings).
    pub(crate) rating: u32,
    /// The stepper's counter (Settings).
    pub(crate) count: i32,
    /// The notification (Snackbar) queue: one at a time, with an animated exit (milestone 193).
    pub(crate) snackbars: SnackbarQueue<String>,
    /// The pagination selector's current page (a demo).
    pub(crate) page: usize,
    /// The expanded tree nodes (the Tree demo).
    pub(crate) expanded: std::collections::HashSet<u64>,
    /// The selected tree node (the Tree demo, milestone 246); `None` = none.
    pub(crate) tree_selected: Option<u64>,
    /// The colour picked (the ColorPicker demo).
    pub(crate) picked: Option<Color>,
    /// The calendar: year / month (1..12) / selected day.
    pub(crate) year: i32,
    pub(crate) month: u32,
    pub(crate) selected_day: Option<u32>,
    /// The showcase calendar: should weekends be disabled (`DatePicker::filtered`)? — milestone 238.
    pub(crate) weekdays_only: bool,
    /// The carousel's current slide (a demo).
    pub(crate) slide: usize,
    /// Is the info popover open?
    pub(crate) info_open: bool,
    /// What is typed in the autocomplete (a demo).
    pub(crate) tag_draft: String,
    /// Section active de l'accueil (0 = Tasks, 1 = Stats, 2 = About) — NavScaffold.
    pub(crate) section: usize,
    /// The metric selected in the Stats section (a master-detail TwoPane).
    pub(crate) stat_sel: usize,
    /// In single-pane (narrow) mode, is the Stats detail open?
    pub(crate) stat_detail_open: bool,
    /// Density (an application-level zoom over the whole UI) — `1.0` by default.
    pub(crate) density: f32,
    /// The current size class (updated by `on_resize`).
    pub(crate) size_class: Option<SizeClass>,
    /// The current orientation (updated by `on_resize`).
    pub(crate) orientation: Option<Orientation>,
    /// Is the side navigation drawer open?
    pub(crate) drawer_open: bool,
    /// Is the quick-actions modal sheet open?
    pub(crate) sheet_open: bool,
    /// System insets (the safe area): status/navigation bars, notches.
    pub(crate) insets: Insets,
    /// The theme seed: `0` = the hand-written scheme, otherwise `from_seed` (HCT).
    pub(crate) seed_index: usize,
    /// A right-to-left layout (Arabic/Hebrew)?
    pub(crate) rtl: bool,
    /// The current language (an index into `LANGS`).
    pub(crate) lang: usize,
    /// The state came from a live-reload snapshot: `init` does not reload the tasks from disk
    /// (the snapshot is the authority).
    pub(crate) restored: bool,
    // --- Sign-up wizard (an integration demo) ---
    /// The wizard's current step (0 = Account, 1 = Security, 2 = Review).
    pub(crate) wizard_step: usize,
    pub(crate) wizard_name: String,
    pub(crate) wizard_email: String,
    pub(crate) wizard_pass: String,
    pub(crate) wizard_confirm: String,
    /// Has the wizard been submitted at least once? (the errors only show afterwards.)
    pub(crate) wizard_submitted: bool,
    /// Are the wizard's passwords **revealed** (unmasked)?
    pub(crate) wizard_reveal: bool,
    /// The editable grid's data (rows × columns of text).
    pub(crate) grid: Vec<Vec<String>>,
    /// The grid's current sort: `(column, ascending)`; `None` = the order it was typed in.
    pub(crate) grid_sort: Option<(usize, bool)>,
    /// The last faulty cell "Next error" targeted (milestone 214) — so it can cycle to the next.
    pub(crate) grid_error_cursor: Option<(usize, usize)>,
    /// Indices of the chart's **hidden** series (toggled through the legend, milestone 218).
    pub(crate) chart_hidden: Vec<usize>,
    /// The kind of chart shown: 0 lines, 1 stacked areas, 2 grouped bars, 3 stacked bars (milestone 219).
    pub(crate) chart_kind: usize,
    /// The **pinned** detail of a clicked point (`series · category = value`), if there is one (milestone 221).
    pub(crate) chart_pin: Option<String>,
    /// The **selected** point/bar `(category, series)`, highlighted in the chart (milestone 223).
    pub(crate) chart_sel: Option<(usize, usize)>,
    /// **100%** stacking (proportions) turned on for the stacked charts (milestone 224).
    pub(crate) chart_normalized: bool,
    /// The data table's sort `(column, ascending)`; `None` = the source order (milestone 237).
    /// The display sort is done by the `DataTable`, **not** duplicated here.
    pub(crate) data_sort: Option<(usize, bool)>,
    /// The data table's current page (1-indexed); `0` = page 1 (milestone 237).
    pub(crate) data_page: usize,
    /// The data table's page size; `0` = the default (milestone 237).
    pub(crate) data_page_size: usize,
    /// The data table's selected **source** row; `None` = none (milestone 239). The `DataTable`
    /// translates that original index into a highlighted position through sorting/pagination.
    pub(crate) data_selected: Option<usize>,
    /// The data table's checked **source** rows (multi-selection, milestone 241). It drives the
    /// boxes and the highlighting; the app decides what "check all" covers (here, all 12 rows).
    pub(crate) data_checked: Vec<usize>,
    /// The data table's search query (milestone 242); the `DataTable` filters the display.
    pub(crate) data_query: String,
    /// The data table's rows (milestone 243): `None` = the `DATA_PEOPLE` starting set; `Some` as
    /// soon as a bulk action (Delete) changes them. See [`TodoApp::data_rows`].
    pub(crate) data_rows: Option<Vec<(String, String, u32, String)>>,
    /// Is the data table's bulk-delete confirmation modal open? (milestone 245)
    pub(crate) data_confirm_delete: bool,
    /// The Kanban cards, per column (milestone 247); `None` = the `KANBAN_SEED` starting set.
    /// `Some` as soon as a card is moved. See [`TodoApp::kanban_cols`].
    pub(crate) kanban: Option<Vec<Vec<String>>>,
}

pub(crate) fn current_route(app: &TodoApp) -> Route {
    app.routes.last().copied().unwrap_or(Route::Home)
}

impl TodoApp {
    /// The data table's current rows: the ones held in the state if they have been changed (a
    /// bulk Delete), otherwise the `DATA_PEOPLE` starting set (milestone 243).
    pub(crate) fn data_rows(&self) -> Vec<(String, String, u32, String)> {
        match &self.data_rows {
            Some(rows) => rows.clone(),
            None => DATA_PEOPLE
                .iter()
                .map(|(n, r, s, l)| (n.to_string(), r.to_string(), *s, l.to_string()))
                .collect(),
        }
    }

    /// The Kanban cards per column: the ones held in the state if any have moved, otherwise the
    /// `KANBAN_SEED` starting set (milestone 247).
    pub(crate) fn kanban_cols(&self) -> Vec<Vec<String>> {
        match &self.kanban {
            Some(cols) => cols.clone(),
            None => KANBAN_SEED
                .iter()
                .map(|col| col.iter().map(|s| s.to_string()).collect())
                .collect(),
        }
    }
}

/// A filter's index (for the segmented control).
pub(crate) fn filter_index(filter: Filter) -> usize {
    match filter {
        Filter::All => 0,
        Filter::Active => 1,
        Filter::Done => 2,
    }
}

/// The filter matching a segment index.
pub(crate) fn filter_from_index(index: usize) -> Filter {
    match index {
        1 => Filter::Active,
        2 => Filter::Done,
        _ => Filter::All,
    }
}

/// Number of tasks that are not done.
pub(crate) fn active_count(app: &TodoApp) -> usize {
    app.todos.iter().filter(|t| !t.done).count()
}

/// Number of tasks that are done.
pub(crate) fn done_count(app: &TodoApp) -> usize {
    app.todos.iter().filter(|t| t.done).count()
}

// --- The demo's fixed data, and the validation that reads it ---
//
// These sat in the screens that happened to draw them, until the application was
// split across files and the direction became visible: `update` was importing a
// view module to read a dataset. A dataset is not a view, and validating a cell is
// not drawing one.

/// The data table's static dataset (name, role, score) — milestone 237.
pub(crate) const DATA_PEOPLE: [(&str, &str, u32, &str); 12] = [
    ("Ada Lovelace", "Engineer", 92, "High"),
    ("Alan Turing", "Cryptographer", 88, "Medium"),
    ("Grace Hopper", "Admiral", 95, "High"),
    ("Katherine Johnson", "Mathematician", 90, "Medium"),
    ("Edsger Dijkstra", "Researcher", 84, "Low"),
    ("Barbara Liskov", "Professor", 91, "High"),
    ("Donald Knuth", "Author", 87, "Low"),
    ("Margaret Hamilton", "Director", 93, "High"),
    ("Tim Berners-Lee", "Inventor", 89, "Medium"),
    ("Linus Torvalds", "Maintainer", 86, "Low"),
    ("Radia Perlman", "Engineer", 90, "Medium"),
    ("Vint Cerf", "Architect", 85, "Low"),
];

/// Titles of the Kanban columns (a demo, milestone 247).
pub(crate) const KANBAN_TITLES: [&str; 3] = ["To do", "Doing", "Done"];
/// The Kanban's starting cards, per column (a demo, milestone 247).
pub(crate) const KANBAN_SEED: [&[&str]; 3] = [
    &["Design API", "Write spec", "Triage bugs"],
    &["Build widget"],
    &["Kickoff", "Research"],
];

/// Categories (the x axis) of the chart dashboard.
pub(crate) const CHART_CATS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
/// The dashboard's series: `(name, values)`. Series 0 = the theme's accent; 1.. = `CHART_COLORS`.
pub(crate) const CHART_SERIES: [(&str, [f32; 5]); 3] = [
    ("Sales", [3.0, 7.0, 5.0, 8.0, 4.0]),
    ("Costs", [2.0, 4.0, 3.0, 5.0, 2.0]),
    ("Profit", [1.0, 3.0, 2.0, 3.0, 2.0]),
];

/// **Pure** validation of a grid cell: `Name` (col 0) is required, `Email` (col 2) must look
/// like an address. `None` = valid. It demonstrates `TextInput::error` per cell.
pub(crate) fn grid_cell_error(col: usize, value: &str) -> Option<&'static str> {
    match col {
        0 if value.trim().is_empty() => Some("Required"),
        2 if !(value.is_empty() || value.contains('@') && value.contains('.')) => {
            Some("Invalid email")
        }
        _ => None,
    }
}

/// Total number of invalid cells in the grid (milestone 204/207) — it gates the submission.
pub(crate) fn grid_error_count(grid: &[Vec<String>]) -> usize {
    grid.iter()
        .flat_map(|row| {
            (0..3).filter(move |&c| {
                grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or("")).is_some()
            })
        })
        .count()
}

/// Every invalid cell `(row, column)`, in row-by-row order.
pub(crate) fn grid_faults(grid: &[Vec<String>]) -> Vec<(usize, usize)> {
    grid.iter()
        .enumerate()
        .flat_map(|(r, row)| {
            (0..3).filter_map(move |c| {
                grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or(""))
                    .is_some()
                    .then_some((r, c))
            })
        })
        .collect()
}

/// The faulty cell **after** `after` (row-by-row order, wrapping at the end) — so every fault
/// can be cycled through (milestone 214). `after = None` returns the first one.
pub(crate) fn grid_next_error(
    grid: &[Vec<String>],
    after: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let faults = grid_faults(grid);
    match after {
        None => faults.first().copied(),
        Some(cur) => faults
            .iter()
            .copied()
            .find(|&f| f > cur)
            .or_else(|| faults.first().copied()),
    }
}
