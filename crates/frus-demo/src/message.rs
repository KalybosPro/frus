//! `Msg` — everything that can happen to this application.
//!
//! One file, because the list *is* the application's vocabulary: seeing it whole
//! is what makes it obvious when a variant is really two.

use crate::prelude::*;

/// Messages emitted by the interface.
#[derive(Clone)]
pub(crate) enum Msg {
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
    SnackBarExpire,
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
