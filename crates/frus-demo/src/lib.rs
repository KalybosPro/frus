//! Sample application: a **to-do list** written with frus, as an **external consumer** of the
//! framework (it implements [`frus_shell::Application`]).
//!
//! Two entry points for the same code:
//! - desktop: `cargo run -p frus-demo` → the `src/bin/frus-demo.rs` binary → [`run_desktop`];
//! - Android: the `cdylib` library exposes `android_main`, called by the native activity.

use std::path::{Path, PathBuf};
use std::time::Duration;

use frus_l10n::{args, Localizer};
use frus_shell::{Application, Command, Lifecycle, Subscription};
use std::sync::OnceLock;

/// The demo's localizer: English, French and Arabic, loaded once from embedded Fluent
/// resources (`i18n/*.ftl`).
fn l10n() -> &'static Localizer {
    static L10N: OnceLock<Localizer> = OnceLock::new();
    L10N.get_or_init(|| {
        let mut l = Localizer::new("en");
        l.add("en", include_str!("../i18n/en.ftl"));
        l.add("fr", include_str!("../i18n/fr.ftl"));
        l.add("ar", include_str!("../i18n/ar.ftl"));
        l
    })
}

/// The languages the demo offers (menu label, locale code). The last one, Arabic, is
/// **right-to-left**: selecting it also mirrors the layout (bidi + mirroring).
const LANGS: [(&str, &str); 3] = [("English", "en"), ("Français", "fr"), ("العربية", "ar")];

/// Is the language at index `lang` written right to left?
fn lang_is_rtl(lang: usize) -> bool {
    LANGS[lang].1 == "ar"
}

/// Translates an argument-free key into the language at index `lang`.
fn tr(lang: usize, key: &str) -> String {
    let loc = l10n();
    loc.format_for(&loc.langid(LANGS[lang].1), key, args![])
}

/// Translates a key with a numeric argument `n` (CLDR plurals).
fn tr_n(lang: usize, key: &str, n: usize) -> String {
    let loc = l10n();
    loc.format_for(&loc.langid(LANGS[lang].1), key, args![n: n])
}
use frus_widgets::form::{Form, Rule};
use frus_widgets::{
    button, column, keyed, row, spacer, text, Alert, Align, AnimationController, AppBar,
    Autocomplete, Avatar, Axis, BarChart, BoxFit, Breadcrumb, Card, Carousel, Checkbox, Chip,
    Collapsible, Color, ColorPicker, Container, CustomPaint, DataTable, DatePicker, Divider,
    Dropdown, ErrorSummary, Flex, FontWeight, Grid, Icon, IconName, Image, ImageData, ImageHandle,
    Insets, Justify, Kanban, Kbd, LayoutBuilder, LineChart, List, NavBar, Navigator, Orientation,
    Dismissible, Draggable, DragTarget, Hero, PageView, Pagination, Placement, Popover, Portal,
    ProgressBar, RadioGroup, Rating, Rect, SizedBox,
    Refresh,
    RichText,
    Scaffold, Scroll, ScrollPhysics, SegmentedControl, Size, SizeClass, Skeleton, Slider,
    SnackbarQueue, SpringDescription, Stack, Stepper, Steps, Switch, Table, Tabs, TextInput,
    TextSpan, Theme, Timeline, Toast, ToastHost, ToastPosition, Tree, TwoPane, Variant, Widget,
    WindowInsets,
};

/// The demo logo, **decoded** from an embedded PNG (milestone 91) and shared across the whole
/// process through a `OnceLock` — decoded once, then cached by identity on the renderer's
/// side. Falls back to a generated gradient if the decoding fails (robustness).
fn demo_image() -> ImageHandle {
    use std::sync::OnceLock;
    static IMG: OnceLock<ImageHandle> = OnceLock::new();
    IMG.get_or_init(|| {
        frus_image::decode(include_bytes!("../assets/logo.png"))
            .map(ImageData::into_handle)
            .unwrap_or_else(|_| fallback_gradient())
    })
    .clone()
}

/// A generated 64×64 gradient — the fallback when decoding the PNG fails.
fn fallback_gradient() -> ImageHandle {
    const W: u32 = 64;
    const H: u32 = 64;
    let mut rgba = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        for x in 0..W {
            rgba.push((x * 255 / (W - 1)) as u8);
            rgba.push((y * 255 / (H - 1)) as u8);
            rgba.push(160u8);
            rgba.push(255u8);
        }
    }
    ImageData::from_rgba(W, H, rgba).into_handle()
}

// A **single** entry point: one declaration generates both the desktop entry (`run()`, called
// by the binary) and the Android one (`android_main`). See `frus_shell::main!`.
frus_shell::main!(TodoApp::default());

// --- Motion constants (shared between the gesture and the navigation) ---

/// Horizon (s) over which the velocity is projected to decide back / cancel.
const BACK_PROJECT: f32 = 0.12;
/// Projected position (a fraction) beyond which the back is committed.
const BACK_COMMIT_POS: f32 = 0.5;
/// Stiffness of the transition spring (fraction·s⁻²).
const NAV_SPRING_K: f32 = 220.0;
/// Damping (~critical) → a gentle arrival with no overshoot.
const NAV_SPRING_C: f32 = 30.0;

/// The spring shared by the navigation and the back gesture, expressed in `frus-core`'s
/// animation layer (`trait Simulation`).
fn nav_spring() -> SpringDescription {
    SpringDescription::new(1.0, NAV_SPRING_K, NAV_SPRING_C)
}

/// Starts a screen transition: the spring drives the progress `0 → 1`.
fn start_nav(app: &mut TodoApp, forward: bool) {
    app.nav_forward = forward;
    app.nav.set_value(0.0);
    app.nav.spring_to(1.0, nav_spring(), 0.0);
}

/// Labels of the dropdown menu (the Settings screen).
const MENU: [&str; 3] = ["Option A", "Option B", "Option C"];

// --- Model ---

/// One task of the list.
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

/// Display filter of the task list.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Active,
    Done,
}

/// The application's screens.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Route {
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
struct BackGesture {
    progress: f32,
    velocity: f32,
    /// `Some` once released: the spring settle in progress (`None` = still following the
    /// finger).
    settle: Option<AnimationController>,
    /// Does the settle, when it ends, commit the back (a pop)?
    commit: bool,
}

/// Messages emitted by the interface.
#[derive(Clone)]
enum Msg {
    DraftChanged(String),
    /// Clears the input field (the "✕" suffix icon).
    ClearDraft,
    AddTodo,
    ToggleTodo(u64),
    /// A task dropped on one of the two state zones: set it done, or set it active.
    SetTodoDone(u64, bool),
    DeleteTodo(u64),
    SetFilter(Filter),
    AskClearDone,
    ConfirmClearDone,
    CancelClear,
    ToggleTheme,
    /// Moves to the next theme seed (default → Blue → Purple → Orange).
    CycleSeed,
    /// Flips the layout direction (LTR ↔ RTL).
    ToggleRtl,
    /// Moves to the next language.
    CycleLang,
    SetNotifs(bool),
    SetVolume(f32),
    SetRadio(usize),
    ToggleMenu,
    SetMenu(usize),
    Push(Route),
    Pop,
    /// Flips the log list between the two scroll behaviours, so the difference can
    /// be felt side by side on one device.
    ToggleScrollPhysics,
    /// The log list was pulled past its top edge: reload it.
    ReloadJournal,
    /// A tick of the stopwatch (the timer subscription).
    Tick,
    /// Starts/stops the stopwatch.
    ToggleTimer,
    /// Changes the active Settings tab.
    SetSettingsTab(usize),
    /// The walkthrough's page, whether the finger or a button asked for it.
    TourPage(usize),
    /// Opens one task's own screen.
    OpenTask(u64),
    /// Opens/closes the actions menu.
    ToggleActions,
    /// Expands/collapses "Advanced options".
    ToggleAdvanced,
    /// The star rating that was picked.
    SetRating(u32),
    /// The stepper's new value.
    SetCount(i32),
    /// Starts the head notification's **exit** (a fade) before it is removed.
    ToastExpire,
    /// Removes the current notification (end of its exit / a click) and moves on to the next.
    DismissToast,
    /// Changes the page (the pagination demo).
    SetPage(usize),
    /// Expands/collapses a tree node.
    ToggleNode(u64),
    /// Selects a node of the file tree (milestone 246).
    SelectNode(u64),
    /// Moves a Kanban card: `(from_col, from_pos, to_col, to_pos)` (milestone 247).
    KanbanMove(usize, usize, usize, usize),
    /// Adds a card at the bottom of a Kanban column (milestone 249).
    KanbanAdd(usize),
    /// Deletes the Kanban card `(col, pos)` (milestone 249).
    KanbanDelete(usize, usize),
    /// Picks a colour.
    PickColor(Color),
    /// Selects a day in the calendar.
    PickDay(u32),
    /// Changes month (±1).
    NavMonth(i32),
    /// Changes the carousel's slide.
    SetSlide(usize),
    /// Sets the density (an application-level zoom).
    SetDensity(f32),
    /// Opens/closes the info popover.
    ToggleInfo,
    /// Typing in the autocomplete.
    TagInput(String),
    /// A suggestion was chosen.
    TagPick(String),
    /// Saves the tasks to disk (an effect).
    Save,
    /// Asks for the tasks to be loaded (an effect).
    Load,
    /// Tasks loaded from disk (the result of an effect).
    Loaded(Vec<(bool, String)>),
    /// Changes the home screen's active section (adaptive navigation).
    SetSection(usize),
    /// Selects a metric in the Stats section (which opens the detail when narrow).
    SelectStat(usize),
    /// Closes the Stats detail (back to the list in single-pane mode).
    CloseDetail,
    /// Opens/closes the side navigation drawer.
    ToggleDrawer,
    /// Opens/closes the quick-actions modal sheet.
    ToggleSheet,
    // --- Sign-up wizard (an integration demo) ---
    /// Jumps to the wizard's step `i` (a Steps marker was clicked).
    WizardStep(usize),
    /// Jumps to step `usize` **and focuses** field `u8` (a summary bullet was clicked).
    WizardFocus(usize, u8),
    /// Typing in a wizard field: `(0=name, 1=email, 2=password, 3=confirmation)`.
    WizardInput(u8, String),
    /// The wizard's previous / next step.
    WizardBack,
    WizardNext,
    /// Reveals / hides the wizard's passwords.
    WizardToggleReveal,
    // --- Editable grid (an integration demo) ---
    /// The new value of cell `(row, column)`.
    GridInput(usize, usize, String),
    /// Enter pressed in cell `(row, column)`: jumps to the next row, same column.
    GridEnter(usize, usize),
    /// Adds an empty row at the end of the grid and puts the caret in it.
    GridAddRow,
    /// Deletes row `row`.
    GridDeleteRow(usize),
    /// Sorts the grid by column `column` (toggling ascending / descending).
    GridSort(usize),
    /// Submits the grid: it only goes through when every cell is valid.
    GridSave,
    /// Puts the focus on the grid's first invalid cell, if there is one.
    GridFocusError,
    /// Toggles the visibility of the chart's series `index` (a legend click, milestone 218).
    ChartToggleSeries(usize),
    /// Changes the kind of chart shown (the selector, milestone 219).
    SetChartKind(usize),
    /// Pins the detail of a clicked point `(category, series)` (milestone 221).
    ChartPoint(usize, usize),
    /// Turns the chart's **100%** stacking (proportions) on/off (milestone 224).
    SetChartNormalized(bool),
    /// A click on a data-table header: sorts (or unsorts) the column (milestone 237).
    DataSort(usize),
    /// Changes the data table's page (milestone 237).
    DataPage(usize),
    /// Changes the data table's page size (milestone 237).
    DataPageSize(usize),
    /// A click on a data-table row: selects (or deselects) the **source** row (milestone 239).
    DataSelectRow(usize),
    /// Checks/unchecks a **source** row of the data table (multi-selection, milestone 241).
    DataCheck(usize),
    /// The data table's "check all" box: checks everything, or clears it when all is already checked (milestone 241).
    DataCheckAll,
    /// Typing in the data table's search field: updates the filter (milestone 242).
    DataSearch(String),
    /// A bulk action: unchecks every checked row of the data table (milestone 243).
    DataClearChecked,
    /// A bulk action: opens the confirmation for deleting the checked rows (milestone 245).
    DataAskDelete,
    /// Closes the delete confirmation without deleting anything (milestone 245).
    DataCancelDelete,
    /// A bulk action: deletes the data table's checked rows (milestone 243).
    DataDeleteChecked,
    /// The showcase calendar's "weekdays only" toggle (milestone 238).
    SetWeekdaysOnly(bool),
    /// Submits the wizard: validates the form, then notifies or shows the errors.
    WizardSubmit,
}

/// Path of the file the tasks are persisted to.
fn todos_path() -> PathBuf {
    std::env::temp_dir().join("frus-todos.txt")
}

/// Serialises the tasks as `done<TAB>text` lines.
fn save_todos(path: &Path, todos: &[(bool, String)]) -> std::io::Result<()> {
    let mut out = String::new();
    for (done, text) in todos {
        out.push(if *done { '1' } else { '0' });
        out.push('\t');
        // Neutralises the separators inside the text.
        out.push_str(&text.replace(['\t', '\n'], " "));
        out.push('\n');
    }
    std::fs::write(path, out)
}

/// Reads the tasks from the file (empty when it is missing/unreadable).
fn load_todos(path: &Path) -> Vec<(bool, String)> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| {
            let (flag, text) = line.split_once('\t')?;
            Some((flag == "1", text.to_string()))
        })
        .collect()
}

/// The to-do application: state + logic. A consumer of the `frus-shell` framework.
#[derive(Default)]
struct TodoApp {
    /// The tasks, in the order they were added.
    todos: Vec<Todo>,
    /// The text currently being typed.
    draft: String,
    /// The current filter.
    filter: Filter,
    /// The next task id.
    next_id: u64,
    /// Is the "clear completed" confirmation modal open?
    confirm_clear: bool,
    /// A light theme (otherwise dark).
    light: bool,
    /// The outgoing theme during a switch fade (`None` = no transition).
    theme_from: Option<Theme>,
    /// Progress of the theme fade (`0 → 1`).
    theme_progress: f32,
    /// Does the log list bounce at its ends rather than stop dead? `false` leaves
    /// it on the platform's own behaviour.
    journal_bounces: bool,
    /// Is the log list reloading? The pull-to-refresh indicator spins for exactly as
    /// long as this is true — the application owns the answer, not the framework.
    journal_reloading: f32,
    /// How many times the log has been reloaded, so a completed pull leaves a trace.
    journal_reloads: usize,
    /// The walkthrough's page. The application owns it: the finger reports its changes
    /// here, and the buttons write to it, so both drive the same one value.
    tour_page: usize,
    /// The screen stack (empty = the home screen).
    routes: Vec<Route>,
    /// The outgoing screen during a transition.
    nav_from: Option<Route>,
    /// Progress of the screen transition (`0 → 1`), driven by a spring.
    nav: AnimationController,
    nav_forward: bool,
    /// The back gesture in progress.
    back: Option<BackGesture>,
    // --- Controls of the Settings screen ---
    notifs: bool,
    volume: f32,
    radio: usize,
    menu_open: bool,
    menu_choice: usize,
    // --- Chrono (souscription timer) ---
    /// Is the stopwatch running? (it drives the `every` subscription).
    running: bool,
    /// Is the app in the **background**? (the lifecycle, milestone 259) — set to `true` on
    /// `Paused`/`Detached` through [`Application::on_lifecycle`], at which point the timer
    /// suspends. `false` by default (the foreground) — which suits `#[derive(Default)]`.
    background: bool,
    /// Seconds elapsed since the stopwatch started.
    elapsed: u32,
    /// The Settings screen's active tab.
    settings_tab: usize,
    /// Is the (header) actions menu open?
    actions_open: bool,
    /// Is the "Advanced options" section expanded?
    advanced_open: bool,
    /// The star rating (Settings).
    rating: u32,
    /// The stepper's counter (Settings).
    count: i32,
    /// The notification (Snackbar) queue: one at a time, with an animated exit (milestone 193).
    snackbars: SnackbarQueue<String>,
    /// The pagination selector's current page (a demo).
    page: usize,
    /// The expanded tree nodes (the Tree demo).
    expanded: std::collections::HashSet<u64>,
    /// The selected tree node (the Tree demo, milestone 246); `None` = none.
    tree_selected: Option<u64>,
    /// The colour picked (the ColorPicker demo).
    picked: Option<Color>,
    /// The calendar: year / month (1..12) / selected day.
    year: i32,
    month: u32,
    selected_day: Option<u32>,
    /// The showcase calendar: should weekends be disabled (`DatePicker::filtered`)? — milestone 238.
    weekdays_only: bool,
    /// The carousel's current slide (a demo).
    slide: usize,
    /// Is the info popover open?
    info_open: bool,
    /// What is typed in the autocomplete (a demo).
    tag_draft: String,
    /// Section active de l'accueil (0 = Tasks, 1 = Stats, 2 = About) — NavScaffold.
    section: usize,
    /// The metric selected in the Stats section (a master-detail TwoPane).
    stat_sel: usize,
    /// In single-pane (narrow) mode, is the Stats detail open?
    stat_detail_open: bool,
    /// Density (an application-level zoom over the whole UI) — `1.0` by default.
    density: f32,
    /// The current size class (updated by `on_resize`).
    size_class: Option<SizeClass>,
    /// The current orientation (updated by `on_resize`).
    orientation: Option<Orientation>,
    /// Is the side navigation drawer open?
    drawer_open: bool,
    /// Is the quick-actions modal sheet open?
    sheet_open: bool,
    /// System insets (the safe area): status/navigation bars, notches.
    insets: Insets,
    /// The theme seed: `0` = the hand-written scheme, otherwise `from_seed` (HCT).
    seed_index: usize,
    /// A right-to-left layout (Arabic/Hebrew)?
    rtl: bool,
    /// The current language (an index into `LANGS`).
    lang: usize,
    /// The state came from a live-reload snapshot: `init` does not reload the tasks from disk
    /// (the snapshot is the authority).
    restored: bool,
    // --- Sign-up wizard (an integration demo) ---
    /// The wizard's current step (0 = Account, 1 = Security, 2 = Review).
    wizard_step: usize,
    wizard_name: String,
    wizard_email: String,
    wizard_pass: String,
    wizard_confirm: String,
    /// Has the wizard been submitted at least once? (the errors only show afterwards.)
    wizard_submitted: bool,
    /// Are the wizard's passwords **revealed** (unmasked)?
    wizard_reveal: bool,
    /// The editable grid's data (rows × columns of text).
    grid: Vec<Vec<String>>,
    /// The grid's current sort: `(column, ascending)`; `None` = the order it was typed in.
    grid_sort: Option<(usize, bool)>,
    /// The last faulty cell "Next error" targeted (milestone 214) — so it can cycle to the next.
    grid_error_cursor: Option<(usize, usize)>,
    /// Indices of the chart's **hidden** series (toggled through the legend, milestone 218).
    chart_hidden: Vec<usize>,
    /// The kind of chart shown: 0 lines, 1 stacked areas, 2 grouped bars, 3 stacked bars (milestone 219).
    chart_kind: usize,
    /// The **pinned** detail of a clicked point (`series · category = value`), if there is one (milestone 221).
    chart_pin: Option<String>,
    /// The **selected** point/bar `(category, series)`, highlighted in the chart (milestone 223).
    chart_sel: Option<(usize, usize)>,
    /// **100%** stacking (proportions) turned on for the stacked charts (milestone 224).
    chart_normalized: bool,
    /// The data table's sort `(column, ascending)`; `None` = the source order (milestone 237).
    /// The display sort is done by the `DataTable`, **not** duplicated here.
    data_sort: Option<(usize, bool)>,
    /// The data table's current page (1-indexed); `0` = page 1 (milestone 237).
    data_page: usize,
    /// The data table's page size; `0` = the default (milestone 237).
    data_page_size: usize,
    /// The data table's selected **source** row; `None` = none (milestone 239). The `DataTable`
    /// translates that original index into a highlighted position through sorting/pagination.
    data_selected: Option<usize>,
    /// The data table's checked **source** rows (multi-selection, milestone 241). It drives the
    /// boxes and the highlighting; the app decides what "check all" covers (here, all 12 rows).
    data_checked: Vec<usize>,
    /// The data table's search query (milestone 242); the `DataTable` filters the display.
    data_query: String,
    /// The data table's rows (milestone 243): `None` = the `DATA_PEOPLE` starting set; `Some` as
    /// soon as a bulk action (Delete) changes them. See [`TodoApp::data_rows`].
    data_rows: Option<Vec<(String, String, u32, String)>>,
    /// Is the data table's bulk-delete confirmation modal open? (milestone 245)
    data_confirm_delete: bool,
    /// The Kanban cards, per column (milestone 247); `None` = the `KANBAN_SEED` starting set.
    /// `Some` as soon as a card is moved. See [`TodoApp::kanban_cols`].
    kanban: Option<Vec<Vec<String>>>,
}

fn current_route(app: &TodoApp) -> Route {
    app.routes.last().copied().unwrap_or(Route::Home)
}

impl TodoApp {
    /// The data table's current rows: the ones held in the state if they have been changed (a
    /// bulk Delete), otherwise the `DATA_PEOPLE` starting set (milestone 243).
    fn data_rows(&self) -> Vec<(String, String, u32, String)> {
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
    fn kanban_cols(&self) -> Vec<Vec<String>> {
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
fn filter_index(filter: Filter) -> usize {
    match filter {
        Filter::All => 0,
        Filter::Active => 1,
        Filter::Done => 2,
    }
}

/// The filter matching a segment index.
fn filter_from_index(index: usize) -> Filter {
    match index {
        1 => Filter::Active,
        2 => Filter::Done,
        _ => Filter::All,
    }
}

/// Number of tasks that are not done.
fn active_count(app: &TodoApp) -> usize {
    app.todos.iter().filter(|t| !t.done).count()
}

/// Number of tasks that are done.
fn done_count(app: &TodoApp) -> usize {
    app.todos.iter().filter(|t| t.done).count()
}

/// Demonstration seeds for the dynamic theme (`from_seed`, HCT).
const THEME_SEEDS: [(&str, Color); 3] = [
    (
        "Blue",
        Color {
            r: 0x42 as f32 / 255.0,
            g: 0x85 as f32 / 255.0,
            b: 0xF4 as f32 / 255.0,
            a: 1.0,
        },
    ),
    (
        "Purple",
        Color {
            r: 0x9C as f32 / 255.0,
            g: 0x27 as f32 / 255.0,
            b: 0xB0 as f32 / 255.0,
            a: 1.0,
        },
    ),
    (
        "Orange",
        Color {
            r: 0xE8 as f32 / 255.0,
            g: 0x71 as f32 / 255.0,
            b: 0x0A as f32 / 255.0,
            a: 1.0,
        },
    ),
];

/// Label of the menu's "seed" action (the **next** seed of the cycle).
fn seed_label(app: &TodoApp) -> String {
    match THEME_SEEDS.get(app.seed_index) {
        Some((name, _)) => format!("Seed: {name}"),
        None => "Seed: default".to_string(),
    }
}

/// The "target" theme for the current state (before the fade): the hand-written scheme by
/// default, or one generated from a seed (`from_seed`, HCT).
fn theme_of(app: &TodoApp) -> Theme {
    let theme = match app
        .seed_index
        .checked_sub(1)
        .and_then(|i| THEME_SEEDS.get(i))
    {
        Some((_, seed)) => Theme::from_seed(*seed, !app.light),
        None => {
            if app.light {
                Theme::light()
            } else {
                Theme::dark()
            }
        }
    };
    // The ambient direction: RTL if the user asked for it OR if the current language is
    // written right to left (Arabic). The whole layout mirrors.
    if app.rtl || lang_is_rtl(app.lang) {
        theme.rtl()
    } else {
        theme
    }
}

/// A timed effect: it starts the head notification's **exit** after it has been up for ~2 s.
fn toast_expire_after() -> Command<Msg> {
    Command::perform(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        Msg::ToastExpire
    })
}

/// Queues a notification (Snackbar); when it becomes the **head** of the queue, its exit is scheduled.
fn show_toast(app: &mut TodoApp, text: &str) -> Command<Msg> {
    let was_empty = app.snackbars.is_empty();
    app.snackbars.push(text.to_string(), 0.0);
    if was_empty {
        toast_expire_after()
    } else {
        Command::none()
    }
}

fn reduce(app: &mut TodoApp, message: Msg) -> Command<Msg> {
    match message {
        Msg::ClearDraft => {
            app.draft.clear();
            Command::none()
        }
        Msg::DraftChanged(text) => {
            app.draft = text;
            Command::none()
        }
        Msg::AddTodo => {
            let text = app.draft.trim();
            if !text.is_empty() {
                app.todos.push(Todo {
                    id: app.next_id,
                    text: text.to_string(),
                    done: false,
                });
                app.next_id += 1;
                app.draft.clear();
            }
            Command::none()
        }
        Msg::ToggleTodo(id) => {
            if let Some(todo) = app.todos.iter_mut().find(|t| t.id == id) {
                todo.done = !todo.done;
            }
            Command::none()
        }
        Msg::SetTodoDone(id, done) => {
            if let Some(todo) = app.todos.iter_mut().find(|t| t.id == id) {
                todo.done = done;
            }
            Command::none()
        }
        Msg::DeleteTodo(id) => {
            app.todos.retain(|t| t.id != id);
            Command::none()
        }
        Msg::SetFilter(filter) => {
            app.filter = filter;
            Command::none()
        }
        Msg::AskClearDone => {
            app.sheet_open = false;
            app.confirm_clear = true;
            Command::none()
        }
        Msg::ConfirmClearDone => {
            app.todos.retain(|t| !t.done);
            app.confirm_clear = false;
            Command::none()
        }
        Msg::CancelClear => {
            app.confirm_clear = false;
            Command::none()
        }
        Msg::ToggleTheme => {
            // Captures the current theme (before the switch) as the fade's starting point.
            app.theme_from = Some(theme_of(app));
            app.light = !app.light;
            app.theme_progress = 0.0;
            Command::none()
        }
        Msg::CycleSeed => {
            // The same fade as the light/dark switch, towards the generated scheme.
            app.theme_from = Some(theme_of(app));
            app.seed_index = (app.seed_index + 1) % (THEME_SEEDS.len() + 1);
            app.theme_progress = 0.0;
            Command::none()
        }
        Msg::ToggleScrollPhysics => {
            app.journal_bounces = !app.journal_bounces;
            Command::none()
        }
        Msg::ReloadJournal => {
            // A stand-in for the request a real application would fire here: the
            // indicator spins for as long as `journal_reloading` counts down in `tick`.
            app.journal_reloading = 1.2;
            Command::none()
        }
        Msg::ToggleRtl => {
            // The direction is discrete (no fade): it flips at once.
            app.rtl = !app.rtl;
            Command::none()
        }
        Msg::CycleLang => {
            app.lang = (app.lang + 1) % LANGS.len();
            Command::none()
        }
        Msg::SetNotifs(v) => {
            app.notifs = v;
            Command::none()
        }
        Msg::SetVolume(v) => {
            app.volume = v;
            Command::none()
        }
        Msg::SetRadio(i) => {
            app.radio = i;
            Command::none()
        }
        Msg::ToggleMenu => {
            app.menu_open = !app.menu_open;
            Command::none()
        }
        Msg::SetMenu(i) => {
            app.menu_choice = i;
            app.menu_open = false;
            Command::none()
        }
        Msg::Push(route) => {
            app.drawer_open = false;
            app.menu_open = false;
            app.nav_from = Some(current_route(app));
            app.routes.push(route);
            start_nav(app, true);
            Command::none()
        }
        Msg::Pop => {
            if !app.routes.is_empty() {
                app.nav_from = Some(current_route(app));
                app.routes.pop();
                start_nav(app, false);
            }
            Command::none()
        }
        Msg::Tick => {
            app.elapsed += 1;
            // A trace of the tick: proof that the subscription emits messages.
            eprintln!("[demo] stopwatch: {}s", app.elapsed);
            Command::none()
        }
        Msg::ToggleTimer => {
            app.running = !app.running;
            Command::none()
        }
        Msg::OpenTask(id) => {
            app.nav_from = Some(current_route(app));
            app.routes.push(Route::Task(id));
            start_nav(app, true);
            Command::none()
        }
        Msg::TourPage(page) => {
            app.tour_page = page;
            Command::none()
        }
        Msg::SetSettingsTab(i) => {
            app.settings_tab = i;
            Command::none()
        }
        Msg::ToggleActions => {
            app.actions_open = !app.actions_open;
            Command::none()
        }
        Msg::ToggleAdvanced => {
            app.advanced_open = !app.advanced_open;
            Command::none()
        }
        Msg::SetRating(r) => {
            app.rating = r;
            Command::none()
        }
        Msg::SetCount(c) => {
            app.count = c;
            Command::none()
        }
        // --- Effets ---
        Msg::Save => {
            app.sheet_open = false;
            // Takes a serialisable snapshot; the writing happens outside update.
            let items: Vec<(bool, String)> =
                app.todos.iter().map(|t| (t.done, t.text.clone())).collect();
            // Shows a notification (Snackbar) and writes to disk (an effect).
            let show = show_toast(app, "Saved");
            Command::batch([
                Command::run(move || {
                    let _ = save_todos(&todos_path(), &items);
                    None
                }),
                show,
            ])
        }
        Msg::ToastExpire => {
            // Moves the notification into its **exit** (the host plays the fade), then removes it.
            app.snackbars.start_leaving();
            Command::perform(|| {
                std::thread::sleep(std::time::Duration::from_millis(300));
                Msg::DismissToast
            })
        }
        Msg::DismissToast => {
            app.snackbars.dismiss();
            // Moves on to the next queued notification, if there is one.
            if app.snackbars.is_empty() {
                Command::none()
            } else {
                toast_expire_after()
            }
        }
        Msg::WizardStep(i) => {
            app.wizard_step = i.min(2);
            Command::none()
        }
        Msg::WizardFocus(step, field) => {
            // Jumps to the faulty field's step then focuses that field (keyed + Command::focus).
            app.wizard_step = step.min(2);
            Command::focus(("wizard", field))
        }
        Msg::WizardInput(field, value) => {
            match field {
                0 => app.wizard_name = value,
                1 => app.wizard_email = value,
                2 => app.wizard_pass = value,
                _ => app.wizard_confirm = value,
            }
            Command::none()
        }
        Msg::WizardBack => {
            app.wizard_step = app.wizard_step.saturating_sub(1);
            Command::none()
        }
        Msg::WizardNext => {
            app.wizard_step = (app.wizard_step + 1).min(2);
            Command::none()
        }
        Msg::WizardToggleReveal => {
            app.wizard_reveal = !app.wizard_reveal;
            Command::none()
        }
        Msg::GridInput(r, c, value) => {
            if let Some(cell) = app.grid.get_mut(r).and_then(|row| row.get_mut(c)) {
                *cell = value;
            }
            Command::none()
        }
        Msg::GridEnter(r, c) => {
            // Enter = move down one row (same column); on the last row, one is created.
            if r + 1 < app.grid.len() {
                Command::focus(("grid", r + 1, c))
            } else {
                let cols = app.grid.first().map(|row| row.len()).unwrap_or(3);
                app.grid.push(vec![String::new(); cols]);
                Command::focus(("grid", r + 1, c))
            }
        }
        Msg::GridAddRow => {
            let cols = app.grid.first().map(|row| row.len()).unwrap_or(3);
            app.grid.push(vec![String::new(); cols]);
            // Focuses the new row's first cell.
            Command::focus(("grid", app.grid.len() - 1, 0))
        }
        Msg::GridDeleteRow(r) => {
            if r < app.grid.len() {
                app.grid.remove(r);
            }
            Command::none()
        }
        Msg::GridSave => {
            let errors = grid_error_count(&app.grid);
            if errors == 0 {
                return show_toast(app, "Grid saved");
            }
            let msg = if errors == 1 {
                "Fix 1 error before saving".to_string()
            } else {
                format!("Fix {errors} errors before saving")
            };
            return show_toast(app, &msg);
        }
        Msg::GridFocusError => {
            // Cycles to the next faulty cell (wrapping) and focuses it.
            match grid_next_error(&app.grid, app.grid_error_cursor) {
                Some(pos) => {
                    app.grid_error_cursor = Some(pos);
                    Command::focus(("grid", pos.0, pos.1))
                }
                None => {
                    app.grid_error_cursor = None;
                    Command::none()
                }
            }
        }
        Msg::ChartToggleSeries(i) => {
            // A legend click: hides the series when it is visible, shows it again otherwise.
            if let Some(pos) = app.chart_hidden.iter().position(|&h| h == i) {
                app.chart_hidden.remove(pos);
            } else {
                app.chart_hidden.push(i);
            }
            Command::none()
        }
        Msg::SetChartKind(k) => {
            app.chart_kind = k;
            Command::none()
        }
        Msg::SetChartNormalized(on) => {
            app.chart_normalized = on;
            Command::none()
        }
        Msg::DataSort(c) => {
            // Toggles ascending/descending (a new column starts ascending). The `DataTable` sorts
            // the display itself — no `sort_by` is duplicated here (milestone 237).
            let asc = match app.data_sort {
                Some((col, asc)) if col == c => !asc,
                _ => true,
            };
            app.data_sort = Some((c, asc));
            app.data_page = 1; // sorting returns to the first page
            Command::none()
        }
        Msg::DataPage(p) => {
            app.data_page = p;
            Command::none()
        }
        Msg::DataPageSize(s) => {
            app.data_page_size = s;
            app.data_page = 1; // changing the size returns to the first page
            Command::none()
        }
        Msg::DataSelectRow(i) => {
            // Clicking the already-selected row again deselects it (a toggle).
            app.data_selected = if app.data_selected == Some(i) {
                None
            } else {
                Some(i)
            };
            Command::none()
        }
        Msg::DataCheck(i) => {
            // Toggles whether the source row belongs to the checked set.
            match app.data_checked.iter().position(|&x| x == i) {
                Some(pos) => {
                    app.data_checked.remove(pos);
                }
                None => app.data_checked.push(i),
            }
            Command::none()
        }
        Msg::DataCheckAll => {
            // All checked → clear it; otherwise check every current source row.
            let n = app.data_rows().len();
            app.data_checked = if app.data_checked.len() == n {
                Vec::new()
            } else {
                (0..n).collect()
            };
            Command::none()
        }
        Msg::DataSearch(q) => {
            app.data_query = q;
            app.data_page = 1; // a new filter returns to the first page
            Command::none()
        }
        Msg::DataClearChecked => {
            app.data_checked.clear();
            Command::none()
        }
        Msg::DataAskDelete => {
            app.data_confirm_delete = true;
            Command::none()
        }
        Msg::DataCancelDelete => {
            app.data_confirm_delete = false;
            Command::none()
        }
        Msg::DataDeleteChecked => {
            // Actually deletes the checked rows (by source index, in descending order so the
            // following ones do not shift), then resets the selection, the focus and the modal.
            app.data_confirm_delete = false;
            let mut rows = app.data_rows();
            let mut checked: Vec<usize> = app
                .data_checked
                .iter()
                .copied()
                .filter(|&i| i < rows.len())
                .collect();
            checked.sort_unstable();
            checked.dedup();
            for &i in checked.iter().rev() {
                rows.remove(i);
            }
            app.data_rows = Some(rows);
            app.data_checked.clear();
            app.data_selected = None;
            Command::none()
        }
        Msg::SetWeekdaysOnly(on) => {
            app.weekdays_only = on;
            Command::none()
        }
        Msg::ChartPoint(cat, s) => {
            // Clicking the already-selected item again **unpins** it (milestone 225). Otherwise it
            // pins "series · category = value" and remembers the selection `(cat, series)`
            // (milestones 221/223).
            if app.chart_sel == Some((cat, s)) {
                app.chart_sel = None;
                app.chart_pin = None;
            } else if let (Some((name, vals)), Some(label)) =
                (CHART_SERIES.get(s), CHART_CATS.get(cat))
            {
                if let Some(v) = vals.get(cat) {
                    app.chart_pin = Some(format!("{name} · {label} = {}", *v as i64));
                    app.chart_sel = Some((cat, s));
                }
            }
            Command::none()
        }
        Msg::GridSort(c) => {
            // Toggles ascending / descending on the clicked column, then sorts the rows.
            let asc = match app.grid_sort {
                Some((col, asc)) if col == c => !asc,
                _ => true,
            };
            app.grid_sort = Some((c, asc));
            app.grid.sort_by(|a, b| {
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
            Command::none()
        }
        Msg::WizardSubmit => {
            if wizard_form(app).is_valid() {
                // Success: resets the wizard and notifies (a Snackbar, with an animated exit).
                app.wizard_step = 0;
                app.wizard_name.clear();
                app.wizard_email.clear();
                app.wizard_pass.clear();
                app.wizard_confirm.clear();
                app.wizard_submitted = false;
                show_toast(app, "Account created")
            } else {
                // Errors: reveals them and shows the summary on the Review step.
                app.wizard_submitted = true;
                app.wizard_step = 2;
                Command::none()
            }
        }
        Msg::SetPage(p) => {
            app.page = p;
            Command::none()
        }
        Msg::ToggleNode(id) => {
            if !app.expanded.remove(&id) {
                app.expanded.insert(id);
            }
            Command::none()
        }
        Msg::SelectNode(id) => {
            // Clicking the already-selected node again deselects it (a toggle).
            app.tree_selected = if app.tree_selected == Some(id) {
                None
            } else {
                Some(id)
            };
            Command::none()
        }
        Msg::KanbanMove(from_col, from_pos, to_col, to_pos) => {
            let mut cols = app.kanban_cols();
            if from_col < cols.len() && from_pos < cols[from_col].len() && to_col < cols.len() {
                let card = cols[from_col].remove(from_pos);
                // After the removal, a target further down the **same** column shifts by one.
                let mut tp = to_pos;
                if from_col == to_col && from_pos < tp {
                    tp -= 1;
                }
                tp = tp.min(cols[to_col].len());
                cols[to_col].insert(tp, card);
                app.kanban = Some(cols);
            }
            Command::none()
        }
        Msg::KanbanAdd(col) => {
            let mut cols = app.kanban_cols();
            if col < cols.len() {
                cols[col].push("New card".to_string());
                app.kanban = Some(cols);
            }
            Command::none()
        }
        Msg::KanbanDelete(col, pos) => {
            let mut cols = app.kanban_cols();
            if col < cols.len() && pos < cols[col].len() {
                cols[col].remove(pos);
                app.kanban = Some(cols);
            }
            Command::none()
        }
        Msg::PickColor(c) => {
            app.picked = Some(c);
            Command::none()
        }
        Msg::PickDay(d) => {
            app.selected_day = Some(d);
            Command::none()
        }
        Msg::NavMonth(delta) => {
            let mut m = app.month as i32 + delta;
            while m < 1 {
                m += 12;
                app.year -= 1;
            }
            while m > 12 {
                m -= 12;
                app.year += 1;
            }
            app.month = m as u32;
            app.selected_day = None;
            Command::none()
        }
        Msg::SetSlide(i) => {
            app.slide = i;
            Command::none()
        }
        Msg::ToggleInfo => {
            app.info_open = !app.info_open;
            Command::none()
        }
        Msg::TagInput(s) => {
            app.tag_draft = s;
            Command::none()
        }
        Msg::TagPick(s) => {
            app.tag_draft = s;
            Command::none()
        }
        Msg::Load => Command::perform(|| Msg::Loaded(load_todos(&todos_path()))),
        Msg::Loaded(items) => {
            app.todos = items
                .into_iter()
                .map(|(done, text)| {
                    let id = app.next_id;
                    app.next_id += 1;
                    Todo { id, text, done }
                })
                .collect();
            Command::none()
        }
        Msg::SetSection(i) => {
            app.section = i;
            // Choosing a section from the drawer closes it.
            app.drawer_open = false;
            Command::none()
        }
        Msg::SelectStat(i) => {
            app.stat_sel = i;
            app.stat_detail_open = true;
            Command::none()
        }
        Msg::CloseDetail => {
            app.stat_detail_open = false;
            Command::none()
        }
        Msg::SetDensity(d) => {
            app.density = d.clamp(0.8, 1.4);
            Command::none()
        }
        Msg::ToggleDrawer => {
            app.drawer_open = !app.drawer_open;
            Command::none()
        }
        Msg::ToggleSheet => {
            app.sheet_open = !app.sheet_open;
            Command::none()
        }
    }
}

impl Application for TodoApp {
    type Message = Msg;

    fn update(&mut self, message: Msg) -> Command<Msg> {
        reduce(self, message)
    }

    fn init(&mut self) -> Command<Msg> {
        // Starts the stopwatch and loads the persisted tasks at start-up.
        self.running = true;
        self.page = 1;
        self.year = 2026;
        self.month = 7;
        self.density = 1.0;
        // Demonstration data for the editable grid.
        self.grid = vec![
            vec![
                "Ada Lovelace".into(),
                "Engineer".into(),
                "ada@example.com".into(),
            ],
            vec![
                "Alan Turing".into(),
                "Cryptographer".into(),
                "alan@example.com".into(),
            ],
            vec![
                "Grace Hopper".into(),
                "Admiral".into(),
                "grace@example.com".into(),
            ],
        ];
        if self.restored {
            // Live-reload: the snapshot is the authority, do not overwrite it from disk.
            return Command::none();
        }
        Command::perform(|| Msg::Loaded(load_todos(&todos_path())))
    }

    /// Live-reload: the essentials of the state survive a recompilation — the tasks, the draft,
    /// the filter, the theme (light/dark + seed), the tab and the screen.
    fn save_state(&self) -> Option<Vec<u8>> {
        let mut out = String::from("frus-demo-state v1\n");
        out.push_str(&format!("light {}\n", self.light as u8));
        out.push_str(&format!("seed {}\n", self.seed_index));
        out.push_str(&format!("filter {}\n", filter_index(self.filter)));
        out.push_str(&format!("section {}\n", self.section));
        let route = match current_route(self) {
            Route::Home => 0,
            Route::Settings => 1,
            Route::Journal => 2,
            Route::Wizard => 3,
            Route::Grid => 4,
            Route::Charts => 5,
            Route::Data => 6,
            Route::Board => 7,
            Route::Tour => 8,
            // A task screen is not restored: the task it names may not exist any more,
            // and reopening a screen about nothing is worse than opening the list.
            Route::Task(_) => 0,
        };
        out.push_str(&format!("route {route}\n"));
        out.push_str(&format!("draft {}\n", self.draft));
        for todo in &self.todos {
            out.push_str(&format!("todo {}\t{}\n", todo.done as u8, todo.text));
        }
        Some(out.into_bytes())
    }

    /// Rehydrates an [`Application::save_state`] snapshot — tolerantly: any unknown line (from
    /// another version of the code) is ignored.
    fn restore_state(&mut self, bytes: &[u8]) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        let mut lines = text.lines();
        if lines.next() != Some("frus-demo-state v1") {
            return;
        }
        for line in lines {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "light" => self.light = value == "1",
                "seed" => self.seed_index = value.parse().unwrap_or(0),
                "filter" => self.filter = filter_from_index(value.parse().unwrap_or(0)),
                "section" => self.section = value.parse().unwrap_or(0),
                "route" => {
                    self.routes.clear();
                    match value {
                        "1" => self.routes.push(Route::Settings),
                        "2" => self.routes.push(Route::Journal),
                        "3" => self.routes.push(Route::Wizard),
                        "4" => self.routes.push(Route::Grid),
                        "5" => self.routes.push(Route::Charts),
                        "6" => self.routes.push(Route::Data),
                        "7" => self.routes.push(Route::Board),
                        "8" => self.routes.push(Route::Tour),
                        _ => {}
                    }
                }
                "draft" => self.draft = value.to_string(),
                "todo" => {
                    if let Some((done, text)) = value.split_once('\t') {
                        let id = self.next_id;
                        self.next_id += 1;
                        self.todos.push(Todo {
                            id,
                            text: text.to_string(),
                            done: done == "1",
                        });
                    }
                }
                _ => {}
            }
        }
        self.restored = true;
    }

    fn subscription(&self) -> Subscription<Msg> {
        // One tick per second while the stopwatch runs **and** the app is in the foreground: in
        // the background (`on_lifecycle` → `foreground = false`) the timer **suspends** (the
        // framework stops the subscription by diffing, then restarts it on the way back).
        if self.running && !self.background {
            Subscription::every(Duration::from_secs(1), |_| Msg::Tick)
        } else {
            Subscription::none()
        }
    }

    fn density(&self) -> f32 {
        if self.density > 0.0 {
            self.density
        } else {
            1.0
        }
    }

    fn on_resize(&mut self, width: f32, height: f32) {
        // Reacts to a change of size class: it closes the Stats detail when narrow.
        let class = SizeClass::from_width(width);
        if self.size_class != Some(class) {
            self.size_class = Some(class);
            if class == SizeClass::Compact {
                self.stat_detail_open = false;
            }
            eprintln!("[demo] size class: {class:?}");
        }
        // A further axis of responsiveness: portrait/landscape orientation.
        let orientation = Orientation::from_size(width, height);
        if self.orientation != Some(orientation) {
            self.orientation = Some(orientation);
            eprintln!("[demo] orientation : {orientation:?}");
        }
    }

    fn on_insets(&mut self, insets: WindowInsets) {
        // The total safe area: system bars **and** the soft keyboard — the content (input
        // fields included) stays above the keyboard.
        let safe = insets.safe();
        if self.insets != safe {
            self.insets = safe;
            eprintln!("[demo] insets : {safe:?}");
        }
    }

    fn on_lifecycle(&mut self, state: Lifecycle) {
        // A demonstration of the lifecycle contract (milestone 259): the stopwatch **suspends**
        // in the background and **resumes** in the foreground — as any app would cut its timers
        // and sensors. The trace also goes to logcat (so it can be checked on a device).
        eprintln!("[demo] lifecycle: {state:?}");
        // Suspends in the background (Paused/Detached); keeps the timer running on `Inactive`
        // (focus lost but still visible) — suspending belongs to `paused`, not to `inactive`.
        self.background = matches!(state, Lifecycle::Paused | Lifecycle::Detached);
    }

    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        // The safe area: the interface is built at the **inner** dimensions (the window minus
        // the system insets), then wrapped in a full-window background held off by `padding` —
        // the background runs under the bars, the content does not.
        let i = self.insets;
        let w = (width - i.left - i.right).max(0.0);
        let h = (height - i.top - i.bottom).max(0.0);
        let nav = build_view(self, theme, w, h);
        if i == Insets::ZERO {
            Box::new(nav)
        } else {
            Box::new(
                Container::new()
                    .width(width)
                    .height(height)
                    .color(theme.background)
                    .padding_each(i.top, i.right, i.bottom, i.left)
                    .child(nav),
            )
        }
    }

    fn theme(&self) -> Theme {
        let target = theme_of(self);
        match self.theme_from {
            Some(from) => from.lerp(&target, self.theme_progress),
            None => target,
        }
    }

    fn tick(&mut self, dt: f32) -> bool {
        let mut animating = false;

        // The stand-in reload behind the log list's pull-to-refresh.
        if self.journal_reloading > 0.0 {
            self.journal_reloading -= dt;
            if self.journal_reloading <= 0.0 {
                self.journal_reloading = 0.0;
                self.journal_reloads += 1;
            }
            animating = true;
        }

        // The theme fade.
        if self.theme_from.is_some() {
            self.theme_progress += dt / 0.25;
            if self.theme_progress >= 1.0 {
                self.theme_progress = 1.0;
                self.theme_from = None;
            } else {
                animating = true;
            }
        }

        // The screen transition: the controller samples the shared spring.
        if self.nav_from.is_some() {
            if self.nav.tick(dt) {
                animating = true;
            } else {
                self.nav_from = None;
            }
        }

        // The back gesture's settle (the same spring, primed by the finger's momentum).
        let mut commit_back = false;
        if let Some(g) = self.back.as_mut() {
            if let Some(settle) = g.settle.as_mut() {
                if settle.tick(dt) {
                    g.progress = settle.value();
                    animating = true;
                } else {
                    commit_back = g.commit;
                    self.back = None;
                }
            }
        }
        if commit_back {
            self.routes.pop();
        }

        animating
    }

    fn title(&self) -> String {
        "frus — Todo".to_string()
    }

    fn window_size(&self) -> Option<(f32, f32)> {
        Some((900.0, 680.0))
    }

    fn can_go_back(&self) -> bool {
        !self.routes.is_empty()
            && !self.confirm_clear
            && !self.data_confirm_delete
            && !self.menu_open
            && !self.drawer_open
            && !self.sheet_open
    }

    fn back_gesture(&mut self, progress: f32) {
        match self.back.as_mut() {
            Some(g) => g.progress = progress,
            None => {
                self.back = Some(BackGesture {
                    progress,
                    velocity: 0.0,
                    settle: None,
                    commit: false,
                })
            }
        }
    }

    fn back_gesture_end(&mut self, velocity: f32) {
        if let Some(g) = self.back.as_mut() {
            g.velocity = velocity;
            // An iOS-style projection: the position plus the momentum decide.
            let projected = g.progress + velocity * BACK_PROJECT;
            let commit = projected > BACK_COMMIT_POS && !self.routes.is_empty();
            g.commit = commit;
            // A spring settle from the current position, primed by the finger's momentum,
            // towards the target (committed `1` or cancelled `0`).
            let mut settle = AnimationController::unit();
            settle.set_value(g.progress);
            settle.spring_to(if commit { 1.0 } else { 0.0 }, nav_spring(), velocity);
            g.settle = Some(settle);
        }
    }
}

// --- View ---

/// The view's entry point: a `Navigator` around the current screen.
fn build_view(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Navigator<Msg> {
    // A back gesture in progress: it previews the pop, driven by the finger.
    if let Some(gesture) = &app.back {
        let progress = gesture.progress;
        let top = screen(current_route(app), app, theme, width, height);
        let below_route = app
            .routes
            .split_last()
            .and_then(|(_, rest)| rest.last().copied())
            .unwrap_or(Route::Home);
        let below = screen(below_route, app, theme, width, height);
        return Navigator::new(below, width, height).from(top, progress, false);
    }

    let current = screen(current_route(app), app, theme, width, height);
    match app.nav_from {
        Some(from) => Navigator::new(current, width, height).from(
            screen(from, app, theme, width, height),
            app.nav.value(),
            app.nav_forward,
        ),
        None => Navigator::new(current, width, height),
    }
}

/// Builds the screen matching a route.
fn screen(
    route: Route,
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Box<dyn Widget<Msg>> {
    match route {
        Route::Home => todo_screen(app, theme, width, height),
        Route::Settings => Box::new(settings_screen(app, theme, width, height)),
        Route::Journal => Box::new(journal_screen(app, theme, width, height)),
        Route::Wizard => wizard_screen(app, theme, width, height),
        Route::Grid => grid_screen(app, theme, width, height),
        Route::Charts => charts_screen(app, theme, width, height),
        Route::Data => data_screen(app, theme, width, height),
        Route::Board => board_screen(app, theme, width, height),
        Route::Tour => tour_screen(app, theme, width, height),
        Route::Task(id) => task_screen(app, theme, width, height, id),
    }
}

/// **Pure** validation of a grid cell: `Name` (col 0) is required, `Email` (col 2) must look
/// like an address. `None` = valid. It demonstrates `TextInput::error` per cell.
fn grid_cell_error(col: usize, value: &str) -> Option<&'static str> {
    match col {
        0 if value.trim().is_empty() => Some("Required"),
        2 if !value.is_empty() && !(value.contains('@') && value.contains('.')) => {
            Some("Invalid email")
        }
        _ => None,
    }
}

/// Total number of invalid cells in the grid (milestone 204/207) — it gates the submission.
fn grid_error_count(grid: &[Vec<String>]) -> usize {
    grid.iter()
        .flat_map(|row| {
            (0..3).filter(move |&c| {
                grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or("")).is_some()
            })
        })
        .count()
}

/// Every invalid cell `(row, column)`, in row-by-row order.
fn grid_faults(grid: &[Vec<String>]) -> Vec<(usize, usize)> {
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

/// The **first** invalid cell (milestone 210).
fn grid_first_error(grid: &[Vec<String>]) -> Option<(usize, usize)> {
    grid_faults(grid).first().copied()
}

/// The faulty cell **after** `after` (row-by-row order, wrapping at the end) — so every fault
/// can be cycled through (milestone 214). `after = None` returns the first one.
fn grid_next_error(grid: &[Vec<String>], after: Option<(usize, usize)>) -> Option<(usize, usize)> {
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

/// The **editable grid** screen: a `Table` whose every cell is an always-editable `TextInput`.
/// Tab / Shift+Tab moves from cell to cell (the shell's focusables), Enter moves down one row
/// (milestone 201). The headers sort (milestone 204, `on_sort`), invalid cells show an error,
/// and Enter on the last row creates a new one.
fn grid_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
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
        let mut cells: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>> = (0..3)
            .map(|c| {
                let value = row[c].clone();
                let w = COL_W[c] - 14.0;
                let err = grid_cell_error(c, &value);
                let factory: Box<dyn Fn() -> Box<dyn Widget<Msg>>> = Box::new(move || {
                    let mut input = TextInput::new(value.clone())
                        .width(w)
                        .size(15.0)
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
                    .child(Icon::new(IconName::Close).size(16.0).color(muted))
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
    let table_area = Scroll::new().axis(Axis::Both).flex(1.0).child(table);
    let body = column![table_area, actions, hint]
        .gap(16.0)
        .padding(24.0)
        .flex(1.0);
    let screen = column![NavBar::new("Editable grid").on_back(Msg::Pop), body]
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

/// The data table's static dataset (name, role, score) — milestone 237.
const DATA_PEOPLE: [(&str, &str, u32, &str); 12] = [
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

/// Semantic rank of a priority level (`Low < Medium < High`) — the **custom** sort key of the
/// data table's "Level" column (a text sort would order it alphabetically).
fn level_rank(s: &str) -> u8 {
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
fn data_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
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
                        .variant(Variant::Secondary)
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
    let table_area = Scroll::new().axis(Axis::Both).flex(1.0).child(table);
    // `flex(1.0)`: the body fills the height under the bar so the table region can stretch
    // (otherwise it falls back to its base size and leaves a large gap below).
    let body = column![table_area, detail, summary, hint]
        .gap(16.0)
        .padding(24.0)
        .flex(1.0);
    let screen = column![NavBar::new("Data table").on_back(Msg::Pop), body]
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
            Portal::new(content)
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

/// Titles of the Kanban columns (a demo, milestone 247).
const KANBAN_TITLES: [&str; 3] = ["To do", "Doing", "Done"];
/// The Kanban's starting cards, per column (a demo, milestone 247).
const KANBAN_SEED: [&[&str]; 3] = [
    &["Design API", "Write spec", "Triage bugs"],
    &["Build widget"],
    &["Kickoff", "Research"],
];

/// A **rich card** of the Kanban (milestone 249): the label on the left, a **×** delete button
/// on the right (`KanbanDelete(col, pos)`).
fn rich_card(label: &str, col: usize, pos: usize) -> Box<dyn Widget<Msg>> {
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
fn board_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let cols = app.kanban_cols();
    // Per-column vertical scrolling **with no explicit height** (milestone 266): the columns fill
    // the board's height (laid out in an ancestor with a defined height — here the bounded screen
    // and the horizontal Scroll below) and each column scrolls its cards through `flex(1)`. No
    // height has to be computed any more (the old `card_area_height` stopgap, milestone 264).
    let mut board = Kanban::new(Msg::KanbanMove)
        .on_add(Msg::KanbanAdd)
        .scrollable_columns();
    for (c, title) in KANBAN_TITLES.iter().enumerate() {
        let cards = cols.get(c).cloned().unwrap_or_default();
        let factories: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>> = cards
            .iter()
            .enumerate()
            .map(|(pos, label)| {
                let label = label.clone();
                Box::new(move || rich_card(&label, c, pos)) as Box<dyn Fn() -> Box<dyn Widget<Msg>>>
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
    let board_area = Scroll::new()
        .axis(Axis::Horizontal)
        .width(width)
        .flex(1.0)
        .child(Container::new().padding(24.0).child(board));
    let hint_bar = Container::new().width(width).padding(24.0).child(hint);
    let screen = column![
        NavBar::new("Kanban board").on_back(Msg::Pop),
        board_area,
        hint_bar
    ]
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

/// The walkthrough's panels: a glyph, a title, and a line of body text.
const TOUR_PAGES: [(&str, &str, &str); 4] = [
    (
        "\u{1F44B}",
        "Welcome",
        "Swipe sideways, or use the picker below. Both drive the same page.",
    ),
    (
        "\u{1F446}",
        "One panel at a time",
        "A release never rests between two panels: it springs to one of them.",
    ),
    (
        "\u{26A1}",
        "A flick is enough",
        "You need not drag a panel all the way across; a short flick turns it.",
    ),
    (
        "\u{2713}",
        "That is the tour",
        "The picker follows the finger as soon as the page reads as changed.",
    ),
];

/// One panel of the walkthrough. Built on demand — a page that is off screen does
/// not exist — so it takes the theme by value rather than borrowing the frame's.
fn tour_panel(index: usize, theme: Theme) -> Container<Msg> {
    let (glyph, title, body) = TOUR_PAGES[index];
    // Every other panel takes the surface colour, so a swipe is visible even at the
    // moment the two panels are half and half.
    let background = if index % 2 == 0 {
        theme.surface
    } else {
        theme.background
    };
    Container::new().color(background).padding(32.0).child(
        column![
            text(glyph).size(56.0),
            text(title).size(24.0).weight(FontWeight::Bold),
            text(body).size(15.0).color(theme.muted).wrap(),
        ]
        .gap(16.0)
        .align(Align::Center)
        .justify(Justify::Center),
    )
}

/// A paged walkthrough: the finger and the picker drive **one** page number, held by
/// the application (milestone 283).
///
/// This is the whole point of the two-way binding: `on_page_changed` writes the page
/// the finger reached into the state, and `page` reads it back out. Neither side owns
/// it, so neither can drift from the other.
/// One task on its own screen (milestone 286).
///
/// The avatar carries the **same** `Hero` tag as the one on the row this screen was
/// opened from, so the two are understood to be one thing and the transition flies it
/// from the row into place instead of fading one out and the other in.
fn task_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
    id: u64,
) -> Box<dyn Widget<Msg>> {
    let todo = app.todos.iter().find(|t| t.id == id);
    let (label, done) = match todo {
        Some(todo) => (todo.text.clone(), todo.done),
        // Deleted while its screen was open: say so rather than show an empty page.
        None => ("This task no longer exists.".to_string(), false),
    };
    let avatar = Hero::new(id, Avatar::new(label.clone()).size(96.0));
    let state = if done { "Done" } else { "Still to do" };
    let body = column![
        avatar,
        text(label).size(24.0).weight(FontWeight::Bold).wrap(),
        text(state).size(15.0).color(theme.muted),
    ]
    .gap(18.0)
    .align(Align::Center)
    .justify(Justify::Center)
    .flex(1.0);

    let screen = column![
        NavBar::new("Task").on_back(Msg::Pop),
        Container::new().width(width).padding(24.0).flex(1.0).child(body)
    ]
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

fn tour_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let last = TOUR_PAGES.len() - 1;
    let page = app.tour_page.min(last);
    let palette = *theme;
    let pages = PageView::new(TOUR_PAGES.len(), move |index| tour_panel(index, palette))
        .width(width)
        .flex(1.0)
        .page(page)
        .on_page_changed(Msg::TourPage);

    let picker = Pagination::new(page + 1, TOUR_PAGES.len(), |p| Msg::TourPage(p - 1));
    let position = text(format!("Panel {} of {}", page + 1, TOUR_PAGES.len()))
        .size(13.0)
        .color(theme.muted);
    let footer = Container::new()
        .width(width)
        .padding(20.0)
        .child(column![picker, position].gap(10.0).align(Align::Center));

    let screen = column![
        NavBar::new("Guided tour").on_back(Msg::Pop),
        pages,
        footer
    ]
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

/// Categories (the x axis) of the chart dashboard.
const CHART_CATS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
/// The dashboard's series: `(name, values)`. Series 0 = the theme's accent; 1.. = `CHART_COLORS`.
const CHART_SERIES: [(&str, [f32; 5]); 3] = [
    ("Sales", [3.0, 7.0, 5.0, 8.0, 4.0]),
    ("Costs", [2.0, 4.0, 3.0, 5.0, 2.0]),
    ("Profit", [1.0, 3.0, 2.0, 3.0, 2.0]),
];
/// Colours of the **extra** series (1..; series 0 takes the theme's accent).
const CHART_COLORS: [Color; 2] = [
    Color {
        r: 220.0 / 255.0,
        g: 120.0 / 255.0,
        b: 80.0 / 255.0,
        a: 1.0,
    },
    Color {
        r: 90.0 / 255.0,
        g: 158.0 / 255.0,
        b: 242.0 / 255.0,
        a: 1.0,
    },
];

/// Builds the dashboard's chart according to `app.chart_kind` (milestone 219): lines (0),
/// stacked areas (1), grouped bars (2), stacked bars (3). Every variant shares the same data,
/// the same axis and the same `chart_hidden` visibility state. `legend` wires up (or leaves out)
/// the clickable legend — useful for a **companion** chart that does not repeat its own.
fn dashboard_chart(app: &TodoApp, kind: usize, height: f32, legend: bool) -> Box<dyn Widget<Msg>> {
    let hidden = app.chart_hidden.clone();
    let cats = (0..5).map(|i| (CHART_CATS[i], CHART_SERIES[0].1[i]));
    if kind < 2 {
        let mut c = LineChart::new(cats)
            .height(height)
            .grid(4)
            .name(CHART_SERIES[0].0)
            .series(CHART_SERIES[1].0, CHART_COLORS[0], CHART_SERIES[1].1)
            .series(CHART_SERIES[2].0, CHART_COLORS[1], CHART_SERIES[2].1)
            .hidden(hidden)
            .animated(true);
        if kind == 1 {
            c = c.stacked(true).normalized(app.chart_normalized);
        }
        if legend {
            // The main chart: a clickable legend + clickable points (milestone 221) + the
            // selected point highlighted (milestone 223).
            c = c
                .legend(true)
                .on_legend(Msg::ChartToggleSeries)
                .on_point(Msg::ChartPoint)
                .selected(app.chart_sel);
        }
        Box::new(c)
    } else {
        let mut c = BarChart::new(cats)
            .height(height)
            .grid(4)
            .name(CHART_SERIES[0].0)
            .series(CHART_SERIES[1].0, CHART_COLORS[0], CHART_SERIES[1].1)
            .series(CHART_SERIES[2].0, CHART_COLORS[1], CHART_SERIES[2].1)
            .hidden(hidden);
        if kind == 3 {
            c = c.stacked(true).normalized(app.chart_normalized);
        }
        if legend {
            // The main chart: a clickable legend + clickable bars (milestone 222) + the selected
            // bar highlighted (milestone 223).
            c = c
                .legend(true)
                .on_legend(Msg::ChartToggleSeries)
                .on_point(Msg::ChartPoint)
                .selected(app.chart_sel);
        }
        Box::new(c)
    }
}

/// The **chart dashboard** screen: a `SegmentedControl` picks the kind (lines / stacked areas /
/// grouped bars / stacked bars, milestone 219), and the **clickable** legend hides or shows a
/// series (milestone 215/218). It demonstrates routing sub-region clicks into the state.
fn charts_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let selector = SegmentedControl::new(app.chart_kind, Msg::SetChartKind)
        .segment("Lines")
        .segment("Stacked area")
        .segment("Grouped bars")
        .segment("Stacked bars");
    // The **100%** toggle (milestone 224): only shown for the stacked kinds (stacked areas/bars),
    // where normalising means something.
    let stacked_kind = app.chart_kind == 1 || app.chart_kind == 3;
    let normalize_row: Box<dyn Widget<Msg>> = if stacked_kind {
        Box::new(
            row![
                text("100% stacking").size(13.0).color(theme.muted),
                Switch::new(app.chart_normalized).on_toggle(Msg::SetChartNormalized)
            ]
            .gap(10.0)
            .align(Align::Center),
        )
    } else {
        // Nothing to show: an empty box says that more plainly than a zero-sized
        // container with a colour it never uses.
        Box::new(SizedBox::empty())
    };
    let chart = dashboard_chart(app, app.chart_kind, 240.0, true);
    // The **companion** chart: the complementary family (bars when the main one is lines, and the
    // other way round), without a legend of its own — it shares `chart_hidden`, so hiding a series
    // through the main chart's legend hides it here **too** (milestone 220).
    let companion_kind = if app.chart_kind < 2 { 2 } else { 0 };
    let companion = dashboard_chart(app, companion_kind, 150.0, false);
    let hint = text(
        "Click a legend entry to toggle a series; click a point to pin it, or again to unpin.",
    )
    .size(13.0)
    .color(theme.muted)
    .wrap();
    // The pinned detail of the last clicked point (milestone 221).
    let pinned: Box<dyn Widget<Msg>> = match &app.chart_pin {
        Some(detail) => Box::new(Chip::new(detail.clone())),
        None => Box::new(text("No point selected").size(13.0).color(theme.muted)),
    };
    let content = column![
        row![selector].align(Align::Center),
        normalize_row,
        chart,
        row![pinned].align(Align::Center),
        text("Companion view").size(13.0).color(theme.muted),
        companion,
        hint
    ]
    .gap(16.0)
    .padding(24.0);
    // Tall fixed content (the charts + the companion, ≈ 550-650 px): it scrolls **vertically** under the bar.
    let body = Scroll::new().width(width).flex(1.0).child(content);
    let screen = column![NavBar::new("Charts").on_back(Msg::Pop), body]
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

/// The wizard's form: a **pure** validation of the current state (milestones 180–181). The order
/// declares `password` before `confirm` (the cross-field `matches` validation).
fn wizard_form(app: &TodoApp) -> Form {
    Form::new()
        .field(
            "name",
            app.wizard_name.as_str(),
            Rule::required("Name is required"),
        )
        .field(
            "email",
            app.wizard_email.as_str(),
            Rule::all([
                Rule::required("Email is required"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field(
            "password",
            app.wizard_pass.as_str(),
            Rule::min_len(8, "Password must be at least 8 characters"),
        )
        .matches(
            "confirm",
            app.wizard_confirm.as_str(),
            "password",
            "Passwords do not match",
        )
}

/// Which step (0 = Account, 1 = Security) the field `key` lives on — so that clicking an error
/// summary bullet jumps to the right step (milestones 181 + 183).
fn wizard_step_of(key: &str) -> usize {
    match key {
        "name" | "email" => 0,
        _ => 1,
    }
}

/// A wizard field's index (its focus key) — for `keyed`/`Command::focus`.
fn wizard_field_of(key: &str) -> u8 {
    match key {
        "name" => 0,
        "email" => 1,
        "password" => 2,
        _ => 3,
    }
}

/// Is step `step` **valid**? (so "Next" is only allowed once the step is filled in.)
fn wizard_step_valid(form: &Form, step: usize) -> bool {
    match step {
        0 => form.error("name").is_none() && form.error("email").is_none(),
        1 => form.error("password").is_none() && form.error("confirm").is_none(),
        _ => form.is_valid(),
    }
}

/// One wizard field: its error is shown **only after** submission, its value is **masked** for a
/// password, and it carries a **focus key** (`keyed`) so the summary can jump to it.
fn wizard_input(
    form: &Form,
    submitted: bool,
    label: &str,
    value: &str,
    key: &str,
    field: u8,
    obscure: bool,
    eye: Option<bool>,
    field_width: f32,
) -> impl Widget<Msg> + 'static {
    let mut input = TextInput::new(value)
        .width(field_width)
        .size(16.0)
        .label(label)
        .obscure(obscure)
        .on_input(move |s| Msg::WizardInput(field, s));
    // `eye = Some(revealed)`: an eye icon **inside the field** toggles the masking (milestone 198).
    if let Some(revealed) = eye {
        let icon = if revealed {
            IconName::EyeOff
        } else {
            IconName::Eye
        };
        input = input.suffix_icon(icon).on_suffix(Msg::WizardToggleReveal);
    }
    if submitted {
        if let Some(err) = form.error(key) {
            input = input.error(err);
        }
    }
    keyed(("wizard", field), input)
}

/// The **sign-up wizard** screen: proof that the recent building blocks fit together — a
/// clickable [`Steps`] indicator (milestone 183), a validated [`Form`] (180) with a **clickable**
/// error summary (181), and a success notification (185/188).
fn wizard_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let form = wizard_form(app);
    let submitted = app.wizard_submitted;
    // A **responsive** field width: it fits the width (minus the 24×2 padding), capped at 360 px
    // so it does not stretch on a large screen.
    let field_w = (width - 48.0).clamp(240.0, 360.0);

    // Steps are marked "done" by **validity** (milestone 195), not merely by position — which
    // matches "Next" being gated by that same validity.
    let steps = Steps::new(["Account", "Security", "Review"])
        .current(app.wizard_step)
        .completed([
            wizard_step_valid(&form, 0),
            wizard_step_valid(&form, 1),
            form.is_valid(),
        ])
        .on_tap(Msg::WizardStep);

    // The current step's content.
    let content: Box<dyn Widget<Msg>> = match app.wizard_step {
        0 => Box::new(
            Flex::column()
                .gap(14.0)
                .child(wizard_input(
                    &form,
                    submitted,
                    "Full name",
                    &app.wizard_name,
                    "name",
                    0,
                    false,
                    None,
                    field_w,
                ))
                .child(wizard_input(
                    &form,
                    submitted,
                    "Email",
                    &app.wizard_email,
                    "email",
                    1,
                    false,
                    None,
                    field_w,
                )),
        ),
        1 => {
            // Passwords are masked unless revealed: the eye icon **inside the field** toggles it (198).
            let obscure = !app.wizard_reveal;
            let eye = Some(app.wizard_reveal);
            Box::new(
                Flex::column()
                    .gap(14.0)
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Password",
                        &app.wizard_pass,
                        "password",
                        2,
                        obscure,
                        eye,
                        field_w,
                    ))
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Confirm password",
                        &app.wizard_confirm,
                        "confirm",
                        3,
                        obscure,
                        eye,
                        field_w,
                    )),
            )
        }
        _ => {
            let mut review = Flex::column().gap(14.0);
            // A clickable summary: each bullet jumps to the faulty field's step **and** focuses
            // it (milestones 181 + 183 + programmatic focus).
            if submitted && !form.is_valid() {
                let links = form.errors().into_iter().map(|(key, message)| {
                    (
                        message.to_string(),
                        Msg::WizardFocus(wizard_step_of(key), wizard_field_of(key)),
                    )
                });
                review = review.child(ErrorSummary::links(links));
            }
            review = review.child(
                text(format!(
                    "Creating account for {} <{}>",
                    if app.wizard_name.is_empty() {
                        "—"
                    } else {
                        app.wizard_name.as_str()
                    },
                    if app.wizard_email.is_empty() {
                        "—"
                    } else {
                        app.wizard_email.as_str()
                    },
                ))
                .size(16.0)
                .wrap(),
            );
            Box::new(review)
        }
    };

    // The navigation bar: Back / Next, or Create on the last step.
    let mut nav = Flex::row().gap(12.0);
    if app.wizard_step > 0 {
        nav = nav.child(
            button("Back", Msg::WizardBack)
                .variant(Variant::Secondary)
                .size(16.0),
        );
    }
    if app.wizard_step < 2 {
        // "Next" only becomes active once the current step is valid (milestone 191: a disabled Button).
        nav = nav.child(
            button("Next", Msg::WizardNext)
                .variant(Variant::Primary)
                .size(16.0)
                .enabled(wizard_step_valid(&form, app.wizard_step)),
        );
    } else {
        nav = nav.child(
            button("Create account", Msg::WizardSubmit)
                .variant(Variant::Primary)
                .size(16.0),
        );
    }

    // A Scaffold, for what a form wants from one (milestone 288): Back / Next go in
    // the **persistent footer**, so they stay put while the steps scroll and are not
    // hunted for at the end of a long form; and the body is shortened by the keyboard
    // rather than covered by it, which is the default and is what a form needs.
    let inner = column![steps, content].gap(24.0).padding(24.0);
    Scaffold::new(width, height)
        .background(theme.background)
        .app_bar(NavBar::new("Sign-up wizard").on_back(Msg::Pop))
        .body(inner)
        .persistent_footer(nav)
        .build()
}

/// The "Journal" screen: a **virtualised list** of 5000 rows, and the place where
/// the two scroll behaviours can be compared by hand (milestone 277).
fn journal_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let t = *theme; // Theme is Copy — captured by the item factory.
    let mut list = List::new(5000, 44.0, move |i| {
        Container::<Msg>::new()
            .height(44.0)
            .radius(8.0)
            .color(if i % 2 == 0 { t.surface } else { t.background })
            .border(1.0, t.border)
            .padding_each(12.0, 14.0, 12.0, 14.0)
            .child(text(format!("Row {}", i + 1)).size(16.0))
    })
    .width((width - 48.0).max(200.0))
    .height((height - 152.0).max(160.0));
    // Unset, the list follows the platform. The toggle overrides it, which is the
    // point of the demonstration: fling to an end and feel the difference.
    if app.journal_bounces {
        list = list.physics(ScrollPhysics::Bouncing);
    }
    let label = if app.journal_bounces {
        "Edges: bounce"
    } else {
        "Edges: platform default"
    };
    // Pull the list past its top edge to reload it. The indicator spins for exactly as
    // long as the application says it is working — here, a countdown in `tick`.
    let pullable = Refresh::new(list)
        .on_refresh(Msg::ReloadJournal)
        .refreshing(app.journal_reloading > 0.0);
    let reloads = if app.journal_reloads == 0 {
        "Pull to reload".to_string()
    } else {
        format!("Reloaded {}×", app.journal_reloads)
    };
    let content = column![
        row![
            text(label).size(14.0).color(theme.muted),
            spacer(),
            text(reloads).size(14.0).color(theme.muted),
            button("Switch", Msg::ToggleScrollPhysics).variant(Variant::Secondary),
        ]
        .gap(12.0),
        pullable,
    ]
    .gap(12.0)
    .padding(24.0);
    let screen = column![NavBar::new("Log · 5000 rows").on_back(Msg::Pop), content]
        .width(width)
        .height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}

/// A statistic tile (a big number + a label) for the grid.
fn stat_tile(theme: &Theme, label: &str, value: usize) -> Container<Msg> {
    Container::new()
        .height(64.0)
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(10.0, 12.0, 10.0, 12.0)
        .child(column![
            text(value.to_string()).size(24.0),
            text(label.to_string()).size(13.0).color(theme.muted),
        ])
}

/// The "Settings" screen: the card of controls (it demonstrates navigation + gesture + widgets).
fn settings_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let volume_pct = (app.volume * 100.0).round() as u32;
    let controls = Card::new().child(
        column![
            row![
                text("Notifications").size(18.0),
                spacer(),
                Switch::new(app.notifs).on_toggle(Msg::SetNotifs),
            ]
            .align(Align::Center)
            .gap(12.0),
            row![
                text(format!("Volume: {volume_pct}%")).size(18.0),
                Slider::new(app.volume)
                    .width(220.0)
                    .on_change(Msg::SetVolume),
            ]
            .align(Align::Center)
            .gap(12.0),
            RadioGroup::new(app.radio, Msg::SetRadio)
                .option("Small")
                .option("Medium")
                .option("Large"),
            Dropdown::new(MENU[app.menu_choice], Msg::ToggleMenu).options(
                app.menu_open,
                &MENU,
                Msg::SetMenu,
            ),
            row![
                text("Your rating").size(18.0),
                spacer(),
                Rating::new(app.rating, 5, Msg::SetRating),
            ]
            .align(Align::Center)
            .gap(12.0),
            row![
                text("Quantity").size(18.0),
                spacer(),
                Stepper::new(app.count, Msg::SetCount).range(0, 20).step(1),
            ]
            .align(Align::Center)
            .gap(12.0),
            Divider::new(),
            row![
                text("Weekdays only").size(16.0),
                spacer(),
                Switch::new(app.weekdays_only).on_toggle(Msg::SetWeekdaysOnly),
            ]
            .align(Align::Center)
            .gap(12.0),
            demo_calendar(app),
        ]
        .gap(14.0),
    );
    let total = app.todos.len();
    let done = app.todos.iter().filter(|t| t.done).count();
    // The tab's usable width (the viewport minus the column/tab paddings), bounded: the showcases
    // adapt to Compact instead of overflowing.
    let inner_w = (width - 72.0).clamp(240.0, 480.0);
    let stats = Grid::new(3)
        .gap(10.0)
        .width(inner_w)
        .cell(stat_tile(theme, "Total", total))
        .cell(stat_tile(theme, "Active", total - done))
        .cell(stat_tile(theme, "Done", done));
    let facts = Table::new(2)
        .width(inner_w)
        .header(&["Metric", "Value"])
        .row(&["Widgets", "35"])
        .row(&["Milestones", "39"]);

    // The file tree (expanded according to the state).
    let open = |id: u64| app.expanded.contains(&id);
    // The chevron expands/collapses; the row's body selects the node (milestone 246).
    let mut tree = Tree::new(Msg::ToggleNode)
        .on_select(Msg::SelectNode)
        .selected(app.tree_selected)
        .node(1, 0, "src", true, open(1));
    if open(1) {
        tree = tree.node(2, 1, "widgets", true, open(2));
        if open(2) {
            tree = tree
                .node(3, 2, "button.rs", false, false)
                .node(4, 2, "grid.rs", false, false);
        }
        tree = tree.node(5, 1, "main.rs", false, false);
    }
    tree = tree.node(6, 0, "Cargo.toml", false, false);

    // The colour palette.
    let palette = [
        Color::rgb8(46, 160, 96),
        Color::rgb8(90, 158, 242),
        Color::rgb8(210, 96, 96),
        Color::rgb8(240, 180, 40),
        Color::rgb8(160, 110, 220),
        Color::rgb8(80, 200, 200),
    ];
    let mut picker = ColorPicker::new(app.picked, 6, Msg::PickColor);
    for color in palette {
        picker = picker.swatch(color);
    }

    // A timeline of the recent milestones.
    let timeline = Timeline::new()
        .event("Grid", "Milestone 35")
        .event("New widgets", "Milestones 36–37")
        .event("Hierarchy & color", "Milestone 38");

    // The carousel: the current slide is supplied by index.
    let slide = match app.slide {
        0 => text("Welcome to frus").size(16.0),
        1 => text("About 35 widgets").size(16.0),
        _ => text("Thanks for trying!").size(16.0),
    };
    let carousel = Carousel::new(app.slide, 3, Msg::SetSlide, slide);

    // An info popover (arbitrary content, dismissed by an outside click).
    let info = Popover::new(
        button("Info", Msg::ToggleInfo)
            .variant(Variant::Secondary)
            .size(15.0),
        app.info_open,
        Msg::ToggleInfo,
    )
    .content(
        Card::new().padding(16.0).child(
            column![
                text("Popover").size(16.0),
                text("An arbitrary floating panel; closes on outside click.")
                    .size(14.0)
                    .color(theme.muted),
            ]
            .gap(6.0),
        ),
    );

    // Autocomplete: suggestions filtered by what is typed (controlled).
    const TAGS: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];
    let mut tags = Autocomplete::new(app.tag_draft.clone(), Msg::TagInput, Msg::TagPick);
    if !app.tag_draft.is_empty() {
        let q = app.tag_draft.to_lowercase();
        for tag in TAGS {
            if tag.starts_with(&q) {
                tags = tags.suggestion(tag);
            }
        }
    }

    // Keyboard shortcut hints.
    let shortcuts = row![
        text("Shortcuts:").size(14.0).color(theme.muted),
        Kbd::new("Enter"),
        text("add").size(14.0).color(theme.muted),
        Kbd::new("Tab"),
        text("navigate").size(14.0).color(theme.muted),
    ]
    .align(Align::Center)
    .gap(6.0);
    let about = column![
        text("frus — widget showcase").size(18.0),
        row![info, tags].align(Align::Start).gap(12.0),
        shortcuts,
        stats,
        facts,
        carousel,
        Pagination::new(app.page, 8, Msg::SetPage),
        column![
            Skeleton::new().width(inner_w),
            Skeleton::new().width(inner_w * 0.8).height(14.0),
        ]
        .gap(8.0),
        Divider::new(),
        Collapsible::new("Advanced options", app.advanced_open, Msg::ToggleAdvanced).content(
            column![
                text("Explorer, palette, timeline:")
                    .size(15.0)
                    .color(theme.muted),
                tree,
                picker,
                timeline,
                row![Chip::new("beta"), Chip::new("experimental")].gap(8.0),
            ]
            .gap(10.0)
        ),
    ]
    .gap(12.0);
    let tabs = Tabs::new(app.settings_tab, Msg::SetSettingsTab)
        .tab("Controls", controls)
        .tab("About", about);
    let content = column![
        Breadcrumb::new(|_| Msg::Pop)
            .crumb("Home")
            .crumb("Settings"),
        row![tabs].justify(Justify::Center),
    ]
    .padding(20.0)
    .gap(16.0);
    // The content (the calendar, the advanced options…) is taller than the screen: it scrolls
    // under the bar, which stays pinned.
    let body = Scroll::new().width(width).flex(1.0).child(content);
    let screen = column![NavBar::new("Settings").on_back(Msg::Pop), body]
        .width(width)
        .height(height);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(screen)
}

/// One task row, **swipeable**: dragging it sideways past 40 % of its width — or
/// flicking it — deletes it, the same thing the × and the long press already do. The
/// row's height is explicit because a `Dismissible` overlays its background under its
/// child, which makes it a layout leaf.
/// The row, made **liftable**: held down, it can be carried to one of the state zones
/// below the filters.
///
/// It lifts on a **hold**, not on the first movement, because the same finger on the
/// same row already means two other things — dragging sideways dismisses it, dragging
/// up and down scrolls the list. Three gestures on one row, told apart by what the
/// finger does rather than by what is on top.
fn todo_row_draggable(todo: &Todo, theme: &Theme) -> Draggable<Msg> {
    Draggable::new(todo_row_swipeable(todo, theme))
        .payload(todo.id)
        .long_press()
}

fn todo_row_swipeable(todo: &Todo, theme: &Theme) -> Dismissible<Msg> {
    Dismissible::new(todo_row(todo, theme))
        .height(TODO_ROW_HEIGHT)
        .on_dismiss(Msg::DeleteTodo(todo.id))
        .background(
            Container::new()
                .radius(10.0)
                .color(theme.error)
                .padding_each(0.0, 16.0, 0.0, 16.0)
                .child(row![text("Delete").size(16.0).color(theme.on_error)].align(Align::Center)),
        )
}

/// The height of a task row. Fixed, because a swipeable row is a layout leaf.
const TODO_ROW_HEIGHT: f32 = 62.0;

/// One task row: a checkbox, the label (dimmed **and struck through** when done) and a delete
/// button.
fn todo_row(todo: &Todo, theme: &Theme) -> Container<Msg> {
    let id = todo.id;
    let label_color = if todo.done {
        theme.muted
    } else {
        theme.on_surface
    };
    let mut label = text(todo.text.clone()).size(18.0).color(label_color);
    if todo.done {
        label = label.strikethrough();
    }
    let line = row![
        // The shared element: the same avatar, tagged by the task's id, appears bigger
        // on the task's own screen and flies between the two.
        Container::new()
            .on_click(Msg::OpenTask(id))
            .child(Hero::new(id, Avatar::new(todo.text.clone()).size(30.0))),
        Checkbox::new(todo.done).on_toggle(move |_| Msg::ToggleTodo(id)),
        label,
        spacer(),
        button("×", Msg::DeleteTodo(id))
            .variant(Variant::Danger)
            .size(15.0),
    ]
    .align(Align::Center)
    .gap(12.0);
    Container::new()
        // No long press here: the hold is what **lifts** the row for dragging
        // (`todo_row_draggable`), and one hold cannot mean two things. Deleting is the
        // ×, or a swipe.
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(line)
}

/// Content of the data table's bulk-delete confirmation modal (milestone 245).
fn data_confirm_content(count: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Delete selected rows?")
                .size(22.0)
                .weight(FontWeight::Medium),
            text(format!("{count} row(s) will be removed.")).size(16.0),
            row![
                button("Cancel", Msg::DataCancelDelete).variant(Variant::Secondary),
                button("Delete", Msg::DataDeleteChecked).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// Content of the "clear completed" confirmation modal.
fn confirm_content(done: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        column![
            text("Clear completed tasks?")
                .size(22.0)
                .weight(FontWeight::Medium),
            text(format!("{done} task(s) will be removed.")).size(16.0),
            row![
                button("Cancel", Msg::CancelClear).variant(Variant::Secondary),
                button("Delete", Msg::ConfirmClearDone).variant(Variant::Danger),
            ]
            .justify(Justify::Center)
            .gap(12.0),
        ]
        .gap(16.0),
    )
}

/// The main screen: the task list (the sample app itself).
fn todo_screen(app: &TodoApp, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
    let active = active_count(app);
    let done = done_count(app);

    // Responsiveness: the card widens with the window in steps. In Compact it follows the
    // available width, and the fields inside adapt to it.
    let class = SizeClass::from_width(width);
    // The card's **inner** width: the window minus the body's padding (24 × 2) and the card's own
    // (20 × 2) — otherwise the card overflows the Compact viewport and the whole screen scrolls
    // horizontally.
    let card_width = match class {
        SizeClass::Compact => (width - 88.0).max(240.0),
        SizeClass::Medium => 560.0,
        SizeClass::Expanded => 680.0,
    };

    // The header: an adaptive AppBar. A title and some actions are declared; it decides on its
    // own how many fit on the line and folds the rest into a "⋯" overflow menu, according to the
    // width — without ever branching on mobile/desktop.
    let theme_label = if app.light { "Dark" } else { "Light" };
    let timer_label = if app.running { "Pause" } else { "Resume" };
    // The title follows the active section (as a real app would) — the Tasks section is
    // localized (Fluent) for the i18n demo.
    let section_title = match app.section {
        1 => "Stats".to_string(),
        2 => "About".to_string(),
        _ => tr(app.lang, "app-title"),
    };
    let header = AppBar::new(section_title)
        .width(width)
        .leading(
            button("☰", Msg::ToggleDrawer)
                .variant(Variant::Secondary)
                .size(16.0),
        )
        .overflow(app.actions_open, Msg::ToggleActions)
        .action(timer_label, Msg::ToggleTimer)
        .action(theme_label, Msg::ToggleTheme)
        .action(seed_label(app), Msg::CycleSeed)
        .action(if app.rtl { "LTR" } else { "RTL" }, Msg::ToggleRtl)
        // The language toggle: the label shows the language being switched TO.
        .action(LANGS[(app.lang + 1) % LANGS.len()].0, Msg::CycleLang)
        .action("A+", Msg::SetDensity(app.density + 0.1))
        .action("A−", Msg::SetDensity(app.density - 0.1))
        .action("Log →", Msg::Push(Route::Journal))
        .action("Settings →", Msg::Push(Route::Settings))
        .action("Quick actions", Msg::ToggleSheet)
        .action("Save", Msg::Save)
        .action("Clear completed", Msg::AskClearDone)
        .build();

    // Input: a field (Enter submits) + an add button. A non-empty field carries a **clickable**
    // "✕" suffix icon that clears it (milestone 198: a positional click on the suffix).
    let mut draft_input = TextInput::new(app.draft.as_str())
        .width((card_width - 150.0).max(160.0))
        .size(18.0)
        .on_input(Msg::DraftChanged)
        .on_submit(Msg::AddTodo);
    if !app.draft.is_empty() {
        draft_input = draft_input
            .suffix_icon(IconName::Close)
            .on_suffix(Msg::ClearDraft);
    }
    let input_row = row![draft_input, button("Add", Msg::AddTodo)]
        .align(Align::Center)
        .gap(10.0);

    // The filters: a segmented control (single selection).
    let segmented = SegmentedControl::new(filter_index(app.filter), |i| {
        Msg::SetFilter(filter_from_index(i))
    })
    .segment(tr(app.lang, "filter-all"))
    .segment(tr(app.lang, "filter-active"))
    .segment(tr(app.lang, "filter-done"));
    let mut filters = row![segmented].align(Align::Center).gap(8.0);
    // The active filter (other than "All") is shown as a removable chip.
    if app.filter != Filter::All {
        let name = if app.filter == Filter::Active {
            "Active"
        } else {
            "Done"
        };
        filters = filters
            .child(spacer())
            .child(Chip::new(name).on_remove(Msg::SetFilter(Filter::All)));
    }

    // Two zones a held task can be carried to. They are `DragTarget`s and nothing else:
    // the highlight while a task hovers one is the target's own, from `Status`.
    let zone = |label: &str, done: bool, theme: &Theme| {
        DragTarget::new(
            Container::new()
                .flex(1.0)
                .padding(12.0)
                .radius(10.0)
                .color(theme.surface)
                .child(row![text(label).size(14.0).color(theme.muted)].justify(Justify::Center)),
        )
        .on_drop(move |payload| Msg::SetTodoDone(payload, done))
    };
    let zones = row![
        zone("↓ Mark active", false, theme),
        zone("✓ Mark done", true, theme)
    ]
    .gap(8.0);

    // The filtered list (or the empty state).
    let mut list = Flex::column().gap(8.0);
    let mut shown = 0;
    for todo in app.todos.iter().filter(|t| match app.filter {
        Filter::All => true,
        Filter::Active => !t.done,
        Filter::Done => t.done,
    }) {
        // A stable identity by `id`: the retained state (hover/animations) does not jump when a
        // task in the middle is deleted.
        list = list.child(keyed(todo.id, todo_row_draggable(todo, theme)));
        shown += 1;
    }
    if shown == 0 {
        list = column![text("Nothing to show for this filter.")
            .size(18.0)
            .italic()
            .color(theme.muted)];
    }
    // **Vertical** responsiveness: in a short window the hint is hidden to preserve the usable
    // height. The scrolling is handled by the Scaffold.
    let short = SizeClass::from_height(height) == SizeClass::Compact;

    // The footer: the counters + clear completed (with a modal confirmation).
    let clear_button = button("Clear completed", Msg::AskClearDone)
        .variant(Variant::Danger)
        .size(15.0);
    let clear = if app.confirm_clear {
        Portal::new(clear_button)
            .overlay(confirm_content(done), Placement::Center)
            .dismiss(Msg::CancelClear)
    } else {
        Portal::new(clear_button)
    };
    let total = app.todos.len().max(1);
    let pct = (done as f32 / total as f32 * 100.0).round() as u32;

    // A summary built from its ACTUAL box (LayoutBuilder). Long text (pluralised counters,
    // localized through Fluent) when there is room, short text when it is narrow — at a fixed
    // height.
    let muted = theme.muted;
    let lang = app.lang;
    let total = active + done;
    let summary = LayoutBuilder::new(move |size: Size| {
        let label = if size.width >= 360.0 {
            format!(
                "{} · {} · {pct}%",
                tr_n(lang, "task-count", total),
                tr_n(lang, "remaining", active)
            )
        } else {
            format!("{active}·{done}")
        };
        text(label).size(16.0).color(muted)
    })
    .flex(1.0)
    .height(20.0);
    let footer = row![
        summary,
        button("Load", Msg::Load)
            .variant(Variant::Secondary)
            .size(15.0),
        button("Save", Msg::Save)
            .variant(Variant::Secondary)
            .size(15.0),
        clear,
    ]
    .align(Align::Center)
    .gap(8.0);

    // The completion progress bar (done / total).
    let progress =
        ProgressBar::new(done as f32 / total as f32).width((card_width - 40.0).max(200.0));

    // The app's card, of responsive width, centred at the top of the screen. The body is built
    // incrementally so the hint can be left out when the window is short.
    let mut card_body = Flex::column().width(card_width).gap(16.0);
    if !short {
        // A **static** banner: a repaint boundary (milestone 88). It is replayed from the cache
        // on frames of pure interaction (hover, focus, scrolling elsewhere) instead of being
        // repainted every frame.
        card_body = card_body.child(
            Container::new().repaint_boundary().child(
                Alert::new("Press Enter to add a task; swipe from the left edge to go back.")
                    .title("Tip"),
            ),
        );
        // A row of vector icons (milestone 89) + a bitmap image (milestone 90): tessellated paths
        // coloured by the theme, and a GPU texture fitted with `Cover`. The widget showcase
        // (~360 px) is wider than the card on a phone, so it **scrolls horizontally** (at a fixed
        // height, the row's) rather than overflowing.
        let showcase = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(Icon::new(IconName::Check).color(theme.primary))
            .child(Icon::new(IconName::Star))
            .child(Icon::new(IconName::Heart))
            .child(Icon::new(IconName::Menu))
            .child(Icon::new(IconName::ChevronRight))
            .child(Image::new(demo_image(), 72.0, 48.0).fit(BoxFit::Cover))
            // A group-opacity layer (milestone 92): two overlapping squares, composited as one →
            // the overlap does not darken (no double-blending of the alpha).
            .child(CustomPaint::new(72.0, 48.0, |scene, bounds, theme| {
                scene.layer(0.55, |inner| {
                    let c = theme.primary;
                    inner.fill_rect(Rect::new(bounds.x + 6.0, bounds.y + 8.0, 32.0, 32.0), c);
                    inner.fill_rect(Rect::new(bounds.x + 30.0, bounds.y + 8.0, 32.0, 32.0), c);
                });
            }));
        card_body = card_body.child(
            Scroll::new()
                .axis(Axis::Horizontal)
                .width(card_width)
                .height(52.0)
                .child(showcase),
        );
    }
    // **Stable** identities (keys): the hint above is conditional — without keys, its
    // disappearance (an open keyboard → a short screen) shifts the siblings' positional ids and
    // the retained state (the field's focus!) jumps.
    card_body = card_body
        .child(keyed("draft-row", input_row))
        .child(keyed("filters", filters))
        .child(keyed("drop-zones", zones))
        .child(keyed("todo-list", list))
        .child(Divider::new())
        .child(progress)
        .child(footer);
    let card = Card::new().padding(20.0).child(card_body);
    let tasks_body = column![row![card].justify(Justify::Center)].padding(24.0);

    // The body follows the active section (the adaptive navigation lives in the Scaffold).
    let section: Box<dyn Widget<Msg>> = match app.section {
        1 => Box::new(stats_section(app, theme, class)),
        2 => Box::new(about_section(theme, width)),
        _ => Box::new(tasks_body),
    };

    // The screen's skeleton: the Scaffold pins the top bar and the navigation, scrolls the body,
    // and coordinates the drawer / sheet / FAB — a single entry point. The insets are already
    // handled by `view` (which passes safe dimensions), so the Scaffold simply pins itself inside
    // that viewport.
    let scaffold = Scaffold::new(width, height)
        .background(theme.background)
        .app_bar(header)
        .body(section)
        .nav(app.section, Msg::SetSection)
        .destination("✔", "Tasks")
        .badge(active as u32)
        .destination("▦", "Stats")
        .destination("★", "About")
        .end_drawer(
            drawer_menu(app, theme, active),
            app.drawer_open,
            Msg::ToggleDrawer,
        )
        .bottom_sheet(quick_actions_sheet(theme), app.sheet_open, Msg::ToggleSheet)
        .build();

    // The notification at the head of the queue floats above everything, anchored bottom-centre by
    // the `ToastHost` layer (milestone 188): it fades **in**, then fades **out** when it moves into
    // its exit before being removed (milestone 193).
    match app.snackbars.current() {
        Some(message) => {
            let host = ToastHost::new(ToastPosition::BottomCenter)
                .toast(Toast::new(message.clone()).success());
            let host = if app.snackbars.is_leaving() {
                host.fade_out(0.3)
            } else {
                host.fade_in(0.25)
            };
            Box::new(
                Stack::new()
                    .width(width)
                    .height(height)
                    .layer(scaffold)
                    .layer(host),
            )
        }
        None => scaffold,
    }
}

/// The modal sheet's content: a few quick actions.
fn quick_actions_sheet(theme: &Theme) -> Container<Msg> {
    Container::new().padding(20.0).child(
        Flex::column()
            .gap(12.0)
            .child(text("Quick actions").size(20.0).color(theme.on_surface))
            .child(
                button("💾  Save", Msg::Save)
                    .variant(Variant::Primary)
                    .size(16.0),
            )
            .child(
                button("🗑  Clear completed", Msg::AskClearDone)
                    .variant(Variant::Secondary)
                    .size(16.0),
            )
            .child(
                button("Close", Msg::ToggleSheet)
                    .variant(Variant::Secondary)
                    .size(16.0),
            ),
    )
}

/// The navigation drawer's content: a header + the destinations + settings.
fn drawer_menu(app: &TodoApp, theme: &Theme, active: usize) -> Container<Msg> {
    let entry = |icon: &str, label: &str, index: usize| {
        let variant = if app.section == index {
            Variant::Primary
        } else {
            Variant::Secondary
        };
        button(format!("{icon}  {label}"), Msg::SetSection(index))
            .variant(variant)
            .size(16.0)
    };
    Container::new().padding(16.0).child(
        column![
            text("frus").size(22.0),
            text("Navigation").size(13.0).color(theme.muted),
            Divider::new(),
            entry("✔", "Tasks", 0),
            entry("▦", "Stats", 1),
            entry("★", "About", 2),
            Divider::new(),
            text(format!("{active} task(s) pending"))
                .size(14.0)
                .color(theme.muted),
            button("Settings →", Msg::Push(Route::Settings))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Sign-up wizard →", Msg::Push(Route::Wizard))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Editable grid →", Msg::Push(Route::Grid))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Charts →", Msg::Push(Route::Charts))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Data table →", Msg::Push(Route::Data))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Guided tour →", Msg::Push(Route::Tour))
                .variant(Variant::Secondary)
                .size(15.0),
            button("Kanban board →", Msg::Push(Route::Board))
                .variant(Variant::Secondary)
                .size(15.0),
        ]
        .gap(12.0),
    )
}

/// Day of the week (0 = Sunday … 6 = Saturday) of a date (Sakamoto) — milestone 238.
fn weekday(y: i32, m: u32, d: u32) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    (((yy + yy / 4 - yy / 100 + yy / 400 + T[(m - 1) as usize] + d as i32) % 7 + 7) % 7) as u32
}

/// True when the date falls on a **weekend** (Saturday or Sunday).
fn is_weekend(y: i32, m: u32, d: u32) -> bool {
    matches!(weekday(y, m, d), 0 | 6)
}

/// The showcase calendar: `DatePicker::filtered`, greying out **weekends** when `weekdays_only`
/// is set (milestone 238), otherwise every day is clickable (`DatePicker::new`).
fn demo_calendar(app: &TodoApp) -> Box<dyn Widget<Msg>> {
    if app.weekdays_only {
        Box::new(DatePicker::filtered(
            app.year,
            app.month,
            app.selected_day,
            |(y, m, d)| !is_weekend(y, m, d),
            Msg::PickDay,
            Msg::NavMonth,
        ))
    } else {
        Box::new(DatePicker::new(
            app.year,
            app.month,
            app.selected_day,
            Msg::PickDay,
            Msg::NavMonth,
        ))
    }
}

/// The "Stats" section: a responsive master-detail layout (`TwoPane`). Side by side when large,
/// a single pane when narrow (tapping a metric opens the detail).
fn stats_section(app: &TodoApp, theme: &Theme, class: SizeClass) -> TwoPane<Msg> {
    let total = app.todos.len();
    let metrics = [
        ("Total tasks", total),
        ("Active tasks", active_count(app)),
        ("Completed", done_count(app)),
    ];

    // The master pane: the list of metrics (a selection).
    let mut cats = Flex::column().gap(6.0);
    for (i, (label, _)) in metrics.iter().enumerate() {
        let variant = if app.stat_sel == i {
            Variant::Primary
        } else {
            Variant::Secondary
        };
        cats = cats.child(
            button(*label, Msg::SelectStat(i))
                .variant(variant)
                .size(15.0),
        );
    }
    let list = Card::new().padding(12.0).child(cats);

    // The detail pane: the selected metric.
    let (label, value) = metrics[app.stat_sel.min(metrics.len() - 1)];
    let mut detail_col = column![
        text(label).size(22.0),
        text(value.to_string()).size(44.0).color(theme.primary),
        text("Detail for the selected metric.")
            .size(14.0)
            .color(theme.muted),
    ]
    .gap(10.0);
    // In single-pane mode, a way back to the list.
    if class != SizeClass::Expanded {
        detail_col = detail_col.child(
            button("← Back", Msg::CloseDetail)
                .variant(Variant::Secondary)
                .size(15.0),
        );
    }
    let detail = Card::new().padding(20.0).child(detail_col);

    TwoPane::new(class)
        .ratio(0.36)
        .show_detail(app.stat_detail_open)
        .list(list)
        .detail(detail)
}

/// The "About" section: static introductory content.
fn about_section(theme: &Theme, width: f32) -> Container<Msg> {
    // The content width = the viewport minus the paddings (the container's 24×2 + the card's
    // 20×2), bounded to a comfortable measure — otherwise it overflows horizontally in Compact.
    let content_width = (width - 88.0).max(240.0).min(560.0);
    Container::new().padding(24.0).child(
        Card::new().padding(20.0).child(
            column![
                text("About frus").size(24.0),
                // Rich text: mixed styles on one line, with cascading inheritance.
                RichText::new(
                    TextSpan::new("A ")
                        .child(TextSpan::new("fast").bold())
                        .child(TextSpan::new(", "))
                        .child(TextSpan::new("portable").italic().underline())
                        .child(TextSpan::new(" Rust UI framework — "))
                        .child(TextSpan::new("no GC").bold().color(theme.primary))
                        .child(TextSpan::new(".")),
                )
                .base_style(theme.text.body_medium.color(theme.muted))
                .wrap(),
                Divider::new(),
                Timeline::new()
                    .event("Responsive primitives", "Milestone 42")
                    .event("Adaptive navigation", "Milestone 43"),
                // A paragraph: it wraps at the card's width.
                text(
                    "Layout, painting, typography and animation are engine-level \
                     foundations shared by every widget in this gallery.",
                )
                .size(13.0)
                .color(theme.muted)
                .wrap(),
            ]
            .gap(12.0)
            .width(content_width),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::{build_ui, Runtime, Size};

    /// Adds a task from a label.
    fn add(app: &mut TodoApp, text: &str) {
        reduce(app, Msg::DraftChanged(text.to_string()));
        reduce(app, Msg::AddTodo);
    }

    fn primitive_count(app: &TodoApp) -> usize {
        let theme = Theme::default();
        let tree = build_view(app, &theme, 800.0, 600.0);
        build_ui(&tree, Size::new(800.0, 600.0), &Runtime::default(), &theme)
            .scene()
            .primitives()
            .len()
    }

    #[test]
    fn density_is_clamped() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::SetDensity(5.0));
        assert_eq!(app.density, 1.4);
        reduce(&mut app, Msg::SetDensity(0.1));
        assert_eq!(app.density, 0.8);
        // density() guards against an uninitialised state (0.0 → 1.0).
        app.density = 0.0;
        assert_eq!(Application::density(&app), 1.0);
    }

    #[test]
    fn on_resize_tracks_class_and_closes_detail_when_compact() {
        let mut app = TodoApp::default();
        app.stat_detail_open = true;
        // Wide: the Expanded class, the detail stays open.
        app.on_resize(1000.0, 700.0);
        assert_eq!(app.size_class, Some(SizeClass::Expanded));
        assert!(app.stat_detail_open);
        // Narrow: it switches to Compact and closes the detail.
        app.on_resize(500.0, 700.0);
        assert_eq!(app.size_class, Some(SizeClass::Compact));
        assert!(!app.stat_detail_open);
    }

    #[test]
    fn drawer_toggles_and_section_choice_closes_it() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::ToggleDrawer);
        assert!(app.drawer_open);
        // Choosing a section closes the drawer.
        reduce(&mut app, Msg::SetSection(1));
        assert_eq!(app.section, 1);
        assert!(!app.drawer_open);
        // Navigating (Push) closes the drawer too.
        reduce(&mut app, Msg::ToggleDrawer);
        reduce(&mut app, Msg::Push(Route::Settings));
        assert!(!app.drawer_open);
    }

    /// A long task's own screen: its title **wraps**, and what follows clears the
    /// lines it wrapped onto. The device found this one (milestone 289) — the title
    /// used to be painted on two lines with the state label sitting on the second.
    #[test]
    fn a_long_task_title_wraps_without_overlapping_what_follows() {
        let mut app = TodoApp::default();
        reduce(
            &mut app,
            Msg::DraftChanged("A rather long task name that certainly wraps".to_string()),
        );
        reduce(&mut app, Msg::AddTodo);
        let id = app.todos[0].id;
        reduce(&mut app, Msg::OpenTask(id));
        // Past the route transition, so the task's screen is the one on show.
        for _ in 0..40 {
            Application::tick(&mut app, 0.05);
        }
        let theme = Theme::dark();
        let tree = Application::view(&app, &theme, 424.0, 918.0);
        let ui = build_ui(
            tree.as_ref(),
            Size::new(424.0, 918.0),
            &Runtime::default(),
            &theme,
        );
        let texts: Vec<(String, f32, Option<f32>)> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_widgets::Primitive::Text {
                    position,
                    text,
                    max_width,
                    ..
                } => Some((text.clone(), position.y, *max_width)),
                _ => None,
            })
            .collect();
        let title = texts
            .iter()
            .find(|(t, _, _)| t.starts_with("A rather long"))
            .expect("the task's title is on its screen");
        let state = texts
            .iter()
            .find(|(t, _, _)| t == "Still to do")
            .expect("the state label is under it");
        // The title is wrapped: it is painted with a width narrower than one line of
        // it would need.
        assert!(
            title.2.is_some_and(|w| w < 400.0),
            "the title is a paragraph in a narrow box: {:?}",
            title.2
        );
        // Two lines at 24 px is about 58 px, plus the column's 18 px gap: 76. One line
        // would put the state label 46 px below. 60 separates the two cleanly, and it
        // is the failure this test exists for — the label used to land on the second
        // line, which the layout had not reserved.
        assert!(
            state.1 - title.1 > 60.0,
            "the state label overlaps the wrapped title: title y={}, state y={}",
            title.1,
            state.1
        );
    }

    #[test]
    fn on_insets_updates_safe_area() {
        let mut app = TodoApp::default();
        assert_eq!(app.insets, Insets::ZERO);
        // The system bars alone.
        app.on_insets(WindowInsets {
            padding: Insets::new(84.0, 0.0, 45.0, 0.0),
            view_insets: Insets::ZERO,
        });
        assert_eq!(app.insets, Insets::new(84.0, 0.0, 45.0, 0.0));
        // An open keyboard: the bottom safe area follows the keyboard (avoidance).
        app.on_insets(WindowInsets {
            padding: Insets::new(84.0, 0.0, 45.0, 0.0),
            view_insets: Insets::new(0.0, 0.0, 345.0, 0.0),
        });
        assert_eq!(app.insets, Insets::new(84.0, 0.0, 345.0, 0.0));
        // The view builds without panicking with non-zero insets (the wrapping path).
        let theme = Theme::dark();
        let tree = Application::view(&app, &theme, 400.0, 800.0);
        let ui = build_ui(
            tree.as_ref(),
            Size::new(400.0, 800.0),
            &Runtime::default(),
            &theme,
        );
        assert!(!ui.scene().primitives().is_empty());
    }

    #[test]
    fn sheet_toggles_and_action_closes_it() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::ToggleSheet);
        assert!(app.sheet_open);
        // A sheet action (Save) closes it.
        reduce(&mut app, Msg::Save);
        assert!(!app.sheet_open);
        // The same for "Clear completed" (which opens the confirmation).
        reduce(&mut app, Msg::ToggleSheet);
        reduce(&mut app, Msg::AskClearDone);
        assert!(!app.sheet_open);
    }

    #[test]
    fn on_resize_tracks_orientation() {
        let mut app = TodoApp::default();
        app.on_resize(400.0, 800.0);
        assert_eq!(app.orientation, Some(Orientation::Portrait));
        app.on_resize(900.0, 500.0);
        assert_eq!(app.orientation, Some(Orientation::Landscape));
    }

    #[test]
    fn clear_draft_empties_the_field() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::DraftChanged("half-typed".to_string()));
        assert_eq!(app.draft, "half-typed");
        // The "✕" suffix (a positional click) emits ClearDraft, which empties the field.
        reduce(&mut app, Msg::ClearDraft);
        assert!(app.draft.is_empty());
    }

    #[test]
    fn add_todo_from_draft_and_trims_blanks() {
        let mut app = TodoApp::default();
        add(&mut app, "Buy bread");
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "Buy bread");
        assert!(app.draft.is_empty(), "the field is emptied after the add");

        add(&mut app, "   ");
        assert_eq!(app.todos.len(), 1);
    }

    #[test]
    fn toggle_delete_and_clear_done() {
        let mut app = TodoApp::default();
        for t in ["a", "b", "c"] {
            add(&mut app, t);
        }
        let id_b = app.todos[1].id;
        reduce(&mut app, Msg::ToggleTodo(id_b));
        assert!(app.todos[1].done);
        assert_eq!(done_count(&app), 1);
        assert_eq!(active_count(&app), 2);

        let id_a = app.todos[0].id;
        reduce(&mut app, Msg::DeleteTodo(id_a));
        assert_eq!(app.todos.len(), 2);

        reduce(&mut app, Msg::ConfirmClearDone);
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "c");
    }

    #[test]
    fn view_builds_a_non_empty_scene() {
        let mut app = TodoApp::default();
        add(&mut app, "a task");
        assert!(primitive_count(&app) > 0);
    }

    #[test]
    fn wizard_flow_validates_navigates_and_notifies() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Wizard));
        assert_eq!(current_route(&app), Route::Wizard);
        assert!(primitive_count(&app) > 0, "the wizard screen renders");

        // The Account step starts invalid (which blocks "Next", milestone 191/192).
        assert!(
            !wizard_step_valid(&wizard_form(&app), 0),
            "Account starts invalid"
        );

        // An empty submission → the errors are revealed and it jumps to Review (the summary).
        reduce(&mut app, Msg::WizardSubmit);
        assert!(app.wizard_submitted);
        assert_eq!(app.wizard_step, 2);
        assert!(app.snackbars.is_empty());
        assert!(primitive_count(&app) > 0, "the error summary renders");

        // A summary bullet jumps to the field's step **and** asks for its focus.
        let cmd = reduce(&mut app, Msg::WizardFocus(0, 1));
        assert_eq!(app.wizard_step, 0);
        assert!(!cmd.is_empty(), "WizardFocus emits a focus request");

        // Filling Account in → the step becomes valid.
        reduce(&mut app, Msg::WizardInput(0, "Ada".to_string()));
        reduce(&mut app, Msg::WizardInput(1, "ada@example.com".to_string()));
        assert!(
            wizard_step_valid(&wizard_form(&app), 0),
            "Account is valid once filled in"
        );
        // Fill Security in (with matching passwords).
        reduce(&mut app, Msg::WizardInput(2, "secret12".to_string()));
        reduce(&mut app, Msg::WizardInput(3, "secret12".to_string()));
        assert!(wizard_step_valid(&wizard_form(&app), 1), "Security valide");
        reduce(&mut app, Msg::WizardSubmit);
        // Success: a notification + the wizard is reset.
        assert_eq!(
            app.snackbars.current().map(String::as_str),
            Some("Account created")
        );
        assert_eq!(app.wizard_step, 0);
        assert!(!app.wizard_submitted);
        assert!(app.wizard_name.is_empty() && app.wizard_email.is_empty());

        // Step navigation (Next / a direct jump / Back, all clamped).
        reduce(&mut app, Msg::WizardNext);
        assert_eq!(app.wizard_step, 1);
        reduce(&mut app, Msg::WizardStep(2));
        assert_eq!(app.wizard_step, 2);
        reduce(&mut app, Msg::WizardNext);
        assert_eq!(app.wizard_step, 2, "clamped to the last step");
        reduce(&mut app, Msg::WizardBack);
        assert_eq!(app.wizard_step, 1);
    }

    #[test]
    fn grid_edit_navigate_and_resize() {
        let mut app = TodoApp::default();
        app.grid = vec![
            vec![
                "Ada".to_string(),
                "Engineer".to_string(),
                "a@x.com".to_string(),
            ],
            vec![
                "Alan".to_string(),
                "Crypto".to_string(),
                "b@x.com".to_string(),
            ],
        ];
        reduce(&mut app, Msg::Push(Route::Grid));
        assert_eq!(current_route(&app), Route::Grid);
        assert!(primitive_count(&app) > 0, "the grid renders");
        // Typing in a cell updates the right box (the grid is always editable).
        reduce(&mut app, Msg::GridInput(0, 1, "Mathematician".to_string()));
        assert_eq!(app.grid[0][1], "Mathematician");
        assert_eq!(app.grid[0][0], "Ada", "the other cells are untouched");
        // Enter moves down one row (same column) and asks for the focus.
        let cmd = reduce(&mut app, Msg::GridEnter(0, 1));
        assert!(!cmd.is_empty(), "Enter focuses the cell below");
        // Enter on the last row creates one (milestone 204) and jumps into it.
        let last = app.grid.len() - 1;
        let before_enter = app.grid.len();
        let cmd = reduce(&mut app, Msg::GridEnter(last, 1));
        assert_eq!(
            app.grid.len(),
            before_enter + 1,
            "Enter on the last row creates one"
        );
        assert!(!cmd.is_empty(), "and puts the focus in it");
        // Adding a row: an empty row at the end, the focus on its 1st cell.
        let before = app.grid.len();
        let cmd = reduce(&mut app, Msg::GridAddRow);
        assert_eq!(app.grid.len(), before + 1);
        assert_eq!(
            app.grid[before],
            vec!["", "", ""],
            "a new empty row (3 columns)"
        );
        assert!(!cmd.is_empty(), "AddRow focuses the new row");
        // Deleting a row.
        reduce(&mut app, Msg::GridDeleteRow(0));
        assert_eq!(app.grid.len(), before, "one row removed");
    }

    #[test]
    fn grid_sort_toggles_and_validates() {
        let mut app = TodoApp::default();
        app.grid = vec![
            vec![
                "Charlie".to_string(),
                "QA".to_string(),
                "c@x.com".to_string(),
            ],
            vec![
                "Ada".to_string(),
                "Engineer".to_string(),
                "a@x.com".to_string(),
            ],
            vec!["Bob".to_string(), "PM".to_string(), "b@x.com".to_string()],
        ];
        // Sort column 0 ascending, then toggle to descending.
        reduce(&mut app, Msg::GridSort(0));
        assert_eq!(app.grid_sort, Some((0, true)));
        assert_eq!(app.grid[0][0], "Ada");
        assert_eq!(app.grid[2][0], "Charlie");
        reduce(&mut app, Msg::GridSort(0));
        assert_eq!(app.grid_sort, Some((0, false)));
        assert_eq!(app.grid[0][0], "Charlie");
        // Per-cell validation: an empty Name and a malformed email are flagged.
        assert_eq!(grid_cell_error(0, ""), Some("Required"));
        assert!(grid_cell_error(0, "Ada").is_none());
        assert_eq!(grid_cell_error(2, "not-an-email"), Some("Invalid email"));
        assert!(grid_cell_error(2, "a@x.com").is_none());
        assert!(
            grid_cell_error(2, "").is_none(),
            "an empty email is tolerated (nothing typed yet)"
        );
    }

    #[test]
    fn grid_save_is_gated_on_cell_errors() {
        let mut app = TodoApp::default();
        // One valid row, one with an empty Name and a malformed email (2 errors).
        app.grid = vec![
            vec![
                "Ada".to_string(),
                "Engineer".to_string(),
                "a@x.com".to_string(),
            ],
            vec!["".to_string(), "PM".to_string(), "nope".to_string()],
        ];
        assert_eq!(grid_error_count(&app.grid), 2);
        reduce(&mut app, Msg::GridSave);
        assert_eq!(
            app.snackbars.current().map(String::as_str),
            Some("Fix 2 errors before saving"),
            "the submission is blocked and counts the errors"
        );
        // Fix both cells: the grid becomes valid and the save goes through.
        app.grid[1][0] = "Bob".to_string();
        app.grid[1][2] = "b@x.com".to_string();
        assert_eq!(grid_error_count(&app.grid), 0);
        let mut app2 = TodoApp {
            grid: app.grid.clone(),
            ..TodoApp::default()
        };
        reduce(&mut app2, Msg::GridSave);
        assert_eq!(
            app2.snackbars.current().map(String::as_str),
            Some("Grid saved")
        );
    }

    #[test]
    fn grid_focus_error_targets_the_first_faulty_cell() {
        let mut app = TodoApp::default();
        // Row 0 is valid, row 1 has an empty Name (column 0) = the first fault expected.
        app.grid = vec![
            vec![
                "Ada".to_string(),
                "Engineer".to_string(),
                "a@x.com".to_string(),
            ],
            vec!["".to_string(), "PM".to_string(), "nope".to_string()],
        ];
        assert_eq!(grid_first_error(&app.grid), Some((1, 0)));
        assert!(
            !reduce(&mut app, Msg::GridFocusError).is_empty(),
            "it focuses the faulty cell"
        );
        // Everything valid: no target left, so no command.
        app.grid[1][0] = "Bob".to_string();
        app.grid[1][2] = "b@x.com".to_string();
        assert_eq!(grid_first_error(&app.grid), None);
        assert!(
            reduce(&mut app, Msg::GridFocusError).is_empty(),
            "nothing to focus"
        );
    }

    #[test]
    fn chart_legend_toggle_hides_and_shows_series() {
        let mut app = TodoApp::default();
        // The chart screen renders.
        reduce(&mut app, Msg::Push(Route::Charts));
        assert_eq!(current_route(&app), Route::Charts);
        assert!(primitive_count(&app) > 0, "the dashboard renders");
        // A legend click hides the series, a second click shows it again (a toggle).
        assert!(app.chart_hidden.is_empty());
        reduce(&mut app, Msg::ChartToggleSeries(1));
        assert_eq!(app.chart_hidden, vec![1], "a click hides series 1");
        reduce(&mut app, Msg::ChartToggleSeries(2));
        assert_eq!(app.chart_hidden, vec![1, 2]);
        reduce(&mut app, Msg::ChartToggleSeries(1));
        assert_eq!(
            app.chart_hidden,
            vec![2],
            "a second click shows series 1 again"
        );
    }

    #[test]
    fn chart_kind_selector_switches_type_and_each_renders() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert_eq!(app.chart_kind, 0, "lines by default");
        // Every kind (stacked areas, grouped bars, stacked bars) renders.
        for k in [1usize, 2, 3] {
            reduce(&mut app, Msg::SetChartKind(k));
            assert_eq!(app.chart_kind, k);
            assert!(primitive_count(&app) > 0, "kind {k} renders");
        }
    }

    #[test]
    fn clicking_a_point_pins_its_detail() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert!(app.chart_pin.is_none(), "nothing is pinned at the start");
        // Thu (index 3) of Sales (series 0) = 8.
        reduce(&mut app, Msg::ChartPoint(3, 0));
        assert_eq!(app.chart_pin.as_deref(), Some("Sales · Thu = 8"));
        // Tue (index 1) of Costs (series 1) = 4: it replaces the pin.
        reduce(&mut app, Msg::ChartPoint(1, 1));
        assert_eq!(app.chart_pin.as_deref(), Some("Costs · Tue = 4"));
        assert!(primitive_count(&app) > 0, "the screen with a pin renders");
    }

    #[test]
    fn tree_node_selection_toggles() {
        // The file tree (the Settings showcase): select a node, then click again to deselect.
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Settings));
        assert_eq!(app.tree_selected, None, "no node is selected at the start");
        reduce(&mut app, Msg::SelectNode(6));
        assert_eq!(app.tree_selected, Some(6), "a click = the node is selected");
        reduce(&mut app, Msg::SelectNode(1));
        assert_eq!(
            app.tree_selected,
            Some(1),
            "clic ailleurs = deplace la selection"
        );
        reduce(&mut app, Msg::SelectNode(1));
        assert_eq!(app.tree_selected, None, "re-clic = deselection");
        assert!(
            primitive_count(&app) > 0,
            "the showcase renders with a selection"
        );
    }

    #[test]
    fn kanban_move_relocates_a_card() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Board));
        assert_eq!(current_route(&app), Route::Board);
        let start = app.kanban_cols();
        // "Design API" sits at (0,0). Moving it to the head of "Doing" (column 1): it leaves
        // column 0 and appears at the head of column 1.
        reduce(&mut app, Msg::KanbanMove(0, 0, 1, 0));
        let after = app.kanban_cols();
        assert_eq!(
            after[0].len(),
            start[0].len() - 1,
            "the card leaves the source column"
        );
        assert_eq!(
            after[1][0], "Design API",
            "the card arrives at the head of the target column"
        );
        assert!(
            !after[0].contains(&"Design API".to_string()),
            "no longer in the source"
        );
        // A move within the SAME column: (1,0) to the end — the index shift is handled.
        let doing_len = after[1].len();
        reduce(&mut app, Msg::KanbanMove(1, 0, 1, doing_len));
        let end = app.kanban_cols();
        assert_eq!(
            end[1].len(),
            doing_len,
            "the same number of cards (reordered, not duplicated)"
        );
        assert_eq!(
            end[1].last().unwrap(),
            "Design API",
            "the card moved to the end of the column"
        );
        assert!(primitive_count(&app) > 0, "the board renders");
        // Adding / deleting (milestone 249): + Add card appends at the bottom; × deletes the card aimed at.
        let col0_len = app.kanban_cols()[0].len();
        reduce(&mut app, Msg::KanbanAdd(0));
        assert_eq!(app.kanban_cols()[0].len(), col0_len + 1, "Add adds a card");
        assert_eq!(
            app.kanban_cols()[0].last().unwrap(),
            "New card",
            "appended at the bottom of the column"
        );
        reduce(&mut app, Msg::KanbanDelete(0, 0));
        assert_eq!(
            app.kanban_cols()[0].len(),
            col0_len,
            "Delete removes a card"
        );
        assert!(
            primitive_count(&app) > 0,
            "it renders after adding/deleting"
        );
    }

    #[test]
    fn grouped_bars_are_clickable_in_dashboard() {
        // The main chart in **grouped bars** (kind 2) wires up `on_point` (milestone 222): at
        // least one point of the plot area emits `ChartPoint`. A sweep (independent of
        // frus-widgets' internal geometry constants).
        let app = TodoApp::default();
        let chart = dashboard_chart(&app, 2, 240.0, true);
        let (w, h) = (600.0, 240.0);
        let mut hit = false;
        let mut y = 30.0;
        while y < h - 22.0 && !hit {
            let mut x = 40.0;
            while x < w {
                if let Some(Msg::ChartPoint(_, _)) = chart.positional_click(x, y, w, h) {
                    hit = true;
                    break;
                }
                x += 4.0;
            }
            y += 4.0;
        }
        assert!(hit, "a dashboard bar emits ChartPoint");
    }

    #[test]
    fn clicking_a_point_marks_it_selected() {
        // The click pins not only the detail (milestone 221) but also the `(cat, series)`
        // selection highlighted in the chart (milestone 223).
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert_eq!(app.chart_sel, None, "rien de selectionne au depart");
        reduce(&mut app, Msg::ChartPoint(3, 0));
        assert_eq!(
            app.chart_sel,
            Some((3, 0)),
            "the clicked point becomes the selection"
        );
        reduce(&mut app, Msg::ChartPoint(1, 1));
        assert_eq!(
            app.chart_sel,
            Some((1, 1)),
            "the selection follows the last click"
        );
        assert!(
            primitive_count(&app) > 0,
            "the screen with a highlighted point renders"
        );
    }

    #[test]
    fn normalized_toggle_applies_to_stacked_kinds() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        assert!(!app.chart_normalized, "absolu par defaut");
        reduce(&mut app, Msg::SetChartNormalized(true));
        assert!(app.chart_normalized, "the toggle is on");
        // Both stacked kinds (stacked areas, kind 1, and stacked bars, kind 3) render in 100% mode.
        for k in [1usize, 3] {
            reduce(&mut app, Msg::SetChartKind(k));
            assert!(
                primitive_count(&app) > 0,
                "stacked kind {k} renders in 100% mode"
            );
        }
        reduce(&mut app, Msg::SetChartNormalized(false));
        assert!(!app.chart_normalized, "bascule desactivee");
    }

    #[test]
    fn re_clicking_a_selected_point_unpins_it() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        reduce(&mut app, Msg::ChartPoint(2, 1));
        assert_eq!(app.chart_sel, Some((2, 1)), "the first click pins");
        assert!(app.chart_pin.is_some(), "the detail is pinned");
        // Clicking the same point again unpins it (both the selection and the detail are cleared).
        reduce(&mut app, Msg::ChartPoint(2, 1));
        assert_eq!(app.chart_sel, None, "a second click unpins");
        assert!(app.chart_pin.is_none(), "the detail is cleared");
        // Another point pins again as usual.
        reduce(&mut app, Msg::ChartPoint(0, 0));
        assert_eq!(app.chart_sel, Some((0, 0)), "another point pins again");
    }

    #[test]
    fn data_table_screen_sorts_and_paginates_without_touching_data() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Data));
        assert_eq!(current_route(&app), Route::Data);
        assert!(primitive_count(&app) > 0, "the data screen renders");
        // Sorting: the first click is ascending, a second is descending; sorting returns to page 1.
        reduce(&mut app, Msg::DataPage(2));
        assert_eq!(app.data_page, 2);
        reduce(&mut app, Msg::DataSort(2));
        assert_eq!(app.data_sort, Some((2, true)), "premier clic = croissant");
        assert_eq!(app.data_page, 1, "sorting returns to page 1");
        reduce(&mut app, Msg::DataSort(2));
        assert_eq!(
            app.data_sort,
            Some((2, false)),
            "a second click = descending"
        );
        // The page size: it changes and returns to page 1.
        reduce(&mut app, Msg::DataPage(2));
        reduce(&mut app, Msg::DataPageSize(10));
        assert_eq!(app.data_page_size, 10);
        assert_eq!(app.data_page, 1, "changing the size returns to page 1");
        assert!(
            primitive_count(&app) > 0,
            "it renders after sorting/pagination"
        );
        // Row selection: a click selects the **source** row, a second click deselects it.
        assert_eq!(app.data_selected, None, "no row at the start");
        reduce(&mut app, Msg::DataSelectRow(3));
        assert_eq!(
            app.data_selected,
            Some(3),
            "a click = the source row is selected"
        );
        reduce(&mut app, Msg::DataSelectRow(7));
        assert_eq!(
            app.data_selected,
            Some(7),
            "clic ailleurs = deplace la selection"
        );
        reduce(&mut app, Msg::DataSelectRow(7));
        assert_eq!(app.data_selected, None, "a second click = deselection");
        assert!(primitive_count(&app) > 0, "it renders with the row detail");
        // The Level column's custom sort (index 3): a semantic order, not an alphabetical one.
        assert_eq!(level_rank("Low"), 0);
        assert!(
            level_rank("Low") < level_rank("Medium") && level_rank("Medium") < level_rank("High")
        );
        reduce(&mut app, Msg::DataSort(3));
        assert_eq!(app.data_sort, Some((3, true)), "the Level column is sorted");
        assert!(primitive_count(&app) > 0, "it renders sorted by priority");
        // Multi-selection (the boxes): toggling one row, then "check all"/"uncheck all".
        assert!(
            app.data_checked.is_empty(),
            "nothing is checked at the start"
        );
        reduce(&mut app, Msg::DataCheck(2));
        assert_eq!(app.data_checked, vec![2], "it checks source row 2");
        reduce(&mut app, Msg::DataCheck(2));
        assert!(app.data_checked.is_empty(), "checking again = unchecked");
        reduce(&mut app, Msg::DataCheckAll);
        assert_eq!(
            app.data_checked.len(),
            DATA_PEOPLE.len(),
            "check all = 12 rows"
        );
        reduce(&mut app, Msg::DataCheckAll);
        assert!(
            app.data_checked.is_empty(),
            "check-all again = uncheck everything"
        );
        assert!(primitive_count(&app) > 0, "it renders with checkboxes");
        // Search: typing updates the filter and returns to page 1.
        reduce(&mut app, Msg::DataPage(2));
        reduce(&mut app, Msg::DataSearch("ada".to_string()));
        assert_eq!(app.data_query, "ada", "the filter is updated");
        assert_eq!(app.data_page, 1, "a new filter returns to page 1");
        assert!(primitive_count(&app) > 0, "it renders filtered");
        // Bulk actions (milestone 243): Clear empties the selection, Delete removes the checked rows.
        reduce(&mut app, Msg::DataCheck(0));
        reduce(&mut app, Msg::DataCheck(1));
        reduce(&mut app, Msg::DataSelectRow(1));
        assert_eq!(app.data_checked.len(), 2);
        reduce(&mut app, Msg::DataClearChecked);
        assert!(app.data_checked.is_empty(), "Clear empties the selection");
        assert_eq!(app.data_selected, Some(1), "Clear leaves the focus alone");
        let before = app.data_rows().len();
        reduce(&mut app, Msg::DataCheck(0));
        // The confirmation (milestone 245): Delete opens the modal; Cancel closes it without deleting.
        reduce(&mut app, Msg::DataAskDelete);
        assert!(app.data_confirm_delete, "Delete ouvre la confirmation");
        assert_eq!(
            app.data_rows().len(),
            before,
            "nothing is deleted until it is confirmed"
        );
        assert!(primitive_count(&app) > 0, "it renders with the modal");
        reduce(&mut app, Msg::DataCancelDelete);
        assert!(!app.data_confirm_delete, "Cancel closes the modal");
        assert_eq!(app.data_rows().len(), before, "Cancel ne supprime rien");
        // Confirming really does delete, and closes the modal.
        reduce(&mut app, Msg::DataAskDelete);
        reduce(&mut app, Msg::DataDeleteChecked);
        assert!(!app.data_confirm_delete, "confirmer ferme la modale");
        assert_eq!(
            app.data_rows().len(),
            before - 1,
            "a confirmed Delete removes the checked row"
        );
        assert!(app.data_checked.is_empty(), "Delete empties the selection");
        assert_eq!(app.data_selected, None, "Delete resets the focus");
        assert!(primitive_count(&app) > 0, "it renders after the deletion");
        // The empty state (milestone 244): a filter with no result still renders (header + message).
        reduce(&mut app, Msg::DataSearch("zzzzz".to_string()));
        assert!(primitive_count(&app) > 0, "it renders with the empty state");
    }

    #[test]
    fn calendar_weekdays_only_filters_weekends() {
        // Juillet 2026 : 4-5 = samedi/dimanche ; 6 = lundi.
        assert!(
            is_weekend(2026, 7, 4) && is_weekend(2026, 7, 5),
            "4-5 juillet = week-end"
        );
        assert!(!is_weekend(2026, 7, 6), "6 juillet = lundi (ouvre)");
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Settings));
        assert!(!app.weekdays_only, "every day by default");
        assert!(primitive_count(&app) > 0, "the showcase renders");
        // The toggle: the filtered calendar renders, then it comes back.
        reduce(&mut app, Msg::SetWeekdaysOnly(true));
        assert!(app.weekdays_only);
        assert!(primitive_count(&app) > 0, "the filtered calendar renders");
        reduce(&mut app, Msg::SetWeekdaysOnly(false));
        assert!(!app.weekdays_only);
    }

    #[test]
    fn companion_chart_renders_across_families_with_hidden() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Charts));
        // A hidden series is shared by the main chart and the companion (the same `chart_hidden`).
        reduce(&mut app, Msg::ChartToggleSeries(1));
        assert_eq!(app.chart_hidden, vec![1]);
        // Lines (companion = bars) then bars (companion = lines): the screen renders both ways.
        reduce(&mut app, Msg::SetChartKind(0));
        assert!(primitive_count(&app) > 0, "main lines + companion bars");
        reduce(&mut app, Msg::SetChartKind(2));
        assert!(primitive_count(&app) > 0, "main bars + companion lines");
    }

    #[test]
    fn grid_next_error_cycles_through_all_faults() {
        let mut app = TodoApp::default();
        // Fautes attendues, en ordre : (0,0) Name vide, (0,2) email invalide, (1,2) email invalide.
        app.grid = vec![
            vec!["".to_string(), "PM".to_string(), "nope".to_string()],
            vec!["Ada".to_string(), "Engineer".to_string(), "bad".to_string()],
        ];
        assert_eq!(grid_faults(&app.grid), vec![(0, 0), (0, 2), (1, 2)]);
        // Each call moves on; the last one wraps back to the first.
        for expected in [(0, 0), (0, 2), (1, 2), (0, 0)] {
            reduce(&mut app, Msg::GridFocusError);
            assert_eq!(app.grid_error_cursor, Some(expected));
        }
    }

    #[test]
    fn snackbar_queue_orders_and_exits() {
        let mut app = TodoApp::default();
        // Two queued notifications: the 1st is visible, the 2nd waits.
        show_toast(&mut app, "A");
        show_toast(&mut app, "B");
        assert_eq!(app.snackbars.current().map(String::as_str), Some("A"));
        assert!(!app.snackbars.is_leaving(), "shown, not exiting yet");
        // Expiry → the head moves into its **exit** (a fade) without disappearing.
        reduce(&mut app, Msg::ToastExpire);
        assert!(app.snackbars.is_leaving());
        assert_eq!(app.snackbars.current().map(String::as_str), Some("A"));
        // Removal → the next one takes over (fading in).
        reduce(&mut app, Msg::DismissToast);
        assert_eq!(app.snackbars.current().map(String::as_str), Some("B"));
        assert!(!app.snackbars.is_leaving());
        // The last one: exit then removal → an empty queue.
        reduce(&mut app, Msg::ToastExpire);
        reduce(&mut app, Msg::DismissToast);
        assert!(app.snackbars.is_empty());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let path = std::env::temp_dir().join("frus-todos-test-roundtrip.txt");
        let items = vec![
            (false, "acheter du pain".to_string()),
            (true, "ranger le bureau".to_string()),
        ];
        save_todos(&path, &items).unwrap();
        assert_eq!(load_todos(&path), items);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loaded_replaces_todos_with_unique_ids() {
        let mut app = TodoApp::default();
        add(&mut app, "old one");
        reduce(
            &mut app,
            Msg::Loaded(vec![(true, "a".to_string()), (false, "b".to_string())]),
        );
        assert_eq!(app.todos.len(), 2);
        assert_eq!(app.todos[0].text, "a");
        assert!(app.todos[0].done);
        assert!(!app.todos[1].done);
        assert_ne!(app.todos[0].id, app.todos[1].id);
    }

    #[test]
    fn timer_subscription_gated_by_running() {
        let mut app = TodoApp::default();
        // By default the stopwatch is not running (`init` starts it at run time).
        assert!(app.subscription().is_empty());

        app.running = true;
        let subs = app.subscription();
        assert!(!subs.is_empty());
        // Two evaluations give the same id (a stable subscription).
        assert_eq!(subs.ids(), app.subscription().ids());
    }

    #[test]
    fn tick_increments_elapsed() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Tick);
        reduce(&mut app, Msg::Tick);
        assert_eq!(app.elapsed, 2);
    }

    #[test]
    fn save_produces_a_run_effect() {
        let mut app = TodoApp::default();
        add(&mut app, "x");
        // Save returns a non-empty command (the write is an effect).
        assert!(!reduce(&mut app, Msg::Save).is_empty());
        // A plain mutation has no effect at all.
        assert!(reduce(&mut app, Msg::DraftChanged("y".to_string())).is_empty());
    }

    #[test]
    fn back_gesture_flick_commits_pop() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::Push(Route::Settings));
        while app.nav_from.is_some() {
            app.tick(0.05);
        }
        assert_eq!(app.routes.len(), 1);

        // A small drag but a fast flick → it must commit the back.
        app.back_gesture(0.2);
        app.back_gesture_end(5.0);
        for _ in 0..200 {
            if app.back.is_none() {
                break;
            }
            app.tick(0.05);
        }
        assert!(app.routes.is_empty(), "the flick popped the screen");
    }

    /// Live-reload: the `save_state` snapshot rehydrates a fresh binary — the tasks, the draft,
    /// the filter, the theme (mode + seed) and the stacked screen.
    #[test]
    fn live_reload_state_round_trips() {
        let mut app = TodoApp::default();
        reduce(&mut app, Msg::DraftChanged("Buy milk".to_string()));
        reduce(&mut app, Msg::AddTodo);
        let id = app.todos[0].id;
        reduce(&mut app, Msg::ToggleTodo(id));
        reduce(&mut app, Msg::DraftChanged("half-typed".to_string()));
        reduce(&mut app, Msg::SetFilter(Filter::Done));
        reduce(&mut app, Msg::ToggleTheme);
        reduce(&mut app, Msg::CycleSeed);
        reduce(&mut app, Msg::Push(Route::Settings));

        let snapshot = Application::save_state(&app).expect("a snapshot");

        let mut fresh = TodoApp::default();
        Application::restore_state(&mut fresh, &snapshot);
        assert_eq!(fresh.todos.len(), 1);
        assert_eq!(fresh.todos[0].text, "Buy milk");
        assert!(fresh.todos[0].done);
        assert_eq!(fresh.draft, "half-typed");
        assert!(fresh.filter == Filter::Done);
        assert_eq!(fresh.light, app.light);
        assert_eq!(fresh.seed_index, 1);
        assert_eq!(current_route(&fresh), Route::Settings);
        // `init` after a rehydration does NOT restart the disk load (the snapshot is the
        // authority): no effect is emitted.
        assert!(fresh.init().is_empty(), "no Loaded after a rehydration");
        // A corrupt snapshot / one from another version is ignored without panicking.
        let mut other = TodoApp::default();
        Application::restore_state(&mut other, b"garbage \xFF");
        Application::restore_state(&mut other, b"frus-demo-state v999\nlight 1\n");
        assert!(other.todos.is_empty() && !other.restored);
    }
}
