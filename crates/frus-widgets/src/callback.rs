//! [`Callback`]: what a handler *is* once a widget holds it.
//!
//! Every interactive widget answers a tap, a change, a submit with a value of its
//! message type. In an application built from components that value is a `Callback`:
//! a piece of work, run at the moment the interaction happens. Handing one to a widget
//! is handing it what to do, which is the shape every handler in this crate is written
//! against — a button's `on_press`, a field's `on_input`.
//!
//! ```
//! use frus_widgets::Callback;
//! use std::{cell::Cell, rc::Rc};
//!
//! let taps = Rc::new(Cell::new(0));
//! let count = taps.clone();
//! let on_tap = Callback::new(move || count.set(count.get() + 1));
//! on_tap.call();
//! on_tap.clone().call();
//! assert_eq!(taps.get(), 2);
//! ```
//!
//! A closure converts by itself wherever a `Callback` is asked for, so a handler is
//! usually just `|| …`.
//!
//! ## Why it is a handle
//!
//! What a handler captures — a state handle, a controller — belongs to the interface's
//! thread, and is not `Send`. A message has to be: the shell moves messages between
//! threads. So a `Callback` is a small `Send` handle to a closure that stays where it was
//! made, and running it anywhere else finds nothing to run. It cannot reach the closure
//! from another thread, which is what makes it safe to hand around.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};

thread_local! {
    /// The closures made on this thread, by the id their callbacks carry.
    static WORK: RefCell<HashMap<u64, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
}

static NEXT: AtomicU64 = AtomicU64::new(1);

/// Ids whose last callback was dropped on a thread that is not the one that owns the
/// closure. The owner lets them go the next time it makes or runs one.
static ABANDONED: Mutex<Vec<(ThreadId, u64)>> = Mutex::new(Vec::new());
static ANY_ABANDONED: AtomicBool = AtomicBool::new(false);

/// The lifetime of one closure: when the last callback to it goes, so does it.
struct Slot {
    id: u64,
    owner: ThreadId,
}

impl Drop for Slot {
    fn drop(&mut self) {
        if thread::current().id() == self.owner {
            // Dropped after the table is released: what a closure captured may be
            // callbacks, whose own drop wants the table.
            let removed = WORK.try_with(|work| work.borrow_mut().remove(&self.id));
            drop(removed);
        } else if let Ok(mut abandoned) = ABANDONED.lock() {
            abandoned.push((self.owner, self.id));
            ANY_ABANDONED.store(true, Ordering::Release);
        }
    }
}

/// Lets go of the closures other threads dropped the last callback to.
fn collect_abandoned() {
    if !ANY_ABANDONED.load(Ordering::Acquire) {
        return;
    }
    let me = thread::current().id();
    let mine: Vec<u64> = match ABANDONED.lock() {
        Ok(mut abandoned) => {
            let (mine, others): (Vec<_>, Vec<_>) =
                abandoned.drain(..).partition(|(owner, _)| *owner == me);
            ANY_ABANDONED.store(!others.is_empty(), Ordering::Release);
            *abandoned = others;
            mine.into_iter().map(|(_, id)| id).collect()
        }
        Err(_) => return,
    };
    let removed: Vec<_> = WORK.with(|work| {
        let mut work = work.borrow_mut();
        mine.iter().filter_map(|id| work.remove(id)).collect()
    });
    drop(removed);
}

/// A piece of work that an interaction sets off.
///
/// Cheap to clone: every clone runs the same closure. `Send` and `Sync` — it is a handle —
/// but it only *runs* on the thread that made it: the closure stays there.
#[derive(Clone)]
pub struct Callback {
    slot: Arc<Slot>,
}

impl Callback {
    /// Wraps `work` so that it can be handed to a widget and run later, any number of
    /// times.
    pub fn new(work: impl Fn() + 'static) -> Self {
        collect_abandoned();
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        WORK.with(|table| table.borrow_mut().insert(id, Rc::new(work)));
        Self {
            slot: Arc::new(Slot {
                id,
                owner: thread::current().id(),
            }),
        }
    }

    /// Runs the work.
    ///
    /// Called from a thread other than the one that made this callback, it finds nothing
    /// to run — and, in a debug build, says so, since that is never what was meant.
    pub fn call(&self) {
        collect_abandoned();
        let work = WORK.with(|table| table.borrow().get(&self.slot.id).cloned());
        match work {
            Some(work) => work(),
            None => debug_assert!(
                thread::current().id() == self.slot.owner,
                "a callback was run on a different thread from the one that made it"
            ),
        }
    }

    /// Whether both are the same closure — a clone of one another — rather than two
    /// that merely look alike.
    pub fn ptr_eq(&self, other: &Callback) -> bool {
        self.slot.id == other.slot.id
    }
}

impl<F: Fn() + 'static> From<F> for Callback {
    fn from(work: F) -> Self {
        Callback::new(work)
    }
}

impl PartialEq for Callback {
    /// Two callbacks are equal when they are the same closure: a closure has no other
    /// identity to compare.
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl fmt::Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callback(..)")
    }
}

/// What a handler's result can be turned into, so a widget's setter takes both shapes: a
/// closure that returns the widget's message type, and — in an application built from
/// components — one that just *does* the work and returns nothing.
///
/// ```
/// # use frus_widgets::{Callback, Checkbox};
/// # use std::{cell::Cell, rc::Rc};
/// let checked = Rc::new(Cell::new(false));
/// let flag = checked.clone();
/// // the closure does the work; there is no message to return
/// let _box: Checkbox = Checkbox::new(false).on_toggle(move |on| flag.set(on));
/// # let _ = Callback::new(|| {});
/// ```
///
/// A closure that returns `()` becomes a message that runs it: the work waits for the message
/// to be *delivered*, not for it to be made, because a widget may ask for its message more
/// than once for one interaction. Anything else is its own message, made at once.
pub trait IntoMsg<Msg>: Sized {
    /// The message that stands for `work`'s result.
    fn defer(work: impl Fn() -> Self + 'static) -> Msg;
}

impl<Msg> IntoMsg<Msg> for Msg {
    fn defer(work: impl Fn() -> Msg + 'static) -> Msg {
        work()
    }
}

impl IntoMsg<Callback> for () {
    fn defer(work: impl Fn() + 'static) -> Callback {
        Callback::new(work)
    }
}

/// A handler of one argument, as the message-making closure a widget stores.
pub(crate) fn handler1<A, R, Msg>(f: impl Fn(A) -> R + 'static) -> impl Fn(A) -> Msg + 'static
where
    A: Clone + 'static,
    R: IntoMsg<Msg>,
{
    let f = Rc::new(f);
    move |a| {
        let f = f.clone();
        R::defer(move || f(a.clone()))
    }
}

/// A handler of two arguments.
pub(crate) fn handler2<A, B, R, Msg>(
    f: impl Fn(A, B) -> R + 'static,
) -> impl Fn(A, B) -> Msg + 'static
where
    A: Clone + 'static,
    B: Clone + 'static,
    R: IntoMsg<Msg>,
{
    let f = Rc::new(f);
    move |a, b| {
        let f = f.clone();
        R::defer(move || f(a.clone(), b.clone()))
    }
}

/// A handler of four arguments.
pub(crate) fn handler4<A, B, C, D, R, Msg>(
    f: impl Fn(A, B, C, D) -> R + 'static,
) -> impl Fn(A, B, C, D) -> Msg + 'static
where
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    D: Clone + 'static,
    R: IntoMsg<Msg>,
{
    let f = Rc::new(f);
    move |a, b, c, d| {
        let f = f.clone();
        R::defer(move || f(a.clone(), b.clone(), c.clone(), d.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_callback_runs_its_work_every_time_and_through_every_clone() {
        let n = Rc::new(Cell::new(0));
        let inner = n.clone();
        let cb = Callback::new(move || inner.set(inner.get() + 1));
        cb.call();
        cb.clone().call();
        assert_eq!(n.get(), 2);
    }

    #[test]
    fn a_closure_converts_where_a_callback_is_asked_for() {
        fn takes(cb: impl Into<Callback>) -> Callback {
            cb.into()
        }
        let hit = Rc::new(Cell::new(false));
        let flag = hit.clone();
        takes(move || flag.set(true)).call();
        assert!(hit.get());
        // and a callback converts into itself, as anything does
        let cb = Callback::new(|| {});
        assert!(takes(cb.clone()).ptr_eq(&cb));
    }

    #[test]
    fn equality_is_identity_not_lookalike() {
        let a = Callback::new(|| {});
        assert_eq!(a, a.clone());
        assert_ne!(a, Callback::new(|| {}));
    }

    #[test]
    fn a_callback_is_a_message_that_may_cross_threads() {
        fn message<T: Send + Sync + Clone + 'static>() {}
        message::<Callback>();
    }

    #[test]
    fn the_closure_lives_until_the_last_callback_to_it_goes() {
        struct Flag(Rc<Cell<bool>>);
        impl Drop for Flag {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Rc::new(Cell::new(false));
        let held = Flag(dropped.clone());
        let cb = Callback::new(move || {
            let _ = &held;
        });
        let clone = cb.clone();
        drop(cb);
        assert!(!dropped.get(), "a clone still holds it");
        clone.call();
        drop(clone);
        assert!(dropped.get(), "and with the last one, it is let go");
    }

    #[test]
    fn a_callback_dropped_on_another_thread_is_let_go_by_its_owner() {
        struct Flag(Rc<Cell<bool>>);
        impl Drop for Flag {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Rc::new(Cell::new(false));
        let held = Flag(dropped.clone());
        let cb = Callback::new(move || {
            let _ = &held;
        });
        thread::spawn(move || drop(cb)).join().unwrap();
        assert!(!dropped.get(), "the closure cannot be dropped from there");
        // the next time the owner makes one, it lets go
        let _ = Callback::new(|| {});
        assert!(dropped.get());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "different thread")]
    fn running_a_callback_elsewhere_is_reported_in_a_debug_build() {
        let cb = Callback::new(|| {});
        thread::spawn(move || cb.call())
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e));
    }

    #[test]
    fn a_callback_that_owns_callbacks_can_be_dropped() {
        let inner = Callback::new(|| {});
        let outer = Callback::new(move || inner.call());
        outer.call();
        drop(outer);
    }
}
