//! Work that finishes later, in a component: [`BuildContext::use_future`],
//! [`BuildContext::use_stream`], and the builders over them — [`FutureBuilder`],
//! [`StreamBuilder`] — with [`ValueListenableBuilder`], [`ListenableBuilder`] and [`Builder`]
//! (milestone 585).
//!
//! The widget layer has no executor of its own and no dependency that brings one. The shell
//! hands it one on the way up ([`set_task_spawner`]), as it hands it the image fetcher: its
//! own executor natively, the browser's on the Web. A value that arrives is written where the
//! component reads it, and the next frame rebuilds.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::component::{component, BuildContext, Component};
use crate::notifier::{Listenable, ValueNotifier};
use crate::widget::Widget;

/// `Send` where there are threads, and nothing on the Web, where a future never leaves the
/// thread it was made on — the bound the work handed to a component must meet.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}
/// `Send` where there are threads, and nothing on the Web.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

/// A piece of work handed to the executor: it runs to its end and gives nothing back.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedTask = Pin<Box<dyn Future<Output = ()> + Send>>;
/// A piece of work handed to the executor: it runs to its end and gives nothing back.
#[cfg(target_arch = "wasm32")]
pub type BoxedTask = Pin<Box<dyn Future<Output = ()>>>;

/// Runs a [`BoxedTask`] somewhere, to its end.
pub type TaskSpawner = fn(BoxedTask);

static SPAWNER: Mutex<Option<TaskSpawner>> = Mutex::new(None);
static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static ARRIVALS: AtomicU64 = AtomicU64::new(0);

/// Tells the widget layer how to run the work a component starts.
///
/// An application using the framework's own shell never calls this: the shell registers its
/// executor on the way up. With none registered, each piece of work gets a thread of its own
/// natively, and does not run on the Web.
pub fn set_task_spawner(spawner: TaskSpawner) {
    if let Ok(mut guard) = SPAWNER.lock() {
        *guard = Some(spawner);
    }
}

/// How many pieces of work a component started are still running. The shell keeps frames
/// coming while there are any, so that what they bring is shown.
pub fn tasks_in_flight() -> usize {
    IN_FLIGHT.load(Ordering::SeqCst)
}

/// How many values pieces of work have brought so far, in this process. A shell keeps the
/// count it last saw and rebuilds when it has grown: a count rather than a flag, so that one
/// shell reading it takes nothing away from another.
pub fn task_arrivals() -> u64 {
    ARRIVALS.load(Ordering::SeqCst)
}

fn spawn(task: BoxedTask) {
    let spawner = SPAWNER.lock().ok().and_then(|guard| *guard);
    match spawner {
        Some(spawner) => spawner(task),
        None => fallback(task),
    }
}

/// No executor registered: a thread of its own, parked while the work waits.
#[cfg(not(target_arch = "wasm32"))]
fn fallback(task: BoxedTask) {
    std::thread::spawn(move || block_on(task));
}

/// No executor registered on the Web: there is no thread to give it, so it does not run.
#[cfg(target_arch = "wasm32")]
fn fallback(_task: BoxedTask) {}

/// Drives `task` to its end on this thread, parking it while the task waits.
#[cfg(not(target_arch = "wasm32"))]
fn block_on(mut task: BoxedTask) {
    use std::task::{Context, Poll, Wake, Waker};
    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    while let Poll::Pending = task.as_mut().poll(&mut cx) {
        std::thread::park();
    }
}

/// Where a piece of work that finishes later has got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// Started, and nothing has come of it yet.
    Waiting,
    /// A stream that has given at least one value and has not finished.
    Active,
    /// Finished.
    Done,
}

/// What a component knows of a piece of work that finishes later: where it has got to, and
/// the last value it gave, if any — the reference's `AsyncSnapshot`.
///
/// An error is a value like any other: work that can fail gives a `Result`.
#[derive(Clone, Debug, PartialEq)]
pub struct AsyncSnapshot<T> {
    /// Where the work has got to.
    pub state: ConnectionState,
    /// The last value it gave: a future's result once it is done, a stream's latest.
    pub data: Option<T>,
}

impl<T> AsyncSnapshot<T> {
    /// Started, nothing yet.
    pub fn waiting() -> Self {
        Self {
            state: ConnectionState::Waiting,
            data: None,
        }
    }

    /// Whether a value has come.
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }
}

type Slot<T> = Arc<Mutex<AsyncSnapshot<T>>>;

/// Records that something arrived, so that the next frame rebuilds.
fn arrived() {
    ARRIVALS.fetch_add(1, Ordering::SeqCst);
}

/// Where a stream started by [`BuildContext::use_stream`] gives its values.
pub struct StreamSink<T> {
    slot: Slot<T>,
}

impl<T> StreamSink<T> {
    /// Gives a value: the component sees it, as the latest, at the next frame.
    pub fn add(&self, value: T) {
        if let Ok(mut snapshot) = self.slot.lock() {
            snapshot.state = ConnectionState::Active;
            snapshot.data = Some(value);
        }
        arrived();
    }
}

impl<'a> BuildContext<'a> {
    /// Runs the future `make()` returns, **once until `deps` change**, and gives what is known
    /// of it: waiting, then done with its value. A rebuild in between does not start it again.
    ///
    /// Natively the future runs on the shell's executor, so it must be `Send`; on the Web the
    /// browser runs it. A value that arrives rebuilds the tree at the next frame. When `deps`
    /// change, a new future starts, and what the old one gives is not shown.
    ///
    /// ```
    /// use frus_widgets::{component, text, ConnectionState};
    ///
    /// let answer = component(|cx| {
    ///     let snapshot = cx.use_future((), || async { 6 * 7 });
    ///     Box::new(match snapshot.data {
    ///         Some(value) => text(format!("{value}")),
    ///         None => text("Working it out…"),
    ///     })
    /// });
    /// # let _ = answer;
    /// ```
    pub fn use_future<D, F>(&self, deps: D, make: impl FnOnce() -> F) -> AsyncSnapshot<F::Output>
    where
        D: PartialEq + 'static,
        F: Future + MaybeSend + 'static,
        F::Output: Clone + MaybeSend + 'static,
    {
        let slot = self.use_memo(deps, || {
            let slot: Slot<F::Output> = Arc::new(Mutex::new(AsyncSnapshot::waiting()));
            let written = slot.clone();
            let future = make();
            IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
            spawn(Box::pin(async move {
                let value = future.await;
                if let Ok(mut snapshot) = written.lock() {
                    *snapshot = AsyncSnapshot {
                        state: ConnectionState::Done,
                        data: Some(value),
                    };
                }
                IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
                arrived();
            }));
            slot
        });
        let snapshot = slot.lock().map(|s| s.clone());
        snapshot.unwrap_or_else(|_| AsyncSnapshot::waiting())
    }

    /// Runs the work `make` returns, **once until `deps` change**, handing it a
    /// [`StreamSink`] to give its values to, and gives what is known of it: waiting, active
    /// with the latest value, then done with the last one.
    ///
    /// The reference builds this on a stream type; here the stream is the work itself, giving
    /// its values as it goes, which asks for no stream library.
    ///
    /// ```
    /// use frus_widgets::{component, text};
    ///
    /// let countdown = component(|cx| {
    ///     let snapshot = cx.use_stream((), |sink| async move {
    ///         for left in (0..3).rev() {
    ///             sink.add(left);
    ///         }
    ///     });
    ///     Box::new(text(format!("{:?}", snapshot.data)))
    /// });
    /// # let _ = countdown;
    /// ```
    pub fn use_stream<D, T, F>(
        &self,
        deps: D,
        make: impl FnOnce(StreamSink<T>) -> F,
    ) -> AsyncSnapshot<T>
    where
        D: PartialEq + 'static,
        T: Clone + MaybeSend + 'static,
        F: Future<Output = ()> + MaybeSend + 'static,
    {
        let slot = self.use_memo(deps, || {
            let slot: Slot<T> = Arc::new(Mutex::new(AsyncSnapshot::waiting()));
            let written = slot.clone();
            let future = make(StreamSink { slot: slot.clone() });
            IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
            spawn(Box::pin(async move {
                future.await;
                if let Ok(mut snapshot) = written.lock() {
                    snapshot.state = ConnectionState::Done;
                }
                IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
                arrived();
            }));
            slot
        });
        let snapshot = slot.lock().map(|s| s.clone());
        snapshot.unwrap_or_else(|_| AsyncSnapshot::waiting())
    }
}

/// A component built by a closure — the reference's `Builder`: a place in the tree that has a
/// [`BuildContext`] of its own, for hooks, without declaring a widget type.
pub struct Builder;

impl Builder {
    /// A component built by `build`. The same as [`component`], under the reference's name.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(build: impl Fn(&BuildContext) -> Box<dyn Widget> + 'static) -> Component {
        component(build)
    }
}

/// Builds from a future — the reference's `FutureBuilder`, over
/// [`BuildContext::use_future`].
///
/// ```
/// use frus_widgets::{text, FutureBuilder};
///
/// let greeting = FutureBuilder::new(
///     "en",
///     |lang: &&str| {
///         let lang = *lang;
///         async move { if lang == "en" { "Hello" } else { "Bonjour" } }
///     },
///     |_, snapshot| Box::new(text(snapshot.data.unwrap_or("…"))),
/// );
/// # let _ = greeting;
/// ```
///
/// The future starts once, and again only when `key` changes.
pub struct FutureBuilder;

impl FutureBuilder {
    /// A component that runs `make(&key)` once per `key`, and builds from what it gives.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<K, F>(
        key: K,
        make: impl Fn(&K) -> F + 'static,
        build: impl Fn(&BuildContext, AsyncSnapshot<F::Output>) -> Box<dyn Widget> + 'static,
    ) -> Component
    where
        K: Clone + PartialEq + 'static,
        F: Future + MaybeSend + 'static,
        F::Output: Clone + MaybeSend + 'static,
    {
        component(move |cx| {
            let snapshot = cx.use_future(key.clone(), || make(&key));
            build(cx, snapshot)
        })
    }
}

/// Builds from a stream of values — the reference's `StreamBuilder`, over
/// [`BuildContext::use_stream`]: the work is given a [`StreamSink`] and adds its values to it.
pub struct StreamBuilder;

impl StreamBuilder {
    /// A component that runs `make(&key, sink)` once per `key`, and builds from the latest
    /// value it gave.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<K, T, F>(
        key: K,
        make: impl Fn(&K, StreamSink<T>) -> F + 'static,
        build: impl Fn(&BuildContext, AsyncSnapshot<T>) -> Box<dyn Widget> + 'static,
    ) -> Component
    where
        K: Clone + PartialEq + 'static,
        T: Clone + MaybeSend + 'static,
        F: Future<Output = ()> + MaybeSend + 'static,
    {
        component(move |cx| {
            let snapshot = cx.use_stream(key.clone(), |sink| make(&key, sink));
            build(cx, snapshot)
        })
    }
}

/// Builds from a [`ValueNotifier`]'s value, and again whenever it changes — the reference's
/// `ValueListenableBuilder`.
///
/// ```
/// use frus_widgets::{text, ValueListenableBuilder, ValueNotifier};
///
/// let count = ValueNotifier::new(0);
/// let label = ValueListenableBuilder::new(count.clone(), |_, n: &i32| Box::new(text(format!("{n}"))));
/// count.set(1); // the label shows 1 at the next frame
/// # let _ = label;
/// ```
pub struct ValueListenableBuilder;

impl ValueListenableBuilder {
    /// A component that builds from `notifier`'s value.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<T: 'static>(
        notifier: ValueNotifier<T>,
        build: impl Fn(&BuildContext, &T) -> Box<dyn Widget> + 'static,
    ) -> Component {
        component(move |cx| notifier.with(|value| build(cx, value)))
    }
}

/// Builds again whenever a [`Listenable`] says it changed — the reference's
/// `ListenableBuilder`.
///
/// A notifier asks for a rebuild when it tells its listeners, so what is built here is always
/// built from the notifier's present state: the notifier is kept so that the builder can read
/// it, and to say in the tree what this part of it follows.
pub struct ListenableBuilder;

impl ListenableBuilder {
    /// A component that builds from `listenable`, whenever it changes.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<L: Listenable + 'static>(
        listenable: L,
        build: impl Fn(&BuildContext, &L) -> Box<dyn Widget> + 'static,
    ) -> Component {
        component(move |cx| build(cx, &listenable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// **With no executor registered, the work still runs** — on a thread of its own — and
    /// its value arrives: the widget layer on its own, in a host that registered nothing.
    #[test]
    fn with_no_executor_the_work_runs_on_a_thread() {
        let before = task_arrivals();
        let slot: Slot<i32> = Arc::new(Mutex::new(AsyncSnapshot::waiting()));
        let written = slot.clone();
        spawn(Box::pin(async move {
            if let Ok(mut snapshot) = written.lock() {
                snapshot.state = ConnectionState::Done;
                snapshot.data = Some(7);
            }
            arrived();
        }));
        let start = Instant::now();
        while task_arrivals() == before && start.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(slot.lock().expect("the slot").data, Some(7));
    }

    /// **A snapshot starts waiting, with nothing**, and says when it has a value.
    #[test]
    fn a_snapshot_starts_waiting() {
        let waiting = AsyncSnapshot::<u8>::waiting();
        assert_eq!(waiting.state, ConnectionState::Waiting);
        assert!(!waiting.has_data());
        let done = AsyncSnapshot {
            state: ConnectionState::Done,
            data: Some(1u8),
        };
        assert!(done.has_data());
    }
}
