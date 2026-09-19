//! **Listenables**: a value, or a controller, that tells whoever cares when it changed.
//!
//! [`ChangeNotifier`] is the plain form — something changed, and no more is said.
//! [`ValueNotifier`] is a notifier that holds one value. Anything that a widget is driven
//! by and something else may change — a text controller, a selection, a theme choice — is
//! one of these underneath.
//!
//! ```
//! use frus_widgets::{Listenable, ValueNotifier};
//! use std::{cell::Cell, rc::Rc};
//!
//! let volume = ValueNotifier::new(3);
//! let heard = Rc::new(Cell::new(0));
//! let seen = heard.clone();
//! let listener = volume.add_listener({
//!     let volume = volume.clone();
//!     move || seen.set(volume.get())
//! });
//!
//! volume.set(7);
//! assert_eq!(heard.get(), 7);
//!
//! drop(listener); // no longer told
//! volume.set(9);
//! assert_eq!(heard.get(), 7);
//! ```
//!
//! Every notification also asks the shell for a rebuild, so a tree that reads a notifier
//! while it is built shows the change without listening to it.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::component::request_rebuild;

type Listener = Rc<dyn Fn()>;

#[derive(Default)]
struct Listeners {
    entries: RefCell<Vec<(u64, Listener)>>,
    next: Cell<u64>,
}

/// Something that can be listened to.
pub trait Listenable {
    /// Calls `listener` whenever this changes, until the returned [`Subscription`] is
    /// dropped.
    fn add_listener(&self, listener: impl Fn() + 'static) -> Subscription;
}

/// The end of a listener's life: dropping it stops the notifications.
///
/// Keep it for as long as the listening should last — in a `State`, alongside the thing
/// listened to; in an effect, returned as the cleanup.
#[must_use = "a subscription stops listening as soon as it is dropped"]
pub struct Subscription {
    listeners: Weak<Listeners>,
    id: u64,
}

impl Subscription {
    /// Keeps listening for as long as the notifier lives, and forgets the handle.
    pub fn forget(self) {
        std::mem::forget(self);
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(listeners) = self.listeners.upgrade() {
            let removed = {
                let mut entries = listeners.entries.borrow_mut();
                entries
                    .iter()
                    .position(|(id, _)| *id == self.id)
                    .map(|at| entries.remove(at))
            };
            drop(removed);
        }
    }
}

/// A notifier with nothing to say but *changed*.
///
/// Cloning gives another handle to the same notifier.
#[derive(Clone, Default)]
pub struct ChangeNotifier {
    listeners: Rc<Listeners>,
}

impl ChangeNotifier {
    /// A notifier nobody is listening to yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tells every listener, in the order they were added, and asks for a rebuild.
    ///
    /// A listener that adds or drops another during the notification does not disturb it:
    /// the ones told are those there when it began.
    pub fn notify_listeners(&self) {
        let told: Vec<Listener> = self
            .listeners
            .entries
            .borrow()
            .iter()
            .map(|(_, listener)| listener.clone())
            .collect();
        for listener in told {
            listener();
        }
        request_rebuild();
    }

    /// How many listeners there are.
    pub fn listener_count(&self) -> usize {
        self.listeners.entries.borrow().len()
    }
}

impl Listenable for ChangeNotifier {
    fn add_listener(&self, listener: impl Fn() + 'static) -> Subscription {
        let id = self.listeners.next.get();
        self.listeners.next.set(id + 1);
        self.listeners
            .entries
            .borrow_mut()
            .push((id, Rc::new(listener)));
        Subscription {
            listeners: Rc::downgrade(&self.listeners),
            id,
        }
    }
}

impl std::fmt::Debug for ChangeNotifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChangeNotifier({} listeners)", self.listener_count())
    }
}

/// A notifier that holds a value, and tells its listeners when the value changes.
///
/// Cloning gives another handle to the same value.
pub struct ValueNotifier<T> {
    value: Rc<RefCell<T>>,
    notifier: ChangeNotifier,
}

impl<T> Clone for ValueNotifier<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

impl<T: 'static> ValueNotifier<T> {
    /// A notifier holding `value`.
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            notifier: ChangeNotifier::new(),
        }
    }

    /// A copy of the value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }

    /// Reads the value without copying it.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.value.borrow())
    }

    /// Replaces the value, and notifies — unless it is the value already there, in which
    /// case nothing changed and nobody is told.
    pub fn set(&self, value: T)
    where
        T: PartialEq,
    {
        if *self.value.borrow() == value {
            return;
        }
        *self.value.borrow_mut() = value;
        self.notifier.notify_listeners();
    }

    /// Changes the value in place, and notifies.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.value.borrow_mut());
        self.notifier.notify_listeners();
    }
}

impl<T: Default + 'static> Default for ValueNotifier<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Listenable for ValueNotifier<T> {
    fn add_listener(&self, listener: impl Fn() + 'static) -> Subscription {
        self.notifier.add_listener(listener)
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ValueNotifier<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValueNotifier")
            .field(&*self.value.borrow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::take_rebuild_request;

    #[test]
    fn listeners_are_told_in_the_order_they_were_added() {
        let notifier = ChangeNotifier::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let (a, b) = (log.clone(), log.clone());
        let _a = notifier.add_listener(move || a.borrow_mut().push("a"));
        let _b = notifier.add_listener(move || b.borrow_mut().push("b"));
        notifier.notify_listeners();
        assert_eq!(*log.borrow(), ["a", "b"]);
    }

    #[test]
    fn a_dropped_subscription_stops_the_notifications() {
        let notifier = ChangeNotifier::new();
        let count = Rc::new(Cell::new(0));
        let counted = count.clone();
        let subscription = notifier.add_listener(move || counted.set(counted.get() + 1));
        notifier.notify_listeners();
        assert_eq!(notifier.listener_count(), 1);
        drop(subscription);
        assert_eq!(notifier.listener_count(), 0);
        notifier.notify_listeners();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn a_forgotten_subscription_keeps_listening() {
        let notifier = ChangeNotifier::new();
        let count = Rc::new(Cell::new(0));
        let counted = count.clone();
        notifier
            .add_listener(move || counted.set(counted.get() + 1))
            .forget();
        notifier.notify_listeners();
        notifier.notify_listeners();
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn a_listener_may_drop_another_during_the_notification() {
        let notifier = ChangeNotifier::new();
        let count = Rc::new(Cell::new(0));
        let slot: Rc<RefCell<Option<Subscription>>> = Rc::default();
        let counted = count.clone();
        let victim = notifier.add_listener(move || counted.set(counted.get() + 1));
        *slot.borrow_mut() = Some(victim);
        let dropper = slot.clone();
        let _first = notifier.add_listener(move || {
            dropper.borrow_mut().take();
        });
        // The listeners told are the ones there when it began, so the victim still hears it
        // this once — and no more.
        notifier.notify_listeners();
        notifier.notify_listeners();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn a_value_notifier_tells_only_when_the_value_changes() {
        let value = ValueNotifier::new(1);
        let count = Rc::new(Cell::new(0));
        let counted = count.clone();
        let _listener = value.add_listener(move || counted.set(counted.get() + 1));
        value.set(1);
        assert_eq!(count.get(), 0, "the same value is not a change");
        value.set(2);
        value.update(|v| *v += 1);
        assert_eq!((count.get(), value.get()), (2, 3));
    }

    #[test]
    fn a_change_asks_for_a_rebuild() {
        take_rebuild_request();
        let value = ValueNotifier::new(String::new());
        value.set("x".into());
        assert!(take_rebuild_request());
        value.set("x".into());
        assert!(!take_rebuild_request(), "and no change asks for nothing");
    }
}
