//! Work that waits, on the interface's own thread (milestone 714): [`spawn_local`],
//! [`StateHandle::spawn`] and [`StateContext::async_callback`].
//!
//! A handler that sends a form wants to say, in one place, what happens: the button turns
//! busy, the request goes out, the answer is shown. [`spawn_async`](crate::host::spawn_async) runs
//! its work on the shell's executor, off this thread, so the work must be `Send` and cannot
//! hold the state it would change; the change has to wait in a second closure. A task here
//! runs on the interface's thread instead, between frames: it may hold a [`StateHandle`],
//! change the state, `await` a request — which goes on waiting off this thread, on the
//! shell's executor or the browser's loop — and change the state again when it comes back.
//!
//! A task is polled when it is woken: whatever woke it, from whatever thread, asks the shell
//! for a frame ([`set_local_wake`]), and the frame polls every task woken since the last
//! ([`run_local_tasks`]) before it builds. A task owned by a state is dropped, unfinished,
//! once that state has left the tree: what it was waiting for no longer has anyone to tell.

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake, Waker};
use std::thread::ThreadId;

use crate::component::{State, StateContext, StateHandle};
use crate::Callback;

type LocalFuture = Pin<Box<dyn Future<Output = ()>>>;

/// One task: the future, and, when a state owns it, whether that state is still there.
struct Entry {
    future: LocalFuture,
    alive: Option<Box<dyn Fn() -> bool>>,
}

impl Entry {
    fn alive(&self) -> bool {
        self.alive.as_ref().is_none_or(|alive| alive())
    }
}

thread_local! {
    /// The tasks of this thread — the interface's — by number.
    static TASKS: RefCell<HashMap<u64, Entry>> = RefCell::new(HashMap::new());
}

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The tasks woken since their thread last ran them, each with that thread. Shared, since a
/// waker may be called from any thread — the one a request was waited on, a timer's — and
/// tagged, since each thread runs only its own (an application has one interface thread; a
/// test run has many).
static WOKEN: Mutex<Vec<(ThreadId, u64)>> = Mutex::new(Vec::new());

/// What asks the shell for a frame.
type WakeFn = Arc<dyn Fn() + Send + Sync>;
static WAKE: Mutex<Option<WakeFn>> = Mutex::new(None);

/// Tells the widget layer how to ask for a frame from any thread, so that a task woken there
/// runs. The framework's shell sets it when its window is made; without one, a woken task
/// waits for the next frame something else brings.
pub fn set_local_wake(wake: impl Fn() + Send + Sync + 'static) {
    if let Ok(mut slot) = WAKE.lock() {
        *slot = Some(Arc::new(wake));
    }
}

fn wake(thread: ThreadId, id: u64) {
    if let Ok(mut woken) = WOKEN.lock() {
        if !woken.contains(&(thread, id)) {
            woken.push((thread, id));
        }
    }
    let wake = WAKE.lock().ok().and_then(|slot| slot.clone());
    if let Some(wake) = wake {
        wake();
    }
}

/// Wakes one task, of one thread.
struct TaskWaker(ThreadId, u64);

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        wake(self.0, self.1);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        wake(self.0, self.1);
    }
}

fn insert(future: LocalFuture, alive: Option<Box<dyn Fn() -> bool>>) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    TASKS.with(|tasks| tasks.borrow_mut().insert(id, Entry { future, alive }));
    // Its first poll is the next frame's: a task started from a handler runs once the
    // handler has returned, never in the middle of it.
    wake(std::thread::current().id(), id);
}

/// Runs `future` on the interface's thread, between frames, to its end.
///
/// It need not be `Send`: it may hold a [`StateHandle`], a controller, anything the
/// interface keeps. What it awaits waits elsewhere — a request on the shell's executor or
/// the browser's loop — and the task goes on when that is done. It must not block: a slow
/// computation belongs to [`spawn`](crate::host::spawn), whose work runs on another thread.
///
/// Nothing owns it, so it runs even after the widget that started it has gone; a task that
/// changes a state is better started with [`StateHandle::spawn`], which ends with the state.
pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    insert(Box::pin(future), None);
}

/// Polls every task woken since the last time, and drops the tasks whose state has left the
/// tree. The shell calls it at the start of each frame, before it builds: a task that changes
/// a state asks for the rebuild that frame then makes. Answers whether any task ran.
pub fn run_local_tasks() -> bool {
    // The tasks whose state has gone end here, woken or not. Taken out first and dropped
    // after: dropping a future runs its destructors, which may start a task of their own.
    let gone: Vec<Entry> = TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let ids: Vec<u64> = tasks
            .iter()
            .filter(|(_, entry)| !entry.alive())
            .map(|(id, _)| *id)
            .collect();
        ids.into_iter().filter_map(|id| tasks.remove(&id)).collect()
    });
    drop(gone);

    let thread = std::thread::current().id();
    let woken: Vec<u64> = WOKEN
        .lock()
        .map(|mut woken| {
            let mine = woken
                .iter()
                .filter(|(t, _)| *t == thread)
                .map(|(_, id)| *id)
                .collect();
            woken.retain(|(t, _)| *t != thread);
            mine
        })
        .unwrap_or_default();
    let mut ran = false;
    for id in woken {
        // Out of the table while it runs: a task may start another, or change a state whose
        // rebuild starts one.
        let Some(mut entry) = TASKS.with(|tasks| tasks.borrow_mut().remove(&id)) else {
            continue;
        };
        ran = true;
        let waker = Waker::from(Arc::new(TaskWaker(thread, id)));
        let mut cx = Context::from_waker(&waker);
        if entry.future.as_mut().poll(&mut cx).is_pending() {
            TASKS.with(|tasks| tasks.borrow_mut().insert(id, entry));
        }
    }
    ran
}

/// How many tasks are waiting or running on this thread.
pub fn local_tasks() -> usize {
    TASKS.with(|tasks| tasks.borrow().len())
}

impl<S: State> StateHandle<S> {
    /// Runs `future` on the interface's thread, owned by this state: dropped, unfinished,
    /// once the state has left the tree. See [`spawn_local`].
    ///
    /// A page that loads what it shows once, when it is first built:
    ///
    /// ```no_run
    /// use frus_widgets::{State, StateContext, StatefulWidget, Text, Widget};
    ///
    /// # async fn load_posts() -> Vec<String> { Vec::new() }
    /// struct Posts;
    ///
    /// #[derive(Default)]
    /// struct PostsState {
    ///     posts: Vec<String>,
    ///     loading: bool,
    /// }
    ///
    /// impl StatefulWidget for Posts {
    ///     type State = PostsState;
    ///     fn create_state(&self) -> PostsState {
    ///         PostsState::default()
    ///     }
    /// }
    ///
    /// impl State for PostsState {
    ///     type Widget = Posts;
    ///     fn init_state(&mut self, cx: &StateContext<Self>) {
    ///         // It first runs at the next frame, once `init_state` has returned.
    ///         let state = cx.handle();
    ///         state.clone().spawn(async move {
    ///             state.set_state(|s| s.loading = true);
    ///             let posts = load_posts().await;
    ///             state.set_state(|s| {
    ///                 s.loading = false;
    ///                 s.posts = posts;
    ///             });
    ///         });
    ///     }
    ///     fn build(&self, _cx: &StateContext<Self>) -> Box<dyn Widget> {
    ///         Box::new(Text::new(format!("{} posts", self.posts.len())))
    ///     }
    /// }
    /// ```
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static) {
        let handle = self.clone();
        insert(
            Box::pin(future),
            Some(Box::new(move || handle.is_mounted())),
        );
    }

    /// An `async` handler, for anything that takes a [`Callback`]: each time it is called,
    /// `task` is given this state's handle and its future runs, owned by the state — see
    /// [`StateHandle::spawn`].
    pub fn async_callback<F>(&self, task: impl Fn(StateHandle<S>) -> F + 'static) -> Callback
    where
        F: Future<Output = ()> + 'static,
    {
        let handle = self.clone();
        Callback::new(move || handle.spawn(task(handle.clone())))
    }
}

impl<S: State> StateContext<'_, '_, S> {
    /// An `async` handler that changes this state between the things it awaits:
    ///
    /// ```no_run
    /// use frus_widgets::{column, Button, State, StateContext, StatefulWidget, Text, Widget};
    ///
    /// # async fn fetch_quote() -> String { String::new() }
    /// struct Quote;
    ///
    /// #[derive(Default)]
    /// struct QuoteState {
    ///     text: String,
    ///     loading: bool,
    /// }
    ///
    /// impl StatefulWidget for Quote {
    ///     type State = QuoteState;
    ///     fn create_state(&self) -> QuoteState {
    ///         QuoteState::default()
    ///     }
    /// }
    ///
    /// impl State for QuoteState {
    ///     type Widget = Quote;
    ///     fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
    ///         let load = cx.async_callback(|state| async move {
    ///             state.set_state(|s| s.loading = true);
    ///             let text = fetch_quote().await;
    ///             state.set_state(|s| {
    ///                 s.loading = false;
    ///                 s.text = text;
    ///             });
    ///         });
    ///         Box::new(column![
    ///             Text::new(if self.loading { "Loading…" } else { self.text.as_str() }),
    ///             Button::new("New quote").enabled(!self.loading).on_press(load),
    ///         ])
    ///     }
    /// }
    /// ```
    ///
    /// Each press starts a task, owned by the state ([`StateHandle::spawn`]): it runs on the
    /// interface's thread, so it holds the state's handle and need not be `Send`, and it is
    /// dropped if the state leaves the tree before it is done.
    pub fn async_callback<F>(&self, task: impl Fn(StateHandle<S>) -> F + 'static) -> Callback
    where
        F: Future<Output = ()> + 'static,
    {
        self.handle().async_callback(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::AtomicUsize;
    use std::task::Poll;
    use std::time::{Duration, Instant};

    /// A value another thread hands over: pending until it is there, and then woken.
    #[derive(Clone, Default)]
    struct Handover(Arc<Mutex<(Option<u32>, Option<Waker>)>>);

    impl Handover {
        fn give(&self, value: u32) {
            let waker = {
                let mut slot = self.0.lock().unwrap();
                slot.0 = Some(value);
                slot.1.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }
    }

    impl Future for Handover {
        type Output = u32;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
            let mut slot = self.0.lock().unwrap();
            match slot.0 {
                Some(value) => Poll::Ready(value),
                None => {
                    slot.1 = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }

    /// Runs this thread's tasks until `done`, or fails after two seconds.
    fn run_until(done: impl Fn() -> bool) {
        let end = Instant::now() + Duration::from_secs(2);
        while !done() {
            assert!(Instant::now() < end, "the task never finished");
            run_local_tasks();
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn a_task_runs_at_the_next_frame_not_in_the_handler() {
        let ran = Rc::new(Cell::new(0));
        let r = ran.clone();
        spawn_local(async move { r.set(r.get() + 1) });
        assert_eq!(ran.get(), 0, "spawning does not run it");
        assert!(run_local_tasks());
        assert_eq!(ran.get(), 1);
        assert_eq!(local_tasks(), 0, "a finished task is gone");
        assert!(!run_local_tasks(), "and nothing is left to run");
    }

    #[test]
    fn a_task_goes_on_when_what_it_awaits_is_given_from_another_thread() {
        let seen = Rc::new(Cell::new(None));
        let (s, handover) = (seen.clone(), Handover::default());
        let given = handover.clone();
        // Not `Send`: an `Rc` is held across the await.
        spawn_local(async move {
            let value = handover.await;
            s.set(Some(value));
        });
        run_local_tasks();
        assert_eq!(seen.get(), None, "it waits");
        assert_eq!(local_tasks(), 1);
        std::thread::spawn(move || given.give(7));
        run_until(|| seen.get().is_some());
        assert_eq!(seen.get(), Some(7));
        assert_eq!(local_tasks(), 0);
    }

    #[test]
    fn a_task_waiting_is_not_polled_again_until_it_is_woken() {
        let polls = Rc::new(Cell::new(0));
        let p = polls.clone();
        let handover = Handover::default();
        let given = handover.clone();
        spawn_local(async move {
            std::future::poll_fn(|_| {
                p.set(p.get() + 1);
                Poll::Ready(())
            })
            .await;
            handover.await;
        });
        run_local_tasks();
        run_local_tasks();
        run_local_tasks();
        assert_eq!(polls.get(), 1, "a frame does not poll a task nobody woke");
        given.give(1);
        run_local_tasks();
        assert_eq!(local_tasks(), 0);
    }

    #[test]
    fn another_threads_frame_leaves_this_threads_wakes() {
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        spawn_local(async move { r.set(true) });
        std::thread::spawn(run_local_tasks).join().unwrap();
        assert!(run_local_tasks(), "the wake is still this thread's");
        assert!(ran.get());
    }

    #[test]
    fn a_woken_task_asks_for_a_frame() {
        static WAKES: AtomicUsize = AtomicUsize::new(0);
        set_local_wake(|| {
            WAKES.fetch_add(1, Ordering::SeqCst);
        });
        let before = WAKES.load(Ordering::SeqCst);
        let handover = Handover::default();
        let given = handover.clone();
        spawn_local(async move {
            handover.await;
        });
        assert!(
            WAKES.load(Ordering::SeqCst) > before,
            "a new task asks for its first frame"
        );
        run_local_tasks();
        let waiting = WAKES.load(Ordering::SeqCst);
        std::thread::spawn(move || given.give(1)).join().unwrap();
        assert!(
            WAKES.load(Ordering::SeqCst) > waiting,
            "and so does one woken elsewhere"
        );
        run_local_tasks();
        assert_eq!(local_tasks(), 0);
    }
}
