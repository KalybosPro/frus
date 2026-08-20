//! Tests that cut across modules — a whole screen's behaviour, an interaction
//! from message to scene. Tests of a single module's own logic live next to it.

use crate::prelude::*;
use crate::screens::*;
use frus_widgets::{build_ui, Point, Runtime, Size};

/// An app whose editable grid is already filled — the shape half of these tests
/// start from.
fn app_with_grid(grid: Vec<Vec<String>>) -> TodoApp {
    TodoApp {
        grid,
        ..Default::default()
    }
}

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
    let mut app = TodoApp {
        stat_detail_open: true,
        ..Default::default()
    };
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

/// The device finding of milestone 327, closed in 334. A task label long enough to
/// overflow the row used to be laid out at its own content width, which pushed the delete
/// button off the card, out of the window, and — the part that mattered — out of the hit
/// registry: the × was not merely invisible, it was unclickable, and that task could not
/// be deleted at all.
///
/// Driven through `view` at a phone's width, and read from the **hit registry** rather
/// than from the picture, because the registry is what the report was about.
#[test]
fn a_long_task_label_still_leaves_its_delete_button_clickable() {
    // A phone in portrait, in logical pixels.
    const W: f32 = 411.0;
    const H: f32 = 869.0;

    let mut app = TodoApp::default();
    add(&mut app, "short");
    add(
        &mut app,
        "a task label far longer than any phone is wide, which is exactly the case that          used to push the delete button out of the window entirely",
    );
    let long_id = app.todos[1].id;

    let theme = Theme::default();
    let tree = build_view(&app, &theme, W, H);
    let ui = build_ui(&tree, Size::new(W, H), &Runtime::default(), &theme);

    // Sweep the window and ask what a tap there would send.
    let mut targets = Vec::new();
    let mut y = 1.0;
    while y < H {
        let mut x = 1.0;
        while x < W {
            if let Some(Msg::DeleteTodo(id)) =
                ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id))
            {
                if id == long_id {
                    targets.push((x, y));
                }
            }
            x += 2.0;
        }
        y += 2.0;
    }

    assert!(
        !targets.is_empty(),
        "no tap anywhere in the window deletes the long task"
    );
    // And it is a real target, not a sliver: an icon button's 40 px, near the right edge.
    let left = targets.iter().map(|t| t.0).fold(f32::MAX, f32::min);
    let right = targets.iter().map(|t| t.0).fold(f32::MIN, f32::max);
    assert!(
        right - left > 20.0,
        "the delete target is a sliver {left}..{right}, not a button"
    );
    assert!(right < W, "and it is inside the window");
    // And it is where a trailing button belongs: against the row's right edge, not
    // sitting on top of the label because the label was given no width at all.
    assert!(
        left > W * 0.5,
        "the delete target is at {left}..{right}, not on the right-hand side"
    );
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
    let mut app = app_with_grid(vec![
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
    ]);
    reduce(&mut app, Msg::Push(Route::GridView));
    assert_eq!(current_route(&app), Route::GridView);
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
    let mut app = app_with_grid(vec![
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
    ]);
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
    // One valid row, one with an empty Name and a malformed email (2 errors).
    let mut app = app_with_grid(vec![
        vec![
            "Ada".to_string(),
            "Engineer".to_string(),
            "a@x.com".to_string(),
        ],
        vec!["".to_string(), "PM".to_string(), "nope".to_string()],
    ]);
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
    // Row 0 is valid, row 1 has an empty Name (column 0) = the first fault expected.
    let mut app = app_with_grid(vec![
        vec![
            "Ada".to_string(),
            "Engineer".to_string(),
            "a@x.com".to_string(),
        ],
        vec!["".to_string(), "PM".to_string(), "nope".to_string()],
    ]);
    assert_eq!(grid_next_error(&app.grid, None), Some((1, 0)));
    assert!(
        !reduce(&mut app, Msg::GridFocusError).is_empty(),
        "it focuses the faulty cell"
    );
    // Everything valid: no target left, so no command.
    app.grid[1][0] = "Bob".to_string();
    app.grid[1][2] = "b@x.com".to_string();
    assert_eq!(grid_next_error(&app.grid, None), None);
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
    assert!(level_rank("Low") < level_rank("Medium") && level_rank("Medium") < level_rank("High"));
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
    // Fautes attendues, en ordre : (0,0) Name vide, (0,2) email invalide, (1,2) email invalide.
    let mut app = app_with_grid(vec![
        vec!["".to_string(), "PM".to_string(), "nope".to_string()],
        vec!["Ada".to_string(), "Engineer".to_string(), "bad".to_string()],
    ]);
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
    reduce(&mut app, Msg::SnackBarExpire);
    assert!(app.snackbars.is_leaving());
    assert_eq!(app.snackbars.current().map(String::as_str), Some("A"));
    // Removal → the next one takes over (fading in).
    reduce(&mut app, Msg::DismissToast);
    assert_eq!(app.snackbars.current().map(String::as_str), Some("B"));
    assert!(!app.snackbars.is_leaving());
    // The last one: exit then removal → an empty queue.
    reduce(&mut app, Msg::SnackBarExpire);
    reduce(&mut app, Msg::DismissToast);
    assert!(app.snackbars.is_empty());
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

/// Turning the phone must not move the navigation (milestone 305).
///
/// Reported from a device: rotating to landscape made the navigation leave the bottom
/// of the screen and reappear as a rail down the left edge. The cause was in
/// `Scaffold`, which measured its own width; the fix is there too. This test is here
/// because it is the demo's own screen, at the reporter's own logical size, that has to
/// come out right — a widget test can pass while the application still looks wrong.
#[test]
fn rotating_the_phone_leaves_the_navigation_at_the_bottom() {
    // The reporter's device: 1080 × 2340 at a density that gives 424 × 918 logical.
    let portrait = (424.0, 918.0);
    let landscape = (918.0, 424.0);

    /// The lowest `y` at which any of the navigation's labels is painted.
    fn nav_bottom(app: &TodoApp, width: f32, height: f32) -> f32 {
        let theme = Application::theme(app);
        let root = Application::view(app, &theme, width, height);
        let ui = build_ui(
            root.as_ref(),
            Size::new(width, height),
            &Runtime::default(),
            &theme,
        );
        let mut lowest = f32::MIN;
        for primitive in ui.scene().primitives() {
            if let frus_widgets::Primitive::Text { text, position, .. } = primitive {
                if text == "Tasks" || text == "Stats" || text == "About" {
                    lowest = lowest.max(position.y);
                }
            }
        }
        assert!(lowest > f32::MIN, "the destinations were never painted");
        lowest
    }

    let mut app = TodoApp::default();
    let _ = app.init();

    let low_portrait = nav_bottom(&app, portrait.0, portrait.1);
    assert!(
        low_portrait > portrait.1 * 0.7,
        "portrait: destinations at y = {low_portrait} of {}",
        portrait.1
    );

    // The rotation itself, through the shell's own entry point.
    Application::on_resize(&mut app, landscape.0, landscape.1);
    let low_landscape = nav_bottom(&app, landscape.0, landscape.1);
    assert!(
        low_landscape > landscape.1 * 0.7,
        "landscape: destinations at y = {low_landscape} of {} — the navigation moved",
        landscape.1
    );
}

/// **Choosing a navigating item closes the overflow menu (milestone 326)** — the
/// application half of a defect found on a device.
///
/// The framework half is that a departing screen's overlay was drawn over the screen that
/// replaced it, and it is guarded in `navigator.rs`. This is the other half of what was
/// seen: the menu also **came back** on returning home, because nothing ever closed it.
/// `Push` already dismisses the drawer and the popup menu; the app bar's overflow was the
/// one that was missed.
#[test]
fn navigating_from_the_overflow_menu_closes_it() {
    let mut app = TodoApp::default();
    let _ = app.init();

    reduce(&mut app, Msg::ToggleActions);
    assert!(app.actions_open, "the menu is open");

    // "Settings →" is one of the overflow's own actions.
    reduce(&mut app, Msg::Push(Route::Settings));
    assert!(
        !app.actions_open,
        "the menu stayed open, and would reappear on returning home"
    );

    // An action that does *not* navigate leaves it alone — dismissing on every press
    // would make the menu useless for the toggles that live in it.
    reduce(&mut app, Msg::ToggleActions);
    reduce(&mut app, Msg::ToggleTheme);
    assert!(app.actions_open, "a toggle does not dismiss the menu");
}

/// **Nothing in the application may draw outside its parent** — checked on every screen,
/// at a phone's width and at a desktop's, because that is where the difference shows.
///
/// This is the instrument milestone 335 exists for. Its first run found a real one: the
/// chart dashboard's segmented control was 584 px of segments in a 363 px row on a phone,
/// running 221 px past the card. Nothing had ever said so.
#[test]
fn no_screen_draws_outside_itself() {
    let theme = Theme::default();
    let routes = [
        Route::Home,
        Route::Settings,
        Route::Journal,
        Route::Wizard,
        Route::GridView,
        Route::Charts,
        Route::Data,
        Route::Board,
        Route::Tour,
    ];
    let mut worst: Vec<String> = Vec::new();
    for route in routes {
        let mut app = TodoApp::default();
        add(&mut app, "short");
        reduce(&mut app, Msg::Push(route));
        for (label, w, h) in [("phone", 411.0, 869.0), ("desktop", 1200.0, 800.0)] {
            let tree = build_view(&app, &theme, w, h);
            let ui = build_ui(&tree, Size::new(w, h), &Runtime::default(), &theme);
            let over = ui
                .overflows()
                .iter()
                .map(|o| o.amount)
                .fold(0.0_f32, f32::max);
            // Settings at a phone's width is a **known** 4.5 px, measured in milestone 335
            // and left on the roadmap: something in the Controls tab will not go below
            // about 380 px, so the row centring the tab set lets it hang out either side.
            // Milestone 345 disproved the cause recorded here — the tab set fills its box
            // now and the overflow did not move — and measured it unrounded, which is why
            // the pin came down from 5.5. Pinned so it cannot grow while it waits.
            let allowed = if matches!(route, Route::Settings) && label == "phone" {
                4.6
            } else {
                0.0
            };
            if over > allowed {
                worst.push(format!("{route:?}/{label} overflows by {over:.1} px"));
            }
        }
    }
    assert!(worst.is_empty(), "{worst:#?}");
}
