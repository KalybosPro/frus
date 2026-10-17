//! `use_api` — a call to the server, from inside a `build`.
//!
//! `cx.use_future(deps, make)` is the framework's hook for work that finishes later: it
//! runs `make()`'s future **once until `deps` change**, and each build reads where it has
//! got to (an `AsyncSnapshot`: waiting, then done with a value). When the value arrives
//! the framework rebuilds, and the page shows it. No callback, no message, no thread to
//! manage — and it is the same on the desktop, Android and the Web.
//!
//! This hook adds two things a screen that talks to a server wants on top of it:
//!
//! - the answer as a `RemoteData` (`Loading` / `Success` / `Failure`), the shape a page
//!   matches on;
//! - **the previous answer kept on screen while the next one loads**: changing `deps` to
//!   reload starts a new future whose snapshot is empty again, and a list that blanked
//!   to a spinner on every pull-to-refresh would be a poor list.

use std::future::Future;

use frus::{BuildContext, MaybeSend, RemoteData};

use crate::common::api::ApiError;

/// What a page knows of a call.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded<T> {
    /// The answer to show: still loading, the data, or why it failed.
    pub data: RemoteData<T, ApiError>,
    /// `true` while a reload is on its way and `data` is the previous answer.
    pub reloading: bool,
}

/// Calls the server once per value of `deps`, and gives what is known of the call.
///
/// Bump something in `deps` — a counter in the page's state — to call again: that is
/// what Retry and pull-to-refresh do.
///
/// Like every hook, call it unconditionally and in the same order on every build.
pub fn use_api<D, T, F>(cx: &BuildContext, deps: D, make: impl FnOnce() -> F) -> Loaded<T>
where
    D: PartialEq + 'static,
    T: Clone + MaybeSend + 'static,
    // `MaybeSend`: natively the future runs on the shell's executor, on another thread,
    // so it must be `Send`; on the Web the browser runs it on the page's own thread and
    // there is no such bound. Futures from `frus::Request` meet it on both.
    F: Future<Output = Result<T, ApiError>> + MaybeSend + 'static,
{
    let snapshot = cx.use_future(deps, make);
    // `use_ref`, not `use_state`: remembering the last answer must not itself ask for a
    // rebuild.
    let last = cx.use_ref(|| None::<T>);
    let loaded = settle(snapshot.data, last.get());
    if let (RemoteData::Success(value), false) = (&loaded.data, loaded.reloading) {
        last.set(Some(value.clone()));
    }
    loaded
}

/// The rule, apart from the hook so it is tested without a window: what arrived wins;
/// while nothing has, the previous answer stands in, marked as reloading.
pub fn settle<T>(arrived: Option<Result<T, ApiError>>, previous: Option<T>) -> Loaded<T> {
    match (arrived, previous) {
        (Some(result), _) => Loaded {
            data: RemoteData::from_result(result),
            reloading: false,
        },
        (None, Some(previous)) => Loaded {
            data: RemoteData::Success(previous),
            reloading: true,
        },
        (None, None) => Loaded {
            data: RemoteData::Loading,
            reloading: false,
        },
    }
}
