//! The **async runtime**: one executor for the whole application, and the reactor
//! that makes waiting on something other than the CPU possible.
//!
//! # What this replaces
//!
//! Until now every asynchronous effect got an OS thread of its own and was driven
//! with `pollster::block_on`. That has two costs, and the second is the serious one:
//!
//! 1. Ten concurrent requests were ten threads, each with its own stack.
//! 2. `block_on` parks the thread on the waker and **nothing ever wakes it** unless
//!    the future wakes itself. There was no reactor, so a future waiting on a timer
//!    or on a socket becoming readable waited for ever. Only self-contained futures
//!    and futures that block internally worked — which is why the `fetch` helper had
//!    to reach for a *blocking* HTTP client and rely on having a thread to waste.
//!
//! So asynchrony was available in the type system and absent underneath it.
//!
//! # What this is
//!
//! A shared [`async_executor::Executor`] on a small pool of worker threads, each
//! running the executor **inside `async_io::block_on`** — which is the piece that
//! matters: it installs `async-io`'s reactor on the thread, so timers fire and I/O
//! readiness wakes the tasks waiting on it.
//!
//! The pool is started on first use and lives for the process. An application that
//! never runs an asynchronous effect never starts it.
//!
//! # What this is not
//!
//! It is not a general-purpose runtime, and it is not tokio. A future that requires
//! tokio's reactor — anything built on `tokio::net`, and therefore most of the HTTP
//! ecosystem — will not run here, for exactly the reason described above: it needs
//! *its* reactor installed on the thread, not this one. Such an application should
//! start its own runtime and hand messages back through
//! [`Command::run_async`](crate::Command::run_async). Letting frus be handed a
//! runtime rather than owning one is worth doing and is not done yet.
//!
//! # The Web
//!
//! None of this exists there. The browser is the executor, futures go to
//! `wasm_bindgen_futures::spawn_local`, and it was always genuinely asynchronous —
//! the Web was ahead of the native side, not behind it.

use std::future::Future;
use std::sync::OnceLock;

use async_executor::{Executor, Task};

/// How many worker threads the pool gets.
///
/// Interface work is overwhelmingly *waiting* — on a response, on a timer, on a file
/// — rather than computing, and waiting costs a task, not a thread. Four is enough to
/// keep a blocking-ish effect from starving the others, and the point of having a
/// reactor at all is that the fifth concurrent request does not need a fifth thread.
const WORKERS: usize = 4;

/// The executor, leaked so that a worker thread can hold it for the process's life
/// without the reference counting that would otherwise be needed to prove it.
static EXECUTOR: OnceLock<&'static Executor<'static>> = OnceLock::new();

/// The process-wide executor, starting its worker threads on first use.
fn executor() -> &'static Executor<'static> {
    EXECUTOR.get_or_init(|| {
        let shared: &'static Executor<'static> = Box::leak(Box::new(Executor::new()));
        for index in 0..WORKERS {
            std::thread::Builder::new()
                .name(format!("frus-async-{index}"))
                // `async_io::block_on`, and not `futures_lite::future::block_on`:
                // this is the line that installs the reactor on the thread. Swapping
                // it for the plain one compiles, passes any test that only awaits
                // ready futures, and hangs the first time anything waits on a timer.
                .spawn(move || async_io::block_on(shared.run(std::future::pending::<()>())))
                .expect("spawning a frus async worker");
        }
        shared
    })
}

/// Runs `future` on the shared executor, returning a handle.
///
/// Dropping the handle **cancels** the task. That is what makes a subscription
/// stoppable: the shell keeps the handle for as long as the application declares the
/// subscription, and lets go of it when it does not. An effect that must outlive its
/// handle calls [`Task::detach`].
pub(crate) fn spawn<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> Task<T> {
    executor().spawn(future)
}

/// Waits for `duration`, then resolves.
///
/// This is the smallest thing that proves the reactor is really there: before it,
/// nothing in the framework could wait without occupying a thread.
///
/// Deliberately **not public**. A portable application reaches for
/// [`Command::after`](crate::Command::after), which works on the Web too; a native-only
/// `sleep` in a cross-platform framework is an invitation to write code that will not
/// compile for one of its targets.
pub(crate) async fn sleep(duration: std::time::Duration) {
    async_io::Timer::after(duration).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_task_runs_on_the_pool_and_gives_its_value_back() {
        let task = spawn(async { 6 * 7 });
        assert_eq!(futures_lite::future::block_on(task), 42);
    }

    /// The point of the whole module. A future that waits on a timer used to hang
    /// for ever, because `pollster::block_on` parks the thread and nothing was there
    /// to wake it. If this test times out, the reactor is not installed.
    #[test]
    fn a_task_that_waits_on_a_timer_actually_wakes_up() {
        let started = Instant::now();
        let task = spawn(async {
            sleep(Duration::from_millis(120)).await;
            "awake"
        });
        assert_eq!(futures_lite::future::block_on(task), "awake");
        assert!(
            started.elapsed() >= Duration::from_millis(100),
            "it returned in {:?}, which means it never actually waited",
            started.elapsed()
        );
    }

    /// Many waiting tasks, far more than there are worker threads. Under the old
    /// thread-per-future model this was 64 OS threads; here it is 64 tasks on four.
    #[test]
    fn more_waiting_tasks_than_there_are_threads() {
        let started = Instant::now();
        let tasks: Vec<_> = (0..64)
            .map(|i| {
                spawn(async move {
                    sleep(Duration::from_millis(60)).await;
                    i
                })
            })
            .collect();
        let sum: usize = tasks.into_iter().map(futures_lite::future::block_on).sum();
        assert_eq!(sum, (0..64).sum::<usize>());
        // Four threads, sixty-four waits of 60 ms. Serialised that is most of a
        // second; concurrent it is one wait. The margin is wide because a loaded CI
        // machine is slow, not because the difference is subtle.
        assert!(
            started.elapsed() < Duration::from_millis(600),
            "{:?} — the waits did not overlap",
            started.elapsed()
        );
    }

    /// Dropping the handle cancels the task, which is how a subscription stops.
    #[test]
    fn dropping_a_handle_cancels_the_task() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let ran = Arc::new(AtomicBool::new(false));
        let flag = ran.clone();
        let task = spawn(async move {
            sleep(Duration::from_millis(250)).await;
            flag.store(true, Ordering::SeqCst);
        });
        drop(task);
        std::thread::sleep(Duration::from_millis(400));
        assert!(
            !ran.load(Ordering::SeqCst),
            "the task carried on after its handle was dropped"
        );
    }
}
