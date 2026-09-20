//! **What more than one screen needs**: the task list, the reader's preferences, and the
//! notifications.
//!
//! Everything else a screen keeps, it keeps itself, in its own `State`. This is the part that
//! could not live there — the list is read by the home screen, the task's own screen and the
//! settings' statistics; the theme dresses every screen — so it is held here, handed to the
//! screens that ask for it, and changed through methods that say what happened. Each of them
//! asks for a rebuild, which is how a screen finds out.
//!
//! It is plain Rust with no window and no widget in it, which is what makes the rules worth
//! testing directly.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use frus_widgets::host;
use frus_widgets::{request_rebuild, Locale, SnackBarQueue};

use crate::l10n::{lang_index, lang_is_rtl, next_lang, LANGS};
use crate::model::{visible, Filter, Todo};
use crate::storage::{load_todos, save_todos, todos_path};
use crate::theme::{theme_of, THEME_SEEDS};

/// How the demo is dressed, as the reader chose it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Prefs {
    /// A light theme (otherwise dark).
    pub(crate) light: bool,
    /// The theme seed: `0` = the hand-written scheme, otherwise `from_seed` (HCT).
    pub(crate) seed_index: usize,
    /// A right-to-left layout (Arabic/Hebrew)?
    pub(crate) rtl: bool,
    /// The language **the reader picked in this application** (an index into `LANGS`). `None` —
    /// the default — follows the device.
    pub(crate) lang: Option<usize>,
    /// An application-level zoom over the whole UI.
    pub(crate) density: f32,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            light: false,
            seed_index: 0,
            rtl: false,
            lang: None,
            density: 1.0,
        }
    }
}

/// The range the zoom is kept to.
const DENSITY: (f32, f32) = (0.8, 1.4);

/// How long a notification stays up, and how long its exit takes.
const TOAST_UP: Duration = Duration::from_secs(2);
const TOAST_EXIT: Duration = Duration::from_millis(300);

/// The state the demo's screens share. One per application, held in an `Rc`.
pub(crate) struct Demo {
    todos: RefCell<Vec<Todo>>,
    next_id: Cell<u64>,
    prefs: RefCell<Prefs>,
    /// Whether the language's direction was last applied as mirrored — so the host is only
    /// told again when it changes.
    mirrored: Cell<bool>,
    toasts: RefCell<SnackBarQueue<String>>,
    /// The state came from a live-reload snapshot: the tasks are not reloaded from disk (the
    /// snapshot is the authority).
    restored: Cell<bool>,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            todos: RefCell::default(),
            next_id: Cell::new(0),
            prefs: RefCell::default(),
            mirrored: Cell::new(false),
            toasts: RefCell::default(),
            restored: Cell::new(false),
        }
    }
}

impl Demo {
    /// A demo with nothing in it, dressed as the defaults say, and told to the host.
    #[cfg(test)]
    pub(crate) fn new() -> Rc<Self> {
        let demo = Rc::new(Self::default());
        demo.apply();
        demo
    }

    // --- The tasks ---

    /// A copy of the tasks, in the order they were added.
    pub(crate) fn todos(&self) -> Vec<Todo> {
        self.todos.borrow().clone()
    }

    /// The number of tasks.
    #[cfg(any(test, feature = "shots"))]
    pub(crate) fn len(&self) -> usize {
        self.todos.borrow().len()
    }

    /// One task, if it is still there.
    pub(crate) fn todo(&self, id: u64) -> Option<Todo> {
        self.todos.borrow().iter().find(|t| t.id == id).cloned()
    }

    /// Adds a task made of `text`, trimmed. Blank text adds nothing; whether it did is
    /// returned.
    pub(crate) fn add(&self, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.todos.borrow_mut().push(Todo {
            id,
            text: text.to_string(),
            done: false,
        });
        request_rebuild();
        true
    }

    /// Ticks or unticks a task.
    pub(crate) fn toggle(&self, id: u64) {
        if let Some(todo) = self.todos.borrow_mut().iter_mut().find(|t| t.id == id) {
            todo.done = !todo.done;
        }
        request_rebuild();
    }

    /// Sets whether a task is done.
    pub(crate) fn set_done(&self, id: u64, done: bool) {
        if let Some(todo) = self.todos.borrow_mut().iter_mut().find(|t| t.id == id) {
            todo.done = done;
        }
        request_rebuild();
    }

    /// Deletes a task.
    pub(crate) fn delete(&self, id: u64) {
        self.todos.borrow_mut().retain(|t| t.id != id);
        request_rebuild();
    }

    /// Deletes every task that is done.
    pub(crate) fn clear_done(&self) {
        self.todos.borrow_mut().retain(|t| !t.done);
        request_rebuild();
    }

    /// A task dragged into a new place in the list: `(from, to)`, counted in the rows **on
    /// screen** — which under a filter are not the rows in the model.
    ///
    /// They are turned back into identities before anything moves: the row that was carried,
    /// and the row it landed on. Moving by index in the model instead would put a task
    /// somewhere else entirely as soon as a filter was on — the failure that makes a
    /// reordering list feel haunted.
    pub(crate) fn move_todo(&self, filter: Filter, from: usize, to: usize) {
        let mut todos = self.todos.borrow_mut();
        let shown: Vec<u64> = visible(&todos, filter).iter().map(|t| t.id).collect();
        let (Some(&moved), Some(&landed)) = (shown.get(from), shown.get(to)) else {
            return;
        };
        let Some(source) = todos.iter().position(|t| t.id == moved) else {
            return;
        };
        let task = todos.remove(source);
        // The destination is found **after** the removal, in the list as it now is, for the
        // same reason the widget hands over an index that already counts the row as gone.
        let target = todos
            .iter()
            .position(|t| t.id == landed)
            .map(|at| if to > from { at + 1 } else { at })
            .unwrap_or(todos.len());
        let at = target.min(todos.len());
        todos.insert(at, task);
        request_rebuild();
    }

    /// Replaces the tasks with `items` — `(done, text)` — under fresh ids.
    pub(crate) fn replace_all(&self, items: Vec<(bool, String)>) {
        let mut todos = self.todos.borrow_mut();
        *todos = items
            .into_iter()
            .map(|(done, text)| {
                let id = self.next_id.get();
                self.next_id.set(id + 1);
                Todo { id, text, done }
            })
            .collect();
        request_rebuild();
    }

    // --- Saving and loading ---

    /// Writes the tasks to disk on another thread, and says so when it is done queued.
    pub(crate) fn save(self: &Rc<Self>) {
        let items: Vec<(bool, String)> = self
            .todos
            .borrow()
            .iter()
            .map(|t| (t.done, t.text.clone()))
            .collect();
        host::spawn(
            move || {
                let _ = save_todos(&todos_path(), &items);
            },
            |_| {},
        );
        self.toast("Saved");
    }

    /// Reads the tasks from disk on another thread and puts them in place of the list.
    pub(crate) fn load(self: &Rc<Self>) {
        let demo = self.clone();
        host::spawn(
            || load_todos(&todos_path()),
            move |items| demo.replace_all(items),
        );
    }

    /// Loads at start-up, unless a live-reload snapshot already holds the tasks.
    pub(crate) fn start(self: &Rc<Self>) {
        if !self.restored.get() {
            self.load();
        }
    }

    // --- Notifications ---

    /// Queues a notification; when it becomes the **head** of the queue, its exit is scheduled.
    pub(crate) fn toast(self: &Rc<Self>, text: &str) {
        let was_empty = self.toasts.borrow().is_empty();
        self.toasts.borrow_mut().push(text.to_string(), 0.0);
        request_rebuild();
        if was_empty {
            self.expire_later();
        }
    }

    /// The notification at the head of the queue, and whether it is on its way out.
    pub(crate) fn current_toast(&self) -> Option<(String, bool)> {
        let toasts = self.toasts.borrow();
        toasts
            .current()
            .map(|message| (message.clone(), toasts.is_leaving()))
    }

    fn expire_later(self: &Rc<Self>) {
        let demo = self.clone();
        host::after(TOAST_UP, move || demo.expire());
    }

    /// Starts the head notification's **exit** (a fade), then removes it.
    fn expire(self: &Rc<Self>) {
        self.toasts.borrow_mut().start_leaving();
        request_rebuild();
        let demo = self.clone();
        host::after(TOAST_EXIT, move || demo.dismiss());
    }

    /// Removes the head notification and moves on to the next queued one, if there is one.
    pub(crate) fn dismiss(self: &Rc<Self>) {
        self.toasts.borrow_mut().dismiss();
        request_rebuild();
        if !self.toasts.borrow().is_empty() {
            self.expire_later();
        }
    }

    // --- Preferences ---

    /// A copy of the preferences.
    pub(crate) fn prefs(&self) -> Prefs {
        self.prefs.borrow().clone()
    }

    /// The language in force: the reader's pick, or the device's.
    pub(crate) fn lang(&self) -> usize {
        lang_index(self.prefs.borrow().lang)
    }

    /// Changes the preferences with `change`, and tells the host what it now looks like.
    fn set_prefs(&self, change: impl FnOnce(&mut Prefs)) {
        change(&mut self.prefs.borrow_mut());
        self.apply();
    }

    /// Light or dark.
    pub(crate) fn toggle_theme(&self) {
        self.set_prefs(|p| p.light = !p.light);
    }

    /// Moves to the next theme seed (default → Blue → Purple → Orange).
    pub(crate) fn cycle_seed(&self) {
        self.set_prefs(|p| p.seed_index = (p.seed_index + 1) % (THEME_SEEDS.len() + 1));
    }

    /// Flips the layout direction (LTR ↔ RTL).
    pub(crate) fn toggle_rtl(&self) {
        self.set_prefs(|p| p.rtl = !p.rtl);
    }

    /// Moves to the next language.
    pub(crate) fn cycle_lang(&self) {
        self.set_prefs(|p| p.lang = next_lang(p.lang));
    }

    /// Sets the zoom, kept to its range.
    pub(crate) fn set_density(&self, density: f32) {
        self.set_prefs(|p| p.density = density.clamp(DENSITY.0, DENSITY.1));
    }

    /// Whether the interface is laid out right to left: the reader asked, or the language is.
    pub(crate) fn mirrored(&self) -> bool {
        self.prefs.borrow().rtl || lang_is_rtl(self.lang())
    }

    /// Tells the host how the demo is dressed: its two themes and which is on show, the
    /// zoom, the language and the words the framework says in it.
    ///
    /// The framework notices that the theme this application resolves to has changed and
    /// crosses to it; nothing here has to capture or start an animation.
    pub(crate) fn apply(&self) {
        let prefs = self.prefs.borrow().clone();
        let mirrored = self.mirrored();
        self.mirrored.set(mirrored);
        let app = host::app();
        app.set_theme(theme_of(prefs.seed_index, mirrored, false));
        app.set_dark_theme(Some(theme_of(prefs.seed_index, mirrored, true)));
        app.set_theme_mode(if prefs.light {
            frus_widgets::ThemeMode::Light
        } else {
            frus_widgets::ThemeMode::Dark
        });
        app.set_density(prefs.density);
        app.set_locale(prefs.lang.map(|index| Locale::new(LANGS[index].1)));
        // The words the framework says on this application's behalf — a calendar's months,
        // the label a screen reader announces on a back arrow. Arabic gets English here,
        // deliberately: there is no Arabic table in the framework yet, and a machine-
        // translated one would be worse than none. Its layout still mirrors.
        app.set_localizations(match LANGS[self.lang()].1 {
            "fr" => Some(Rc::new(frus_widgets::French)),
            _ => None,
        });
    }

    /// Called from a build: if the language the device resolved to turned the layout the other
    /// way since the host was told — a device set to Arabic, before the reader picked
    /// anything — tells the host again. Cheap when nothing changed, which is nearly always.
    pub(crate) fn sync_direction(&self) {
        if self.mirrored() != self.mirrored.get() {
            self.apply();
        }
    }

    // --- Live reload ---

    /// The essentials of the state, for a live reload: the tasks and how the demo is dressed.
    pub(crate) fn snapshot(&self) -> Vec<u8> {
        let prefs = self.prefs.borrow();
        let mut out = String::from("frus-demo-state v2\n");
        out.push_str(&format!("light {}\n", prefs.light as u8));
        out.push_str(&format!("seed {}\n", prefs.seed_index));
        for todo in self.todos.borrow().iter() {
            out.push_str(&format!("todo {}\t{}\n", todo.done as u8, todo.text));
        }
        out.into_bytes()
    }

    /// Rehydrates a [`snapshot`](Self::snapshot) — tolerantly: any unknown line (from another
    /// version of the code) is ignored.
    pub(crate) fn restore(&self, bytes: &[u8]) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        let mut lines = text.lines();
        if lines.next() != Some("frus-demo-state v2") {
            return;
        }
        for line in lines {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "light" => self.prefs.borrow_mut().light = value == "1",
                "seed" => self.prefs.borrow_mut().seed_index = value.parse().unwrap_or(0),
                "todo" => {
                    if let Some((done, text)) = value.split_once('\t') {
                        let id = self.next_id.get();
                        self.next_id.set(id + 1);
                        self.todos.borrow_mut().push(Todo {
                            id,
                            text: text.to_string(),
                            done: done == "1",
                        });
                    }
                }
                _ => {}
            }
        }
        self.restored.set(true);
        self.apply();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{active_count, done_count};

    fn texts(demo: &Demo) -> Vec<String> {
        demo.todos().into_iter().map(|t| t.text).collect()
    }

    #[test]
    fn adding_trims_and_refuses_blank_text() {
        let demo = Demo::new();
        assert!(!demo.add("   "));
        assert!(demo.add("  buy bread "));
        assert_eq!(texts(&demo), ["buy bread"]);
        assert_eq!(demo.todos()[0].id, 0);
        assert!(demo.add("tidy"));
        assert_eq!(demo.todos()[1].id, 1, "ids are never reused");
    }

    #[test]
    fn toggle_delete_and_clear_done() {
        let demo = Demo::new();
        for text in ["a", "b", "c"] {
            demo.add(text);
        }
        demo.toggle(0);
        demo.set_done(1, true);
        let counts = |d: &Demo| (active_count(&d.todos()), done_count(&d.todos()));
        assert_eq!(counts(&demo), (1, 2));
        demo.toggle(0);
        assert_eq!(counts(&demo).1, 1);
        demo.clear_done();
        assert_eq!(texts(&demo), ["a", "c"]);
        demo.delete(0);
        assert_eq!(texts(&demo), ["c"]);
        demo.delete(99);
        assert_eq!(demo.len(), 1, "deleting what is not there does nothing");
    }

    #[test]
    fn moving_a_task_reorders_what_the_list_is_showing() {
        let demo = Demo::new();
        for text in ["a", "b", "c", "d"] {
            demo.add(text);
        }
        // Under the filter that hides b and d, the rows on screen are a and c.
        demo.set_done(1, true);
        demo.set_done(3, true);
        demo.move_todo(Filter::Active, 0, 1);
        assert_eq!(
            texts(&demo),
            ["b", "c", "a", "d"],
            "a was carried past c, and the hidden b stays where it was"
        );
        demo.move_todo(Filter::All, 3, 0);
        assert_eq!(texts(&demo), ["d", "b", "c", "a"]);
        demo.move_todo(Filter::All, 0, 9);
        assert_eq!(texts(&demo), ["d", "b", "c", "a"], "off the end is refused");
    }

    #[test]
    fn a_load_replaces_the_tasks_under_fresh_ids() {
        let demo = Demo::new();
        demo.add("old");
        demo.replace_all(vec![(true, "x".into()), (false, "y".into())]);
        let todos = demo.todos();
        assert_eq!(texts(&demo), ["x", "y"]);
        assert!(todos[0].done && !todos[1].done);
        assert_eq!((todos[0].id, todos[1].id), (1, 2), "the ids are unique");
    }

    #[test]
    fn density_is_clamped() {
        let demo = Demo::new();
        demo.set_density(5.0);
        assert_eq!(demo.prefs().density, 1.4);
        demo.set_density(0.1);
        assert_eq!(demo.prefs().density, 0.8);
        assert_eq!(host::app().density(), 0.8, "and the host was told");
    }

    #[test]
    fn the_light_switch_is_what_the_host_shows() {
        let demo = Demo::new();
        assert_eq!(host::app().theme_mode(), frus_widgets::ThemeMode::Dark);
        demo.toggle_theme();
        assert_eq!(host::app().theme_mode(), frus_widgets::ThemeMode::Light);
        assert!(
            host::app().dark_theme().is_some(),
            "both themes are supplied"
        );
    }

    #[test]
    fn the_language_cycle_runs_through_all_three_and_back_to_the_device() {
        let demo = Demo::new();
        let picked = |d: &Demo| d.prefs().lang;
        assert_eq!(picked(&demo), None);
        demo.cycle_lang();
        assert_eq!(picked(&demo), Some(0));
        demo.cycle_lang();
        demo.cycle_lang();
        assert_eq!(picked(&demo), Some(2));
        assert!(demo.mirrored(), "Arabic mirrors the layout by itself");
        demo.cycle_lang();
        assert_eq!(picked(&demo), None);
        assert!(!demo.mirrored());
    }

    #[test]
    fn choosing_french_hands_the_framework_a_french_table() {
        let demo = Demo::new();
        assert!(host::app().localizations().is_none());
        demo.cycle_lang();
        assert!(host::app().localizations().is_none(), "English needs none");
        demo.cycle_lang();
        assert!(host::app().localizations().is_some(), "French does");
        assert_eq!(
            host::app().locale().map(|l| l.language_code().to_string()),
            Some("fr".into())
        );
    }

    #[test]
    fn a_notification_is_queued_and_expires_in_two_steps() {
        let demo = Demo::new();
        let _ = host::take_effects();
        demo.toast("one");
        demo.toast("two");
        assert_eq!(demo.current_toast(), Some(("one".into(), false)));
        assert_eq!(
            host::take_effects().len(),
            1,
            "only the head schedules its exit"
        );
        demo.expire();
        assert_eq!(demo.current_toast(), Some(("one".into(), true)), "leaving");
        demo.dismiss();
        assert_eq!(
            demo.current_toast(),
            Some(("two".into(), false)),
            "moved on"
        );
        assert!(
            !host::take_effects().is_empty(),
            "and the next one's exit is scheduled"
        );
        demo.expire();
        demo.dismiss();
        assert_eq!(demo.current_toast(), None);
    }

    #[test]
    fn live_reload_state_round_trips() {
        let demo = Demo::new();
        demo.add("buy bread");
        demo.add("tidy");
        demo.toggle(1);
        demo.toggle_theme();
        demo.cycle_seed();
        let bytes = demo.snapshot();

        let fresh = Demo::new();
        fresh.restore(&bytes);
        assert_eq!(texts(&fresh), ["buy bread", "tidy"]);
        assert!(fresh.todos()[1].done);
        assert!(fresh.prefs().light);
        assert_eq!(fresh.prefs().seed_index, 1);
        assert!(fresh.restored.get(), "the snapshot is the authority");

        let other = Demo::new();
        other.restore(b"something else entirely");
        assert_eq!(other.len(), 0, "an unknown snapshot is ignored");
        assert!(!other.restored.get());
    }
}
