//! `reduce` — the **one** place state changes, whatever screen sent the message,
//! plus the motion constants the navigation and the back gesture share.

use crate::prelude::*;
use crate::screens::wizard_form;

// The motion constants, shared between the back gesture and the navigation.

/// Horizon (s) over which the velocity is projected to decide back / cancel.
pub(crate) const BACK_PROJECT: f32 = 0.12;
/// Projected position (a fraction) beyond which the back is committed.
pub(crate) const BACK_COMMIT_POS: f32 = 0.5;
/// Stiffness of the transition spring (fraction·s⁻²).
pub(crate) const NAV_SPRING_K: f32 = 220.0;
/// Damping (~critical) → a gentle arrival with no overshoot.
pub(crate) const NAV_SPRING_C: f32 = 30.0;

/// The spring shared by the navigation and the back gesture, expressed in `frus-core`'s
/// animation layer (`trait Simulation`).
pub(crate) fn nav_spring() -> SpringDescription {
    SpringDescription::new(1.0, NAV_SPRING_K, NAV_SPRING_C)
}

/// Starts a screen transition: the spring drives the progress `0 → 1`.
pub(crate) fn start_nav(app: &mut TodoApp, forward: bool) {
    app.nav_forward = forward;
    app.nav.set_value(0.0);
    app.nav.spring_to(1.0, nav_spring(), 0.0);
}

/// A timed effect: it starts the head notification's **exit** after it has been up for ~2 s.
pub(crate) fn toast_expire_after() -> Command<Msg> {
    Command::perform(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        Msg::ToastExpire
    })
}

/// Queues a notification (Snackbar); when it becomes the **head** of the queue, its exit is scheduled.
pub(crate) fn show_toast(app: &mut TodoApp, text: &str) -> Command<Msg> {
    let was_empty = app.snackbars.is_empty();
    app.snackbars.push(text.to_string(), 0.0);
    if was_empty {
        toast_expire_after()
    } else {
        Command::none()
    }
}

pub(crate) fn reduce(app: &mut TodoApp, message: Msg) -> Command<Msg> {
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
