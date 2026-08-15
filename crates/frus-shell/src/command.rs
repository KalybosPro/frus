//! [`Command`]: the **effects** `Application::update` returns.
//!
//! A command describes work to be done **outside** the `update` cycle — a file
//! write, a load, a network `fetch`, a long computation — whose result comes back as
//! a **message** fed into `update`. There are three shapes:
//!
//! - **synchronous** ([`Command::perform`] / [`Command::run`]): a closure that
//!   blocks. It gets a background thread natively, or a microtask on the Web.
//! - **asynchronous** ([`Command::perform_async`] / [`Command::run_async`]): a
//!   **future** that can `await`. Natively it goes to the shell's executor, which
//!   has a reactor, so a future may wait on a timer or on I/O without holding a
//!   thread. On the **Web**, which is single-threaded, the browser drives it
//!   (`spawn_local`) and always did.
//! - **delayed** ([`Command::after`]): a message on a real timer.
//!
//! The distinction that matters is the first against the second, and it is about
//! **what the work does while it is not finished**. Something that blocks — a
//! synchronous file read, a CPU-bound computation — must be a `perform`, because
//! occupying a thread is exactly what a thread is for. Something that *waits* should
//! be a `perform_async`, because waiting no longer costs a thread. Putting a blocking
//! call inside `perform_async` is the one mistake worth naming here: it parks one of
//! the executor's few workers and starves every other effect.
//!
//! Until milestone 303 the asynchronous side was a thread and a `block_on` with no
//! reactor underneath it — `frus-shell`'s own `runtime` module records what that
//! could and could not do.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// A command taken apart, for the shell to run. A struct rather than a tuple: there
/// are four kinds of effect now, and a four-tuple at a call site says nothing about
/// which is which.
pub(crate) struct Parts<Msg> {
    pub(crate) tasks: Vec<Task<Msg>>,
    pub(crate) async_tasks: Vec<AsyncTask<Msg>>,
    pub(crate) timers: Vec<(Duration, Msg)>,
    pub(crate) focus: Vec<u64>,
}

/// A **synchronous** task: work that may produce a message.
type Task<Msg> = Box<dyn FnOnce() -> Option<Msg> + Send + 'static>;

/// An **asynchronous** task: a future that may produce a message.
///
/// **Natively** it crosses a thread (`block_on`), hence the `Send` bound. On the
/// **Web**, which is single-threaded, the browser's futures (`JsFuture`/`fetch`) are
/// **not** `Send` and do not need to be — hence the relaxed bound.
#[cfg(not(web))]
type AsyncTask<Msg> = Pin<Box<dyn Future<Output = Option<Msg>> + Send + 'static>>;
#[cfg(web)]
type AsyncTask<Msg> = Pin<Box<dyn Future<Output = Option<Msg>> + 'static>>;

/// A batch of effects to run, possibly empty: background **tasks**, synchronous or
/// asynchronous, and **focus** requests addressed by widget key.
pub struct Command<Msg> {
    tasks: Vec<Task<Msg>>,
    async_tasks: Vec<AsyncTask<Msg>>,
    /// Messages to deliver after a delay. Kept as data rather than as a future, so
    /// that the one piece of platform knowledge — how this machine waits — stays in
    /// the shell with all the other platform knowledge.
    timers: Vec<(Duration, Msg)>,
    /// The keys of widgets to focus — the key's hash, as in [`crate::Subscription`]
    /// and the widgets' `keyed(...)`. The shell resolves them after the next build.
    focus: Vec<u64>,
}

/// A focus key's hash — **identical** to the hash the widgets' `keyed(key, …)` uses,
/// so that `Command::focus(k)` targets the `keyed(k, …)` widget.
fn focus_key(key: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

impl<Msg: Send + 'static> Command<Msg> {
    /// No effect at all.
    pub fn none() -> Self {
        Self {
            tasks: Vec::new(),
            async_tasks: Vec::new(),
            timers: Vec::new(),
            focus: Vec::new(),
        }
    }

    /// Groups several commands into one.
    pub fn batch(commands: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let mut tasks = Vec::new();
        let mut async_tasks = Vec::new();
        let mut timers = Vec::new();
        let mut focus = Vec::new();
        for command in commands {
            tasks.extend(command.tasks);
            async_tasks.extend(command.async_tasks);
            timers.extend(command.timers);
            focus.extend(command.focus);
        }
        Self {
            tasks,
            async_tasks,
            timers,
            focus,
        }
    }

    /// Delivers `message` once, after `delay`.
    ///
    /// This waits without occupying anything: natively it is a task on the shell's
    /// executor parked on the reactor's timer wheel, and on the Web a `setTimeout`.
    /// A hundred pending timers are a hundred entries in a queue, not a hundred
    /// threads.
    ///
    /// For a message that repeats, declare a [`Subscription::every`](crate::Subscription::every)
    /// instead — a subscription is a function of the state and stops when the state
    /// says so, where a chain of `after`s has to be stopped by hand.
    pub fn after(delay: Duration, message: Msg) -> Self {
        Self {
            timers: vec![(delay, message)],
            ..Self::none()
        }
    }

    /// Runs a **synchronous** task in the background; its result becomes a message.
    pub fn perform(task: impl FnOnce() -> Msg + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(move || Some(task()))],
            ..Self::none()
        }
    }

    /// Runs a **synchronous** side effect; it may return a message (`None` for none).
    pub fn run(task: impl FnOnce() -> Option<Msg> + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(task)],
            ..Self::none()
        }
    }

    /// Runs an asynchronous **future**; its value becomes a message.
    ///
    /// On the **Web**, which is single-threaded, the browser drives the future
    /// (`spawn_local`), so a real `fetch` can `await` without blocking. **Natively**
    /// it runs on the shell's executor (the `runtime` module), which has a reactor: a
    /// future may wait on a timer or on a socket and costs nothing while it does.
    ///
    /// Do not put a **blocking** call in here — that is what [`Command::perform`] is
    /// for. The executor has four worker threads, and a blocking call holds one of
    /// them for its whole duration.
    #[cfg(not(web))]
    pub fn perform_async<F>(future: F) -> Self
    where
        F: Future<Output = Msg> + Send + 'static,
    {
        Self {
            async_tasks: vec![Box::pin(async move { Some(future.await) })],
            ..Self::none()
        }
    }

    /// Runs an asynchronous **future**; its value becomes a message. See the native
    /// version for the full semantics; the `Send` bounds are relaxed on the Web.
    #[cfg(web)]
    pub fn perform_async<F>(future: F) -> Self
    where
        F: Future<Output = Msg> + 'static,
    {
        Self {
            async_tasks: vec![Box::pin(async move { Some(future.await) })],
            ..Self::none()
        }
    }

    /// Runs an asynchronous **future** for its side effect; it may return a message
    /// (`None` for none). The asynchronous counterpart of [`Command::run`].
    #[cfg(not(web))]
    pub fn run_async<F>(future: F) -> Self
    where
        F: Future<Output = Option<Msg>> + Send + 'static,
    {
        Self {
            async_tasks: vec![Box::pin(future)],
            ..Self::none()
        }
    }

    /// Runs an asynchronous **future** for its side effect (`None` for no message).
    /// The Web version, with the `Send` bound relaxed.
    #[cfg(web)]
    pub fn run_async<F>(future: F) -> Self
    where
        F: Future<Output = Option<Msg>> + 'static,
    {
        Self {
            async_tasks: vec![Box::pin(future)],
            ..Self::none()
        }
    }

    /// Requests **focus** for the widget carrying `key` — the field wrapped in
    /// `keyed(key, …)`. The shell resolves it after the view is next built; it is
    /// typically returned when a form submission fails, to jump to the first invalid
    /// field (`Form::first_invalid`).
    pub fn focus(key: impl std::hash::Hash) -> Self {
        Self {
            focus: vec![focus_key(key)],
            ..Self::none()
        }
    }

    /// `true` when the command has neither an effect nor a focus request.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
            && self.async_tasks.is_empty()
            && self.timers.is_empty()
            && self.focus.is_empty()
    }

    /// Takes the command apart for the framework to run.
    pub(crate) fn into_parts(self) -> Parts<Msg> {
        Parts {
            tasks: self.tasks,
            async_tasks: self.async_tasks,
            timers: self.timers,
            focus: self.focus,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_empty() {
        assert!(Command::<u32>::none().is_empty());
    }

    #[test]
    fn perform_yields_a_message() {
        let command = Command::perform(|| 42u32);
        let tasks = command.into_parts().tasks;
        assert_eq!(tasks.len(), 1);
        let produced = (tasks.into_iter().next().unwrap())();
        assert_eq!(produced, Some(42));
    }

    #[test]
    fn run_may_produce_nothing() {
        let command = Command::run(|| -> Option<u32> { None });
        let tasks = command.into_parts().tasks;
        assert_eq!((tasks.into_iter().next().unwrap())(), None);
    }

    #[test]
    fn batch_flattens_and_drops_empties() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::none(),
            Command::run(|| Some(2u32)),
        ]);
        assert_eq!(command.into_parts().tasks.len(), 2);
    }

    #[test]
    fn focus_carries_a_key_and_no_task() {
        // A focus command carries no task, yet it is not "empty".
        let f = Command::<u32>::focus("email");
        assert!(!f.is_empty());
        let parts = f.into_parts();
        assert!(parts.tasks.is_empty());
        assert!(parts.async_tasks.is_empty());
        assert_eq!(parts.focus, vec![focus_key("email")]);
    }

    #[cfg(not(web))]
    #[test]
    fn perform_async_yields_a_message() {
        // The future is driven to completion (`block_on`) and its value becomes a message.
        let command = Command::perform_async(async { 7u32 });
        let mut parts = command.into_parts();
        assert!(parts.tasks.is_empty());
        assert_eq!(parts.async_tasks.len(), 1);
        let produced = pollster::block_on(parts.async_tasks.remove(0));
        assert_eq!(produced, Some(7));
    }

    #[cfg(not(web))]
    #[test]
    fn run_async_may_produce_nothing() {
        let command = Command::run_async(async { None::<u32> });
        let mut parts = command.into_parts();
        assert_eq!(pollster::block_on(parts.async_tasks.remove(0)), None);
    }

    #[cfg(not(web))]
    #[test]
    fn batch_combines_sync_and_async_tasks() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::perform_async(async { 2u32 }),
        ]);
        let parts = command.into_parts();
        assert_eq!(parts.tasks.len(), 1);
        assert_eq!(parts.async_tasks.len(), 1);
    }

    #[test]
    fn after_carries_its_delay_and_is_not_empty() {
        let command = Command::after(Duration::from_millis(250), 9u32);
        assert!(!command.is_empty(), "a pending timer is an effect");
        let parts = command.into_parts();
        assert!(parts.tasks.is_empty() && parts.async_tasks.is_empty());
        assert_eq!(parts.timers, vec![(Duration::from_millis(250), 9u32)]);
    }

    #[test]
    fn batch_carries_timers_through() {
        let command = Command::batch([
            Command::after(Duration::from_millis(10), 1u32),
            Command::perform(|| 2u32),
            Command::after(Duration::from_millis(20), 3u32),
        ]);
        let parts = command.into_parts();
        assert_eq!(parts.tasks.len(), 1);
        assert_eq!(parts.timers.len(), 2, "a batch must not swallow a timer");
    }
}
