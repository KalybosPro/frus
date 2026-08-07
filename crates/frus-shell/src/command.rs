//! [`Command`]: the **effects** `Application::update` returns.
//!
//! A command describes work to be done **outside** the `update` cycle — a file
//! write, a load, a network `fetch`, a long computation — whose result comes back as
//! a **message** fed into `update`. There are two shapes:
//!
//! - **synchronous** ([`Command::perform`] / [`Command::run`]): a closure run on a
//!   background thread natively, or as a microtask on the Web.
//! - **asynchronous** ([`Command::perform_async`] / [`Command::run_async`]): a
//!   **future** that can `await`. On the **Web**, which is single-threaded, the
//!   browser drives it (`spawn_local`), so a real `fetch` can be awaited.
//!   **Natively** it is driven to completion on a thread (`block_on`).

use std::future::Future;
use std::pin::Pin;

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
            focus: Vec::new(),
        }
    }

    /// Groups several commands into one.
    pub fn batch(commands: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let mut tasks = Vec::new();
        let mut async_tasks = Vec::new();
        let mut focus = Vec::new();
        for command in commands {
            tasks.extend(command.tasks);
            async_tasks.extend(command.async_tasks);
            focus.extend(command.focus);
        }
        Self {
            tasks,
            async_tasks,
            focus,
        }
    }

    /// Runs a **synchronous** task in the background; its result becomes a message.
    pub fn perform(task: impl FnOnce() -> Msg + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(move || Some(task()))],
            async_tasks: Vec::new(),
            focus: Vec::new(),
        }
    }

    /// Runs a **synchronous** side effect; it may return a message (`None` for none).
    pub fn run(task: impl FnOnce() -> Option<Msg> + Send + 'static) -> Self {
        Self {
            tasks: vec![Box::new(task)],
            async_tasks: Vec::new(),
            focus: Vec::new(),
        }
    }

    /// Runs an asynchronous **future**; its value becomes a message.
    ///
    /// On the **Web**, which is single-threaded, the browser drives the future
    /// (`spawn_local`), so a real `fetch` can `await` without blocking. **Natively** it
    /// is driven to completion on a background thread (`block_on`), which suits a
    /// **self-contained** future — a computation, a channel, a driven timer; real
    /// network I/O wants the application's own async runtime. See the module's
    /// platform note.
    #[cfg(not(web))]
    pub fn perform_async<F>(future: F) -> Self
    where
        F: Future<Output = Msg> + Send + 'static,
    {
        Self {
            tasks: Vec::new(),
            async_tasks: vec![Box::pin(async move { Some(future.await) })],
            focus: Vec::new(),
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
            tasks: Vec::new(),
            async_tasks: vec![Box::pin(async move { Some(future.await) })],
            focus: Vec::new(),
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
            tasks: Vec::new(),
            async_tasks: vec![Box::pin(future)],
            focus: Vec::new(),
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
            tasks: Vec::new(),
            async_tasks: vec![Box::pin(future)],
            focus: Vec::new(),
        }
    }

    /// Requests **focus** for the widget carrying `key` — the field wrapped in
    /// `keyed(key, …)`. The shell resolves it after the view is next built; it is
    /// typically returned when a form submission fails, to jump to the first invalid
    /// field (`Form::first_invalid`).
    pub fn focus(key: impl std::hash::Hash) -> Self {
        Self {
            tasks: Vec::new(),
            async_tasks: Vec::new(),
            focus: vec![focus_key(key)],
        }
    }

    /// `true` when the command has neither an effect nor a focus request.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty() && self.async_tasks.is_empty() && self.focus.is_empty()
    }

    /// Takes out the synchronous tasks, the asynchronous tasks and the focus
    /// requests, for the framework to run.
    pub(crate) fn into_parts(self) -> (Vec<Task<Msg>>, Vec<AsyncTask<Msg>>, Vec<u64>) {
        (self.tasks, self.async_tasks, self.focus)
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
        let (tasks, _, _) = command.into_parts();
        assert_eq!(tasks.len(), 1);
        let produced = (tasks.into_iter().next().unwrap())();
        assert_eq!(produced, Some(42));
    }

    #[test]
    fn run_may_produce_nothing() {
        let command = Command::run(|| -> Option<u32> { None });
        let (tasks, _, _) = command.into_parts();
        assert_eq!((tasks.into_iter().next().unwrap())(), None);
    }

    #[test]
    fn batch_flattens_and_drops_empties() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::none(),
            Command::run(|| Some(2u32)),
        ]);
        assert_eq!(command.into_parts().0.len(), 2);
    }

    #[test]
    fn focus_carries_a_key_and_no_task() {
        // A focus command carries no task, yet it is not "empty".
        let f = Command::<u32>::focus("email");
        assert!(!f.is_empty());
        let (tasks, asyncs, focus) = f.into_parts();
        assert!(tasks.is_empty());
        assert!(asyncs.is_empty());
        assert_eq!(focus, vec![focus_key("email")]);
    }

    #[cfg(not(web))]
    #[test]
    fn perform_async_yields_a_message() {
        // The future is driven to completion (`block_on`) and its value becomes a message.
        let command = Command::perform_async(async { 7u32 });
        let (tasks, mut asyncs, _) = command.into_parts();
        assert!(tasks.is_empty());
        assert_eq!(asyncs.len(), 1);
        let produced = pollster::block_on(asyncs.remove(0));
        assert_eq!(produced, Some(7));
    }

    #[cfg(not(web))]
    #[test]
    fn run_async_may_produce_nothing() {
        let command = Command::run_async(async { None::<u32> });
        let (_, mut asyncs, _) = command.into_parts();
        assert_eq!(pollster::block_on(asyncs.remove(0)), None);
    }

    #[cfg(not(web))]
    #[test]
    fn batch_combines_sync_and_async_tasks() {
        let command = Command::batch([
            Command::perform(|| 1u32),
            Command::perform_async(async { 2u32 }),
        ]);
        let (tasks, asyncs, _) = command.into_parts();
        assert_eq!(tasks.len(), 1);
        assert_eq!(asyncs.len(), 1);
    }
}
