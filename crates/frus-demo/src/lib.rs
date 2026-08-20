//! Sample application: a **to-do list** written with frus, as an **external consumer** of the
//! framework (it implements [`frus_shell::Application`]).
//!
//! Two entry points for the same code:
//! - desktop: `cargo run -p frus-demo` → the `src/bin/frus-demo.rs` binary → `run()`;
//! - Android: the `cdylib` library exposes `android_main`, called by the native activity.

mod assets;
mod l10n;
mod message;
mod model;
mod parts;
mod prelude;
mod screens;
/// The tool that renders the README's pictures; see the module's own documentation.
#[cfg(feature = "shots")]
pub mod shots;
mod storage;
#[cfg(test)]
mod tests;
mod theme;
mod update;

use crate::prelude::*;
use crate::screens::build_view;

// A **single** entry point: one declaration generates both the desktop entry (`run()`,
// called by the binary) and the Android one (`android_main`). See `frus_shell::main!`.
frus_shell::main!(TodoApp::default());

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
            Route::GridView => 4,
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
                        "4" => self.routes.push(Route::GridView),
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
