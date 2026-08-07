//! [`RemoteData`] — the Elm idiom for a value **loaded asynchronously**.
//!
//! A value coming from the network, or from any [`crate::Command`] effect, goes
//! through four clearly distinct states. Blending them into an `Option<Result<T, E>>`
//! — or worse, two `loading`/`error` booleans that can drift apart — is a classic
//! source of bugs. `RemoteData` makes the states **exclusive** and forces the `view`
//! to handle each one:
//!
//! ```ignore
//! use frus::{Command, RemoteData, Request};
//!
//! struct App { user: RemoteData<User> }         // E defaults to String
//!
//! fn update(&mut self, msg: Msg) -> Command<Msg> {
//!     match msg {
//!         Msg::Load => {
//!             self.user = RemoteData::Loading;
//!             return Command::perform_async(async {
//!                 let res = Request::get(URL).send().await;
//!                 Msg::Loaded(res.map_err(|e| e.to_string()))
//!             });
//!         }
//!         // An effect's `Result` becomes a `RemoteData` directly.
//!         Msg::Loaded(res) => self.user = RemoteData::from_result(res),
//!     }
//!     Command::none()
//! }
//! ```
//!
//! In the `view` the four cases are **folded** over, and the compiler guarantees none
//! is forgotten.

/// The four states of an asynchronously loaded value.
///
/// `E`, the error type, defaults to `String` — the common case after
/// `FetchError::to_string()`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RemoteData<T, E = String> {
    /// Nothing has been asked for yet; the initial state.
    #[default]
    NotAsked,
    /// A request is in flight.
    Loading,
    /// The value has arrived.
    Success(T),
    /// The request failed.
    Failure(E),
}

impl<T, E> RemoteData<T, E> {
    /// Builds from an effect's `Result`: `Ok` → [`Success`](RemoteData::Success),
    /// `Err` → [`Failure`](RemoteData::Failure). The natural bridge inside `update`.
    pub fn from_result(res: Result<T, E>) -> Self {
        match res {
            Ok(value) => RemoteData::Success(value),
            Err(err) => RemoteData::Failure(err),
        }
    }

    /// Is a request in flight?
    pub fn is_loading(&self) -> bool {
        matches!(self, RemoteData::Loading)
    }

    /// Has the value arrived?
    pub fn is_success(&self) -> bool {
        matches!(self, RemoteData::Success(_))
    }

    /// Did the request fail?
    pub fn is_failure(&self) -> bool {
        matches!(self, RemoteData::Failure(_))
    }

    /// The value if it has arrived, `None` otherwise.
    pub fn value(&self) -> Option<&T> {
        match self {
            RemoteData::Success(value) => Some(value),
            _ => None,
        }
    }

    /// The error if the request failed, `None` otherwise.
    pub fn error(&self) -> Option<&E> {
        match self {
            RemoteData::Failure(err) => Some(err),
            _ => None,
        }
    }

    /// Borrows the inside: `RemoteData<T, E>` → `RemoteData<&T, &E>`, so a `view` can
    /// fold over it without consuming the state.
    pub fn as_ref(&self) -> RemoteData<&T, &E> {
        match self {
            RemoteData::NotAsked => RemoteData::NotAsked,
            RemoteData::Loading => RemoteData::Loading,
            RemoteData::Success(value) => RemoteData::Success(value),
            RemoteData::Failure(err) => RemoteData::Failure(err),
        }
    }

    /// Transforms the value on success, leaving the other states untouched — decoding
    /// a JSON body into a domain type, say.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> RemoteData<U, E> {
        match self {
            RemoteData::NotAsked => RemoteData::NotAsked,
            RemoteData::Loading => RemoteData::Loading,
            RemoteData::Success(value) => RemoteData::Success(f(value)),
            RemoteData::Failure(err) => RemoteData::Failure(err),
        }
    }

    /// Transforms the error on failure, leaving the other states untouched.
    pub fn map_err<F>(self, f: impl FnOnce(E) -> F) -> RemoteData<T, F> {
        match self {
            RemoteData::NotAsked => RemoteData::NotAsked,
            RemoteData::Loading => RemoteData::Loading,
            RemoteData::Success(value) => RemoteData::Success(value),
            RemoteData::Failure(err) => RemoteData::Failure(f(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_asked() {
        let data: RemoteData<i32> = RemoteData::default();
        assert_eq!(data, RemoteData::NotAsked);
    }

    #[test]
    fn from_result_bridges_ok_and_err() {
        let ok: RemoteData<i32> = RemoteData::from_result(Ok(7));
        assert_eq!(ok, RemoteData::Success(7));
        let err: RemoteData<i32> = RemoteData::from_result(Err("boom".to_string()));
        assert_eq!(err, RemoteData::Failure("boom".to_string()));
    }

    #[test]
    fn predicates_and_accessors() {
        let ok: RemoteData<i32> = RemoteData::Success(42);
        assert!(ok.is_success() && !ok.is_loading() && !ok.is_failure());
        assert_eq!(ok.value(), Some(&42));
        assert_eq!(ok.error(), None);

        let err: RemoteData<i32> = RemoteData::Failure("nope".to_string());
        assert!(err.is_failure());
        assert_eq!(err.value(), None);
        assert_eq!(err.error(), Some(&"nope".to_string()));

        let loading: RemoteData<i32> = RemoteData::Loading;
        assert!(loading.is_loading());
    }

    #[test]
    fn map_only_touches_success() {
        let ok: RemoteData<i32> = RemoteData::Success(3);
        assert_eq!(ok.map(|n| n * 2), RemoteData::Success(6));

        let loading: RemoteData<i32> = RemoteData::Loading;
        assert_eq!(loading.map(|n| n * 2), RemoteData::Loading);

        let err: RemoteData<i32> = RemoteData::Failure("e".to_string());
        assert_eq!(err.map(|n| n * 2), RemoteData::Failure("e".to_string()));
    }

    #[test]
    fn map_err_only_touches_failure() {
        let err: RemoteData<i32, u16> = RemoteData::Failure(404);
        let mapped: RemoteData<i32, String> = err.map_err(|c| format!("HTTP {c}"));
        assert_eq!(mapped, RemoteData::Failure("HTTP 404".to_string()));
    }

    #[test]
    fn as_ref_borrows_without_moving() {
        let ok: RemoteData<String> = RemoteData::Success("hi".to_string());
        assert_eq!(ok.as_ref().value().map(|s| s.as_str()), Some("hi"));
        // `ok` is still usable afterwards.
        assert!(ok.is_success());
    }
}
