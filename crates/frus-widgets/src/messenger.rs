//! The application's snack bars: [`show_snack_bar`] from any handler, and frus keeps the
//! queue, the timer and the place on screen (milestone 716).
//!
//! A [`SnackBar`] used to be the application's to queue and to time: a state holding a
//! [`SnackBarQueue`](crate::SnackBarQueue), a timer to start it leaving and another to
//! dismiss it, and a [`ScaffoldMessenger`](crate::ScaffoldMessenger) layer in a stack over
//! the page — sixty lines to say "Saved". The reference has
//! `ScaffoldMessenger.of(context).showSnackBar(..)`. This is that call: one bar at a time,
//! the next one shown when the current one goes, each for its duration, an action that also
//! hides the bar. The application's root (`FrusApp`) lays [`current_snack_bar`] over every page, so that a bar shown before a navigation is
//! still there after it.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::time::Duration;

use crate::callback::Callback;
use crate::component::request_rebuild;
use crate::widget::Widget;
use crate::SnackBar;

/// How long a bar stays when nothing says otherwise: the reference's four seconds.
pub const SNACK_BAR_DURATION: Duration = Duration::from_secs(4);

struct Shown {
    id: u64,
    bar: SnackBar,
    duration: Duration,
}

#[derive(Default)]
struct Messenger {
    queue: VecDeque<Shown>,
    next_id: u64,
}

/// Turns the bar on screen, if any, into a widget.
type MakeBar = fn() -> Option<Box<dyn Widget>>;

thread_local! {
    static MESSENGER: RefCell<Messenger> = RefCell::new(Messenger::default());
    /// What turns the bar on screen into a widget, once a bar has been shown.
    static MAKE: Cell<Option<MakeBar>> = const { Cell::new(None) };
}

/// Shows `bar` for four seconds, after the bars already waiting.
///
/// ```no_run
/// use frus_widgets::{host, Button, SnackBar};
///
/// let save: Button = Button::new("Save").on_press(|| {
///     // … save …
///     host::show_snack_bar(SnackBar::new("Saved").action("Undo", (|| {}).into()));
/// });
/// ```
///
/// Pressing the bar's action runs it and hides the bar. [`hide_current_snack_bar`] hides
/// it from code, and a bar's close icon (`SnackBar::close_icon(host::hide_current_snack_bar)`)
/// from the bar itself.
#[inline(never)]
pub fn show_snack_bar(bar: SnackBar) {
    show_snack_bar_for(bar, SNACK_BAR_DURATION);
}

/// Shows `bar` for `duration`, after the bars already waiting.
#[inline(never)]
pub fn show_snack_bar_for(bar: SnackBar, duration: Duration) {
    install_maker();
    let bar = bar.map_action(|action| {
        Callback::new(move || {
            action.call();
            hide_current_snack_bar();
        })
    });
    let first = MESSENGER.with(|messenger| {
        let mut messenger = messenger.borrow_mut();
        let id = messenger.next_id;
        messenger.next_id += 1;
        messenger.queue.push_back(Shown { id, bar, duration });
        (messenger.queue.len() == 1).then_some((id, duration))
    });
    if let Some((id, duration)) = first {
        expire(id, duration);
    }
    request_rebuild();
}

/// Hides the bar on screen, and shows the next one waiting.
#[inline(never)]
pub fn hide_current_snack_bar() {
    let next = MESSENGER.with(|messenger| {
        let mut messenger = messenger.borrow_mut();
        messenger.queue.pop_front()?;
        messenger
            .queue
            .front()
            .map(|shown| (shown.id, shown.duration))
    });
    if let Some((id, duration)) = next {
        expire(id, duration);
    }
    request_rebuild();
}

/// Hides the bar on screen and forgets the ones waiting.
#[inline(never)]
pub fn clear_snack_bars() {
    MESSENGER.with(|messenger| messenger.borrow_mut().queue.clear());
    request_rebuild();
}

/// The bar on screen as a widget: what the application's root lays over its pages.
///
/// It goes through a function that [`show_snack_bar`] installs, so that an application that
/// never shows a bar does not carry the code that draws one: every `FrusApp` asks this,
/// and a counter grew by 400 KB when it asked [`current_snack_bar`] instead.
#[inline(never)]
pub fn current_snack_bar_widget() -> Option<Box<dyn Widget>> {
    let make = MAKE.with(Cell::get)?;
    make()
}

#[inline(never)]
fn install_maker() {
    MAKE.with(|make| make.set(Some(make_current)));
}

fn make_current() -> Option<Box<dyn Widget>> {
    current_snack_bar().map(|bar| Box::new(bar) as Box<dyn Widget>)
}

/// The bar on screen, if there is one.
#[inline(never)]
pub fn current_snack_bar() -> Option<SnackBar> {
    MESSENGER.with(|messenger| {
        messenger
            .borrow()
            .queue
            .front()
            .map(|shown| shown.bar.clone())
    })
}

/// The bar shown as `id` goes once `duration` has passed — unless it has gone already, and
/// another is on screen in its place.
#[inline(never)]
fn expire(id: u64, duration: Duration) {
    crate::host::after(duration, move || {
        let current = MESSENGER.with(|messenger| messenger.borrow().queue.front().map(|s| s.id));
        if current == Some(id) {
            hide_current_snack_bar();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{take_effects, Effect};

    fn text(bar: &SnackBar) -> String {
        bar.text_for_tests().to_string()
    }

    #[test]
    fn bars_are_shown_one_at_a_time_in_order() {
        clear_snack_bars();
        let _ = take_effects();
        show_snack_bar(SnackBar::new("first"));
        show_snack_bar(SnackBar::new("second"));
        assert_eq!(current_snack_bar().map(|b| text(&b)), Some("first".into()));
        // Only the bar on screen is timed.
        assert_eq!(take_effects().len(), 1);
        hide_current_snack_bar();
        assert_eq!(current_snack_bar().map(|b| text(&b)), Some("second".into()));
        // The next one gets its own time.
        assert_eq!(take_effects().len(), 1);
        hide_current_snack_bar();
        assert!(current_snack_bar().is_none());
    }

    #[test]
    fn a_bar_goes_when_its_time_is_up_but_not_one_shown_after_it() {
        clear_snack_bars();
        let _ = take_effects();
        show_snack_bar_for(SnackBar::new("saved"), Duration::from_secs(2));
        let Some(Effect::After(delay, expire_first)) = take_effects().pop() else {
            panic!("a bar asks for its timer");
        };
        assert_eq!(delay, Duration::from_secs(2));
        // Hidden early, and another shown: the first timer must leave the second alone.
        hide_current_snack_bar();
        show_snack_bar(SnackBar::new("sent"));
        expire_first.call();
        assert_eq!(current_snack_bar().map(|b| text(&b)), Some("sent".into()));
        clear_snack_bars();
    }

    #[test]
    fn pressing_the_action_runs_it_and_hides_the_bar() {
        clear_snack_bars();
        let undone = std::rc::Rc::new(std::cell::Cell::new(false));
        let undo = undone.clone();
        show_snack_bar(
            SnackBar::new("deleted").action("Undo", Callback::new(move || undo.set(true))),
        );
        let action = current_snack_bar()
            .and_then(|bar| bar.action_for_tests())
            .expect("the bar has its action");
        action.call();
        assert!(undone.get());
        assert!(current_snack_bar().is_none());
        let _ = take_effects();
    }
}
