//! **Components**: [`StatelessWidget`], [`StatefulWidget`] with its [`State`], and the
//! hooks that let a plain function keep state too.
//!
//! An interface is a tree of widgets, and some of them are not drawn: they are *built* —
//! a component is a widget whose only job is to say which other widgets it stands for.
//! `build` is called when the tree is rebuilt, and what it returns is put in its place.
//!
//! ```
//! use frus_widgets::{text, BuildContext, StatelessWidget, Widget};
//!
//! struct Greeting {
//!     name: String,
//! }
//!
//! impl StatelessWidget for Greeting {
//!     fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
//!         Box::new(text(format!("Hello, {}", self.name)))
//!     }
//! }
//!
//! // used anywhere a widget is:
//! let greeting = Greeting { name: "world".into() }.into_widget();
//! # let _ = greeting;
//! ```
//!
//! ## State that outlives a rebuild
//!
//! The tree is rebuilt from scratch whenever something changes, so anything a widget must
//! remember cannot live in the widget. A [`StatefulWidget`] splits in two: the widget is
//! the *configuration* — cheap, thrown away and made again on every rebuild — and its
//! [`State`] is what stays. The framework keeps each `State` under the widget's place in
//! the tree, calls [`State::init_state`] when it is first created, [`State::did_update_widget`]
//! whenever the configuration above it changes, and [`State::dispose`] when the widget
//! leaves the tree.
//!
//! ```
//! use frus_widgets::{
//!     button, column, text, BuildContext, State, StateContext, StatefulWidget, Widget,
//! };
//!
//! struct Counter;
//!
//! struct CounterState {
//!     count: i32,
//! }
//!
//! impl StatefulWidget for Counter {
//!     type State = CounterState;
//!     fn create_state(&self) -> CounterState {
//!         CounterState { count: 0 }
//!     }
//! }
//!
//! impl State for CounterState {
//!     type Widget = Counter;
//!     fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
//!         Box::new(column![
//!             text(format!("{}", self.count)),
//!             // `callback` is `set_state` wrapped up as a handler
//!             button("+", cx.callback(|state| state.count += 1)),
//!         ])
//!     }
//! }
//! ```
//!
//! ## State without a struct
//!
//! A function of `&BuildContext` is a component too, and inside one, **hooks** —
//! [`BuildContext::use_state`], [`BuildContext::use_ref`], [`BuildContext::use_memo`],
//! [`BuildContext::use_effect`] — keep what it needs between rebuilds. They are matched to
//! their values by the order they are called in, so they are called **unconditionally and in
//! the same order** every time: never inside an `if`, a loop or a closure.
//!
//! ```
//! use frus_widgets::{button, column, text, BuildContext, StatelessWidget, Widget};
//!
//! fn counter(cx: &BuildContext) -> Box<dyn Widget> {
//!     let count = cx.use_state(|| 0);
//!     let add = count.clone();
//!     Box::new(column![
//!         text(format!("{}", count.get())),
//!         button("+", move || add.update(|n| *n += 1)),
//!     ])
//! }
//!
//! let widget = counter.into_widget();
//! # let _ = widget;
//! ```
//!
//! ## Where a state lives
//!
//! A state is found again by **where its widget sits**: the same place in the tree, the
//! same kind of widget. Give a widget a key ([`Component::with_key`], or `keyed(..)`
//! around it) and it is found by the key instead, wherever its siblings move it. A widget
//! that is no longer built in a frame that rebuilt the tree is disposed at the end of that
//! frame.

use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::callback::Callback;
use crate::interaction::{Status, WidgetId};
use crate::runtime::Runtime;
use crate::theme::Theme;
use crate::widget::Widget;

// ---------------------------------------------------------------------------------------
// Asking for a rebuild
// ---------------------------------------------------------------------------------------

thread_local! {
    static REBUILD: Cell<bool> = const { Cell::new(false) };
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Asks the shell to rebuild the tree on its next frame. Anything that changed what a
/// component would build — [`StateHandle::set_state`], a hook's `set`, a controller —
/// calls this; an application rarely does.
pub fn request_rebuild() {
    REBUILD.with(|flag| flag.set(true));
}

/// Whether a rebuild has been asked for and not yet taken. Asking does not consume it.
pub fn rebuild_requested() -> bool {
    REBUILD.with(|flag| flag.get())
}

/// Whether a rebuild was asked for since the last time this was called, and forgets it.
/// The shell reads this at the top of every frame.
pub fn take_rebuild_request() -> bool {
    REBUILD.with(|flag| flag.replace(false))
}

/// How many components deep the transparent chain being expanded is. A component that
/// builds another component *directly* is the same place in the tree as it, so they are
/// told apart by this.
struct Deeper(u32);

impl Deeper {
    fn enter() -> Self {
        let now = DEPTH.with(|d| d.get());
        DEPTH.with(|d| d.set(now + 1));
        Deeper(now)
    }
}

impl Drop for Deeper {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(self.0));
    }
}

// ---------------------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------------------

/// Where one component's state is found: its place in the tree, and how far down a chain of
/// components that place is.
type StateKey = (WidgetId, u32);

/// What every component's state, and every hook's, is kept in between rebuilds. It lives in
/// the [`Runtime`], under the place of the component that owns it.
#[derive(Default)]
pub struct StateStore {
    entries: RefCell<HashMap<StateKey, Entry>>,
    /// Counts the rebuilds. What a rebuild does not touch, it has left behind.
    epoch: Cell<u64>,
    /// Whether a rebuild has run since the last sweep.
    building: Cell<bool>,
    effects: RefCell<Vec<PendingEffect>>,
    services: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
}

struct Entry {
    kind: TypeId,
    epoch: u64,
    state: Option<Rc<dyn Any>>,
    hooks: Rc<Hooks>,
    dispose: Option<Box<dyn FnOnce()>>,
}

impl Entry {
    fn dispose_all(self) {
        if let Some(dispose) = self.dispose {
            dispose();
        }
        self.hooks.dispose();
    }
}

/// Work that undoes what an effect did.
type Cleanup = Box<dyn FnOnce()>;

struct PendingEffect {
    hooks: Rc<Hooks>,
    index: usize,
    run: Box<dyn FnOnce() -> Option<Cleanup>>,
}

impl StateStore {
    /// A rebuild of the tree is about to run. Every state it reaches is marked as reached.
    pub fn begin_build(&self) {
        self.epoch.set(self.epoch.get() + 1);
        self.building.set(true);
    }

    /// The frame is done laying out: the states a rebuild did not reach are disposed, and
    /// the effects it asked for run.
    ///
    /// Effects run *after* the tree is built and laid out, so that what they do — start a
    /// timer, read a controller, ask for another rebuild — never happens in the middle of
    /// building.
    pub fn end_frame(&self) {
        if self.building.replace(false) {
            self.sweep();
        }
        self.flush_effects();
    }

    /// Makes `value` available to every component below as a service: the way an
    /// application-wide object — a router, a repository — is reached from a `build`.
    pub fn provide<T: 'static>(&self, value: Rc<T>) {
        self.services.borrow_mut().insert(TypeId::of::<T>(), value);
    }

    /// The service of type `T`, if one was provided.
    pub fn service<T: 'static>(&self) -> Option<Rc<T>> {
        self.services
            .borrow()
            .get(&TypeId::of::<T>())
            .and_then(|s| s.clone().downcast::<T>().ok())
    }

    /// How many components currently hold state. For tests and diagnostics.
    pub fn len(&self) -> usize {
        self.entries.borrow().len()
    }

    /// Whether no component holds any state.
    pub fn is_empty(&self) -> bool {
        self.entries.borrow().is_empty()
    }

    fn sweep(&self) {
        let epoch = self.epoch.get();
        let stale: Vec<Entry> = {
            let mut entries = self.entries.borrow_mut();
            let keys: Vec<StateKey> = entries
                .iter()
                .filter(|(_, entry)| entry.epoch < epoch)
                .map(|(key, _)| *key)
                .collect();
            keys.into_iter()
                .filter_map(|key| entries.remove(&key))
                .collect()
        };
        for entry in stale {
            entry.dispose_all();
        }
    }

    fn flush_effects(&self) {
        let pending: Vec<PendingEffect> = std::mem::take(&mut *self.effects.borrow_mut());
        for effect in pending {
            if effect.hooks.disposed.get() {
                continue;
            }
            let previous = effect.hooks.take_cleanup(effect.index);
            if let Some(cleanup) = previous {
                cleanup();
            }
            let cleanup = (effect.run)();
            effect.hooks.set_cleanup(effect.index, cleanup);
        }
    }

    /// The hooks of the component at `key`, made if it has none yet. A component of a
    /// different kind that was there is disposed: the place is the same, the widget is not.
    fn hooks_for(&self, key: StateKey, kind: TypeId) -> Rc<Hooks> {
        let stale = {
            let mut entries = self.entries.borrow_mut();
            match entries.get_mut(&key) {
                Some(entry) if entry.kind == kind => {
                    entry.epoch = self.epoch.get();
                    return entry.hooks.clone();
                }
                Some(_) => entries.remove(&key),
                None => None,
            }
        };
        if let Some(stale) = stale {
            stale.dispose_all();
        }
        let hooks = Rc::new(Hooks::default());
        self.entries.borrow_mut().insert(
            key,
            Entry {
                kind,
                epoch: self.epoch.get(),
                state: None,
                hooks: hooks.clone(),
                dispose: None,
            },
        );
        hooks
    }

    /// The state of the stateful widget at `key`: found if there is one, created if not.
    /// The widget that was there before comes back with a state that already existed.
    fn mount<W: StatefulWidget>(&self, key: StateKey, widget: Rc<W>) -> Mounted<W> {
        let kind = TypeId::of::<W>();
        let found = {
            let mut entries = self.entries.borrow_mut();
            entries
                .get_mut(&key)
                .filter(|entry| entry.kind == kind)
                .map(|entry| {
                    entry.epoch = self.epoch.get();
                    (entry.state.clone(), entry.hooks.clone())
                })
        };
        if let Some((Some(state), hooks)) = found {
            let cell = state
                .downcast::<StateCell<W::State>>()
                .unwrap_or_else(|_| unreachable!("an entry's kind says what its state is"));
            let previous = std::mem::replace(&mut *cell.widget.borrow_mut(), widget);
            return Mounted {
                cell,
                hooks,
                previous: Some(previous),
            };
        }
        let stale = self.entries.borrow_mut().remove(&key);
        if let Some(stale) = stale {
            stale.dispose_all();
        }
        let cell = Rc::new(StateCell {
            state: RefCell::new(widget.create_state()),
            widget: RefCell::new(widget),
            mounted: Cell::new(true),
        });
        let hooks = Rc::new(Hooks::default());
        let leaving = cell.clone();
        self.entries.borrow_mut().insert(
            key,
            Entry {
                kind,
                epoch: self.epoch.get(),
                state: Some(cell.clone() as Rc<dyn Any>),
                hooks: hooks.clone(),
                dispose: Some(Box::new(move || {
                    leaving.mounted.set(false);
                    if let Ok(mut state) = leaving.state.try_borrow_mut() {
                        state.dispose();
                    }
                })),
            },
        );
        Mounted {
            cell,
            hooks,
            previous: None,
        }
    }
}

struct Mounted<W: StatefulWidget> {
    cell: Rc<StateCell<W::State>>,
    hooks: Rc<Hooks>,
    previous: Option<Rc<W>>,
}

// ---------------------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------------------

const ORDER: &str = "hooks were called in a different order than on the last build: call every \
                     hook unconditionally, in the same order, never inside an `if`, a loop or a \
                     closure";

#[derive(Default)]
struct Hooks {
    slots: RefCell<Vec<Slot>>,
    disposed: Cell<bool>,
}

enum Slot {
    Value(Rc<dyn Any>),
    Effect {
        deps: Box<dyn Any>,
        cleanup: Option<Cleanup>,
    },
}

impl Hooks {
    /// The value in slot `index`, made by `init` the first time.
    fn value<T: 'static>(&self, index: usize, init: impl FnOnce() -> T) -> Rc<T> {
        let existing = self.slots.borrow().get(index).map(|slot| match slot {
            Slot::Value(value) => Some(value.clone()),
            Slot::Effect { .. } => None,
        });
        match existing {
            Some(Some(value)) => value.downcast::<T>().unwrap_or_else(|_| panic!("{ORDER}")),
            Some(None) => panic!("{ORDER}"),
            None => {
                let value = Rc::new(init());
                let mut slots = self.slots.borrow_mut();
                assert_eq!(slots.len(), index, "{ORDER}");
                slots.push(Slot::Value(value.clone() as Rc<dyn Any>));
                value
            }
        }
    }

    fn take_cleanup(&self, index: usize) -> Option<Cleanup> {
        match self.slots.borrow_mut().get_mut(index) {
            Some(Slot::Effect { cleanup, .. }) => cleanup.take(),
            _ => None,
        }
    }

    fn set_cleanup(&self, index: usize, cleanup: Option<Cleanup>) {
        if let Some(Slot::Effect { cleanup: slot, .. }) = self.slots.borrow_mut().get_mut(index) {
            *slot = cleanup;
        }
    }

    fn dispose(&self) {
        self.disposed.set(true);
        let cleanups: Vec<Cleanup> = self
            .slots
            .borrow_mut()
            .iter_mut()
            .filter_map(|slot| match slot {
                Slot::Effect { cleanup, .. } => cleanup.take(),
                Slot::Value(_) => None,
            })
            .collect();
        for cleanup in cleanups {
            cleanup();
        }
    }
}

/// What an effect may hand back: nothing, or the work that undoes it.
pub trait IntoCleanup {
    /// The cleanup, if there is one.
    fn into_cleanup(self) -> Option<Box<dyn FnOnce()>>;
}

impl IntoCleanup for () {
    fn into_cleanup(self) -> Option<Cleanup> {
        None
    }
}

impl<F: FnOnce() + 'static> IntoCleanup for F {
    fn into_cleanup(self) -> Option<Cleanup> {
        Some(Box::new(self))
    }
}

/// A value kept between rebuilds by [`BuildContext::use_state`]. Changing it asks for a
/// rebuild; reading it does not.
///
/// Cloning a `UseState` clones the *handle*: every clone reads and writes the same value,
/// which is what lets a handler own one.
pub struct UseState<T> {
    cell: Rc<RefCell<T>>,
}

impl<T> Clone for UseState<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T: 'static> UseState<T> {
    /// A copy of the current value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.cell.borrow().clone()
    }

    /// Reads the current value without copying it.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.cell.borrow())
    }

    /// Replaces the value, and asks for a rebuild.
    pub fn set(&self, value: T) {
        *self.cell.borrow_mut() = value;
        request_rebuild();
    }

    /// Changes the value in place, and asks for a rebuild.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.cell.borrow_mut());
        request_rebuild();
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("UseState")
            .field(&*self.cell.borrow())
            .finish()
    }
}

/// A value kept between rebuilds by [`BuildContext::use_ref`]. Unlike [`UseState`],
/// changing it does *not* ask for a rebuild: it is for what the interface does not show.
pub struct UseRef<T> {
    cell: Rc<RefCell<T>>,
}

impl<T> Clone for UseRef<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T: 'static> UseRef<T> {
    /// A copy of the current value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.cell.borrow().clone()
    }

    /// Reads the current value without copying it.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.cell.borrow())
    }

    /// Replaces the value.
    pub fn set(&self, value: T) {
        *self.cell.borrow_mut() = value;
    }

    /// Changes the value in place.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.cell.borrow_mut());
    }
}

// ---------------------------------------------------------------------------------------
// BuildContext
// ---------------------------------------------------------------------------------------

/// What a component is told when it is built: where it is, under which theme, and where
/// its hooks keep their values.
pub struct BuildContext<'a> {
    key: StateKey,
    kind: TypeId,
    runtime: &'a Runtime,
    theme: &'a Theme,
    hooks: OnceCell<Rc<Hooks>>,
    cursor: Cell<usize>,
}

impl<'a> BuildContext<'a> {
    fn new(key: StateKey, kind: TypeId, runtime: &'a Runtime, theme: &'a Theme) -> Self {
        Self {
            key,
            kind,
            runtime,
            theme,
            hooks: OnceCell::new(),
            cursor: Cell::new(0),
        }
    }

    /// The theme this component is built under.
    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// Where in the tree this component sits.
    pub fn id(&self) -> WidgetId {
        self.key.0
    }

    /// The runtime the tree is being built for.
    pub fn runtime(&self) -> &Runtime {
        self.runtime
    }

    /// An application-wide object provided with [`StateStore::provide`] — a router, say.
    pub fn service<T: 'static>(&self) -> Option<Rc<T>> {
        self.runtime.states.service::<T>()
    }

    fn hooks(&self) -> Rc<Hooks> {
        self.hooks
            .get_or_init(|| self.runtime.states.hooks_for(self.key, self.kind))
            .clone()
    }

    fn next_slot(&self) -> usize {
        let slot = self.cursor.get();
        self.cursor.set(slot + 1);
        slot
    }

    /// A value that lives as long as this component does, and that asks for a rebuild when
    /// it changes. `init` makes it the first time and is not called again.
    ///
    /// ```
    /// # use frus_widgets::{text, BuildContext, Widget};
    /// fn label(cx: &BuildContext) -> Box<dyn Widget> {
    ///     let taps = cx.use_state(|| 0);
    ///     taps.set(taps.get() + 1);
    ///     Box::new(text(format!("{}", taps.get())))
    /// }
    /// ```
    pub fn use_state<T: 'static>(&self, init: impl FnOnce() -> T) -> UseState<T> {
        let cell = self
            .hooks()
            .value(self.next_slot(), || RefCell::new(init()));
        UseState { cell }
    }

    /// A value that lives as long as this component does and does *not* ask for a
    /// rebuild when it changes.
    pub fn use_ref<T: 'static>(&self, init: impl FnOnce() -> T) -> UseRef<T> {
        let cell = self
            .hooks()
            .value(self.next_slot(), || RefCell::new(init()));
        UseRef { cell }
    }

    /// `compute()`, kept until `deps` change. The work is not repeated on a rebuild that
    /// changes nothing it depends on.
    pub fn use_memo<D: PartialEq + 'static, T: 'static>(
        &self,
        deps: D,
        compute: impl FnOnce() -> T,
    ) -> Rc<T> {
        let memo = self
            .hooks()
            .value(self.next_slot(), || RefCell::new(None::<(D, Rc<T>)>));
        let mut memo = memo.borrow_mut();
        match memo.as_ref() {
            Some((previous, value)) if *previous == deps => value.clone(),
            _ => {
                let value = Rc::new(compute());
                *memo = Some((deps, value.clone()));
                value
            }
        }
    }

    /// Work to do *after* the tree is built — start a timer, subscribe, read a controller —
    /// run again whenever `deps` change, not otherwise. Pass `()` to run it once.
    ///
    /// The work may return a closure, which undoes it: it runs before the next run and when
    /// the component leaves the tree.
    ///
    /// ```
    /// # use frus_widgets::{text, BuildContext, Widget};
    /// fn title(cx: &BuildContext) -> Box<dyn Widget> {
    ///     cx.use_effect((), || {
    ///         println!("mounted");
    ///         || println!("unmounted")
    ///     });
    ///     Box::new(text("hello"))
    /// }
    /// ```
    pub fn use_effect<D, C>(&self, deps: D, effect: impl FnOnce() -> C + 'static)
    where
        D: PartialEq + 'static,
        C: IntoCleanup,
    {
        let hooks = self.hooks();
        let index = self.next_slot();
        let changed = {
            let mut slots = hooks.slots.borrow_mut();
            match slots.get_mut(index) {
                None => {
                    assert_eq!(slots.len(), index, "{ORDER}");
                    slots.push(Slot::Effect {
                        deps: Box::new(deps),
                        cleanup: None,
                    });
                    true
                }
                Some(Slot::Effect { deps: old, .. }) => {
                    if old.downcast_ref::<D>().is_some_and(|old| *old == deps) {
                        false
                    } else {
                        *old = Box::new(deps);
                        true
                    }
                }
                Some(Slot::Value(_)) => panic!("{ORDER}"),
            }
        };
        if changed {
            self.runtime
                .states
                .effects
                .borrow_mut()
                .push(PendingEffect {
                    hooks,
                    index,
                    run: Box::new(move || effect().into_cleanup()),
                });
        }
    }
}

// ---------------------------------------------------------------------------------------
// StatelessWidget
// ---------------------------------------------------------------------------------------

/// A widget with nothing to remember: it says what it looks like from what it is
/// configured with, and from nothing else.
///
/// Any `Fn(&BuildContext) -> Box<dyn Widget>` is one, which is how a function becomes a
/// component — and, with hooks, one that remembers.
pub trait StatelessWidget: 'static {
    /// The widget this one stands for. Called whenever the tree is rebuilt.
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget>;

    /// This component as a widget: what goes in a `column![..]`, a `Scaffold`'s body, a
    /// route's page.
    fn into_widget(self) -> Component
    where
        Self: Sized,
    {
        Component::stateless(self)
    }
}

impl<F> StatelessWidget for F
where
    F: Fn(&BuildContext) -> Box<dyn Widget> + 'static,
{
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        self(cx)
    }
}

// ---------------------------------------------------------------------------------------
// StatefulWidget
// ---------------------------------------------------------------------------------------

/// A widget whose [`State`] outlives the rebuilds. The widget is only the configuration:
/// it is made again on every rebuild, and the state is what is kept.
pub trait StatefulWidget: 'static {
    /// What is kept.
    type State: State<Widget = Self>;

    /// Makes the state, once, when the widget first enters the tree.
    fn create_state(&self) -> Self::State;

    /// This component as a widget.
    fn into_widget(self) -> Component
    where
        Self: Sized,
    {
        Component::stateful(self)
    }
}

/// What a [`StatefulWidget`] keeps, and how it draws itself from it.
pub trait State: Sized + 'static {
    /// The widget this is the state of.
    type Widget: StatefulWidget<State = Self>;

    /// Called once, right after the state is made. Start what has to run for as long as
    /// the widget is there — a listener, a timer — here, and stop it in
    /// [`State::dispose`].
    fn init_state(&mut self, _cx: &StateContext<Self>) {}

    /// Called when the tree is rebuilt and the widget above changed, or was made again.
    /// `old` is the configuration it replaces; the new one is [`StateContext::widget`].
    fn did_update_widget(&mut self, _old: &Self::Widget, _cx: &StateContext<Self>) {}

    /// Called once, when the widget leaves the tree. The state is never used again.
    fn dispose(&mut self) {}

    /// The widget this state stands for. Called whenever the tree is rebuilt.
    ///
    /// It only *reads* the state. What a handler changes goes through
    /// [`StateContext::callback`] or [`StateContext::handle`]; changing the state from
    /// inside `build` itself panics, because a build that changes what it builds from
    /// never settles.
    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget>;
}

pub(crate) struct StateCell<S: State> {
    state: RefCell<S>,
    widget: RefCell<Rc<S::Widget>>,
    mounted: Cell<bool>,
}

/// A way to reach a widget's state from outside its `build`: from a handler, a listener, a
/// timer. Cheap to clone.
pub struct StateHandle<S: State> {
    cell: Rc<StateCell<S>>,
}

impl<S: State> Clone for StateHandle<S> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<S: State> StateHandle<S> {
    /// Changes the state, and asks for a rebuild.
    ///
    /// Does nothing once the widget has left the tree: a timer that outlives its widget
    /// finds nothing to change.
    pub fn set_state(&self, change: impl FnOnce(&mut S)) {
        if !self.cell.mounted.get() {
            return;
        }
        match self.cell.state.try_borrow_mut() {
            Ok(mut state) => change(&mut state),
            Err(_) => panic!(
                "set_state was called while the state was being read — from inside `build`, or \
                 from inside another `set_state`. Change state from a handler, not while building"
            ),
        }
        request_rebuild();
    }

    /// Reads the state.
    pub fn read<R>(&self, read: impl FnOnce(&S) -> R) -> R {
        read(&self.cell.state.borrow())
    }

    /// The widget's current configuration.
    pub fn widget(&self) -> Rc<S::Widget> {
        self.cell.widget.borrow().clone()
    }

    /// Whether the widget is still in the tree.
    pub fn is_mounted(&self) -> bool {
        self.cell.mounted.get()
    }

    /// A handler that changes the state, for anything that takes a [`Callback`].
    pub fn callback(&self, change: impl Fn(&mut S) + 'static) -> Callback {
        let handle = self.clone();
        Callback::new(move || handle.set_state(|state| change(state)))
    }
}

/// What a [`State`] is told when it builds and when its lifecycle runs: everything a
/// [`BuildContext`] offers (this dereferences to one), and the state's own handle.
pub struct StateContext<'c, 'a, S: State> {
    cx: &'c BuildContext<'a>,
    cell: &'c Rc<StateCell<S>>,
}

impl<'a, S: State> Deref for StateContext<'_, 'a, S> {
    type Target = BuildContext<'a>;

    fn deref(&self) -> &BuildContext<'a> {
        self.cx
    }
}

impl<S: State> StateContext<'_, '_, S> {
    /// The widget's current configuration.
    pub fn widget(&self) -> Rc<S::Widget> {
        self.cell.widget.borrow().clone()
    }

    /// A handle to this state that a handler, a listener or a timer can own.
    pub fn handle(&self) -> StateHandle<S> {
        StateHandle {
            cell: self.cell.clone(),
        }
    }

    /// A handler that changes this state — [`StateHandle::callback`], from here.
    pub fn callback(&self, change: impl Fn(&mut S) + 'static) -> Callback {
        self.handle().callback(change)
    }
}

// ---------------------------------------------------------------------------------------
// The node
// ---------------------------------------------------------------------------------------

type Builder<Msg> = Box<dyn FnOnce(&BuildContext) -> Box<dyn Widget<Msg>>>;

/// A component in the tree: a widget that is built into another when the tree is.
///
/// It is transparent: once built it *is* the widget it built, the same box, the same
/// children, the same behaviour, so wrapping something in a component changes nothing about
/// where it lands. Made by [`StatelessWidget::into_widget`] and
/// [`StatefulWidget::into_widget`].
pub struct Component<Msg = Callback> {
    inner: LazyChild<Msg>,
    key: Option<u64>,
}

impl Component<Callback> {
    /// A stateless component.
    pub fn stateless<W: StatelessWidget>(widget: W) -> Self {
        Self::from_builder(
            TypeId::of::<W>(),
            Box::new(move |cx: &BuildContext| widget.build(cx)),
        )
    }

    /// A stateful component.
    pub fn stateful<W: StatefulWidget>(widget: W) -> Self {
        Self::from_builder(
            TypeId::of::<W>(),
            Box::new(move |cx: &BuildContext| {
                let Mounted {
                    cell,
                    hooks,
                    previous,
                } = cx.runtime.states.mount(cx.key, Rc::new(widget));
                let _ = cx.hooks.set(hooks);
                let state_cx = StateContext { cx, cell: &cell };
                match previous {
                    None => cell.state.borrow_mut().init_state(&state_cx),
                    Some(old) => cell.state.borrow_mut().did_update_widget(&old, &state_cx),
                }
                let built = cell.state.borrow().build(&state_cx);
                built
            }),
        )
    }
}

impl<Msg> Component<Msg> {
    fn from_builder(kind: TypeId, build: Builder<Msg>) -> Self {
        Self {
            inner: LazyChild {
                kind,
                build: RefCell::new(Some(build)),
                built: OnceCell::new(),
                unbuilt: Unbuilt,
            },
            key: None,
        }
    }

    /// Gives this component an identity that does not depend on where it sits: its state
    /// follows the key when its siblings move.
    pub fn with_key(mut self, key: impl Hash) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        self.key = Some(hasher.finish());
        self
    }

    /// A component is its child's box: nothing to add.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

/// The child a component builds, made the first time the tree reaches it.
struct LazyChild<Msg> {
    kind: TypeId,
    build: RefCell<Option<Builder<Msg>>>,
    built: OnceCell<Box<dyn Widget<Msg>>>,
    unbuilt: Unbuilt,
}

impl<Msg> LazyChild<Msg> {
    /// Builds the child if it has not been, then lets *it* expand: what it built may be a
    /// component too.
    fn expand(&self, id: WidgetId, runtime: &Runtime, theme: &Theme) {
        if self.built.get().is_none() {
            let build = self.build.borrow_mut().take();
            if let Some(build) = build {
                let depth = DEPTH.with(|d| d.get());
                let cx = BuildContext::new((id, depth), self.kind, runtime, theme);
                let child = build(&cx);
                let _ = self.built.set(child);
            }
        }
        if let Some(child) = self.built.get() {
            let _deeper = Deeper::enter();
            child.expand(id, runtime, theme);
        }
    }

    fn build_in(&self, id: WidgetId, runtime: &Runtime, theme: &Theme) {
        if let Some(child) = self.built.get() {
            child.build_in(id, runtime, theme);
        }
    }

    fn build_themed(&self, theme: &Theme) {
        if let Some(child) = self.built.get() {
            child.build_themed(theme);
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        match self.built.get() {
            Some(child) => child.children(),
            None => {
                #[cfg(debug_assertions)]
                if self.build.borrow().is_some() {
                    panic!(
                        "a component's children were read before it was built: a traversal \
                         reached this tree before the walk that builds components did. Call \
                         `build_deferred(tree, &theme, &runtime)` on it first"
                    );
                }
                &[]
            }
        }
    }
}

impl<Msg> Deref for LazyChild<Msg> {
    type Target = dyn Widget<Msg>;

    fn deref(&self) -> &(dyn Widget<Msg> + 'static) {
        match self.built.get() {
            Some(child) => child.as_ref(),
            None => &self.unbuilt,
        }
    }
}

/// What a component answers as before it is built: nothing.
struct Unbuilt;

impl<Msg> Widget<Msg> for Unbuilt {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "Component"
    }
}

crate::transparent::forward_transparent!(Component {
    /// A component's key is its own and never its child's: the parent reads it to place
    /// the component *before* the component has been built, and an answer that changed
    /// when the child arrived would move the place under the state that lives there.
    fn key(&self) -> Option<u64> {
        self.key
    }

    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }

    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }

    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }

    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }

    fn autofill_group(&self) -> bool {
        self.inner.autofill_group()
    }
});

/// A component made of a function: `component(|cx| …)`. The closure is called whenever the
/// tree is rebuilt, and may call hooks.
///
/// ```
/// use frus_widgets::{component, text};
///
/// let clock = component(|cx| {
///     let ticks = cx.use_state(|| 0);
///     Box::new(text(format!("{} ticks", ticks.get())))
/// });
/// # let _ = clock;
/// ```
pub fn component(build: impl Fn(&BuildContext) -> Box<dyn Widget> + 'static) -> Component {
    Component::stateless(build)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_deferred, keyed, Container, Flex};
    use frus_layout::Dimension;

    type Builder = Rc<dyn Fn(&BuildContext) -> Box<dyn Widget>>;
    type Log = Rc<RefCell<Vec<String>>>;
    type Handle = Rc<RefCell<Option<StateHandle<LifeState>>>>;

    fn log() -> Log {
        Rc::new(RefCell::new(Vec::new()))
    }

    fn taken(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    /// One frame of the shell's, less the drawing: the tree is built, and what it did not
    /// reach is let go.
    fn frame(runtime: &Runtime, root: &dyn Widget) {
        runtime.states.begin_build();
        build_deferred(root, &Theme::default(), runtime);
        runtime.states.end_frame();
    }

    /// A stateful widget that says what happens to it.
    struct Life {
        tag: u32,
        log: Log,
        out: Handle,
    }

    struct LifeState {
        log: Log,
        out: Handle,
        count: i32,
    }

    impl StatefulWidget for Life {
        type State = LifeState;
        fn create_state(&self) -> LifeState {
            LifeState {
                log: self.log.clone(),
                out: self.out.clone(),
                count: 0,
            }
        }
    }

    impl State for LifeState {
        type Widget = Life;
        fn init_state(&mut self, cx: &StateContext<Self>) {
            self.log
                .borrow_mut()
                .push(format!("init {}", cx.widget().tag));
            *self.out.borrow_mut() = Some(cx.handle());
        }
        fn did_update_widget(&mut self, old: &Life, cx: &StateContext<Self>) {
            self.log
                .borrow_mut()
                .push(format!("update {}->{}", old.tag, cx.widget().tag));
        }
        fn dispose(&mut self) {
            self.log.borrow_mut().push("dispose".to_string());
        }
        fn build(&self, _cx: &StateContext<Self>) -> Box<dyn Widget> {
            Box::new(Container::new().width(10.0 + self.count as f32).height(4.0))
        }
    }

    fn life(tag: u32, log: &Log, out: &Handle) -> Component {
        Life {
            tag,
            log: log.clone(),
            out: out.clone(),
        }
        .into_widget()
    }

    fn handle() -> Handle {
        Rc::new(RefCell::new(None))
    }

    fn width(widget: &dyn Widget) -> Dimension {
        widget.style().width
    }

    #[test]
    fn a_stateless_component_is_the_widget_it_builds() {
        let runtime = Runtime::default();
        let root = component(|_| Box::new(Container::new().width(120.0).height(30.0)));
        frame(&runtime, &root);
        assert_eq!(width(&root), Dimension::Length(120.0));
        assert_eq!(root.style().height, Dimension::Length(30.0));
    }

    #[test]
    fn a_state_is_made_once_and_survives_rebuilds() {
        let (runtime, log, out) = (Runtime::default(), log(), handle());
        let first = life(1, &log, &out);
        frame(&runtime, &first);
        assert_eq!(taken(&log), ["init 1"]);
        assert_eq!(width(&first), Dimension::Length(10.0));

        // the state is changed from outside, as a handler does
        out.borrow().as_ref().unwrap().set_state(|s| s.count = 5);
        assert!(take_rebuild_request(), "set_state asks for a rebuild");

        let second = life(2, &log, &out);
        frame(&runtime, &second);
        assert_eq!(
            taken(&log),
            ["update 1->2"],
            "the same place, the same kind: the state is kept and told what changed"
        );
        assert_eq!(
            width(&second),
            Dimension::Length(15.0),
            "and it is the kept one"
        );
    }

    #[test]
    fn a_state_is_disposed_when_a_rebuild_no_longer_reaches_it() {
        let (runtime, log, out) = (Runtime::default(), log(), handle());
        frame(&runtime, &Flex::column().child(life(1, &log, &out)));
        taken(&log);
        assert_eq!(runtime.states.len(), 1);

        frame(&runtime, &Flex::column().child(Container::new()));
        assert_eq!(taken(&log), ["dispose"]);
        assert!(runtime.states.is_empty());
        let handle = out.borrow().clone().unwrap();
        assert!(!handle.is_mounted());
        take_rebuild_request();
        handle.set_state(|s| s.count = 9);
        assert!(
            !take_rebuild_request(),
            "a state that has left cannot ask for a rebuild"
        );
    }

    #[test]
    fn a_different_kind_of_widget_in_the_same_place_starts_over() {
        let (runtime, log, out) = (Runtime::default(), log(), handle());
        frame(&runtime, &Flex::column().child(life(1, &log, &out)));
        taken(&log);
        // a stateless component that keeps hooks takes the place the state had
        let other = component(|cx| {
            cx.use_state(|| 0u8);
            Box::new(Container::new())
        });
        frame(&runtime, &Flex::column().child(other));
        assert_eq!(
            taken(&log),
            ["dispose"],
            "the old kind is disposed, not reused"
        );
    }

    #[test]
    fn a_key_carries_the_state_wherever_the_siblings_go() {
        let (runtime, log) = (Runtime::default(), log());
        let (a, b) = (handle(), handle());
        let column = |order: [u32; 2]| {
            let mut column = Flex::column();
            for tag in order {
                let out = if tag == 1 { &a } else { &b };
                column = column.child(keyed(tag, life(tag, &log, out)));
            }
            column
        };
        frame(&runtime, &column([1, 2]));
        a.borrow().as_ref().unwrap().set_state(|s| s.count = 1);
        b.borrow().as_ref().unwrap().set_state(|s| s.count = 2);
        taken(&log);

        frame(&runtime, &column([2, 1]));
        let counts = |h: &Handle| h.borrow().as_ref().unwrap().read(|s| s.count);
        assert_eq!(
            (counts(&a), counts(&b)),
            (1, 2),
            "each state stayed with its key"
        );
        assert_eq!(runtime.states.len(), 2);
        assert!(
            !taken(&log)
                .iter()
                .any(|line| line == "dispose" || line.starts_with("init")),
            "and neither was made again or let go"
        );
    }

    #[test]
    fn a_component_that_builds_a_component_keeps_both_states() {
        let (runtime, log, out) = (Runtime::default(), log(), handle());
        let (inner_log, inner_out) = (log.clone(), out.clone());
        let outer = component(move |cx| {
            cx.use_state(|| 0);
            // built *directly*: the same place in the tree as this component
            Box::new(life(7, &inner_log, &inner_out))
        });
        frame(&runtime, &outer);
        assert_eq!(
            runtime.states.len(),
            2,
            "one for the hooks, one for the state"
        );
        assert_eq!(taken(&log), ["init 7"]);
    }

    #[test]
    fn hooks_keep_their_values_between_builds() {
        let runtime = Runtime::default();
        let seen = Rc::new(RefCell::new(Vec::new()));
        // One closure, built again by every frame: the same kind of component each time.
        let build = {
            let seen = seen.clone();
            Rc::new(move |cx: &BuildContext| -> Box<dyn Widget> {
                let taps = cx.use_state(|| 10);
                let scratch = cx.use_ref(|| 0);
                scratch.update(|n| *n += 1);
                taps.update(|n| *n += 1);
                seen.borrow_mut().push((taps.get(), scratch.get()));
                Box::new(Container::new())
            })
        };
        let make = |build: &Builder| {
            let build = build.clone();
            Component::stateless(move |cx: &BuildContext| build(cx))
        };
        let build: Builder = build;
        frame(&runtime, &make(&build));
        assert!(
            take_rebuild_request(),
            "changing a state hook asks for a rebuild"
        );
        frame(&runtime, &make(&build));
        frame(&runtime, &make(&build));
        assert_eq!(*seen.borrow(), [(11, 1), (12, 2), (13, 3)]);
    }

    #[test]
    fn use_memo_recomputes_only_when_its_dependencies_change() {
        let runtime = Runtime::default();
        let computed = Rc::new(Cell::new(0));
        let dep = Rc::new(Cell::new(1));
        let build: Builder = {
            let (computed, dep) = (computed.clone(), dep.clone());
            Rc::new(move |cx| {
                let d = dep.get();
                let value = cx.use_memo(d, || {
                    computed.set(computed.get() + 1);
                    d * 100
                });
                Box::new(Container::new().width(*value as f32))
            })
        };
        let make = || {
            let build = build.clone();
            Component::stateless(move |cx: &BuildContext| build(cx))
        };
        let root = make();
        frame(&runtime, &root);
        assert_eq!(
            (computed.get(), width(&root)),
            (1, Dimension::Length(100.0))
        );
        let root = make();
        frame(&runtime, &root);
        assert_eq!(computed.get(), 1, "the same dependency: not computed again");
        dep.set(2);
        let root = make();
        frame(&runtime, &root);
        assert_eq!(
            (computed.get(), width(&root)),
            (2, Dimension::Length(200.0))
        );
    }

    #[test]
    fn effects_run_after_the_build_and_clean_up_before_the_next_and_at_the_end() {
        let (runtime, log) = (Runtime::default(), log());
        let dep = Rc::new(Cell::new(1));
        let build: Builder = {
            let (log, dep) = (log.clone(), dep.clone());
            Rc::new(move |cx| {
                let d = dep.get();
                let log = log.clone();
                log.borrow_mut().push(format!("build {d}"));
                cx.use_effect(d, move || {
                    log.borrow_mut().push(format!("effect {d}"));
                    move || log.borrow_mut().push(format!("cleanup {d}"))
                });
                Box::new(Container::new())
            })
        };
        let make = || {
            let build = build.clone();
            Component::stateless(move |cx: &BuildContext| build(cx))
        };
        frame(&runtime, &make());
        assert_eq!(
            taken(&log),
            ["build 1", "effect 1"],
            "after the build, not during it"
        );

        frame(&runtime, &make());
        assert_eq!(
            taken(&log),
            ["build 1"],
            "the same dependency: nothing runs"
        );

        dep.set(2);
        frame(&runtime, &make());
        assert_eq!(
            taken(&log),
            ["build 2", "cleanup 1", "effect 2"],
            "the old one is undone before the new one starts"
        );

        // leaving the tree runs the last cleanup
        frame(&runtime, &Flex::column().child(Container::new()));
        assert_eq!(taken(&log), ["cleanup 2"]);
    }

    #[test]
    #[should_panic(expected = "different order")]
    fn hooks_called_in_a_different_order_are_caught() {
        let runtime = Runtime::default();
        let first = Rc::new(Cell::new(true));
        let build: Builder = {
            let first = first.clone();
            Rc::new(move |cx| {
                if first.get() {
                    cx.use_state(|| 1i32);
                }
                cx.use_ref(String::new);
                Box::new(Container::new())
            })
        };
        let make = || {
            let build = build.clone();
            Component::stateless(move |cx: &BuildContext| build(cx))
        };
        frame(&runtime, &make());
        first.set(false);
        frame(&runtime, &make());
    }

    #[test]
    #[should_panic(expected = "set_state was called while")]
    fn changing_state_from_inside_build_is_refused() {
        struct Greedy;
        struct GreedyState;
        impl StatefulWidget for Greedy {
            type State = GreedyState;
            fn create_state(&self) -> GreedyState {
                GreedyState
            }
        }
        impl State for GreedyState {
            type Widget = Greedy;
            fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
                cx.handle().set_state(|_| {});
                Box::new(Container::new())
            }
        }
        frame(&Runtime::default(), &Greedy.into_widget());
    }

    #[test]
    fn a_service_provided_to_the_runtime_is_reachable_from_a_build() {
        let runtime = Runtime::default();
        runtime.states.provide(Rc::new(41u32));
        let root = component(|cx| {
            let n = cx.service::<u32>().expect("provided");
            Box::new(Container::new().width(*n as f32 + 1.0))
        });
        frame(&runtime, &root);
        assert_eq!(width(&root), Dimension::Length(42.0));
        assert!(runtime.states.service::<String>().is_none());
    }
}
