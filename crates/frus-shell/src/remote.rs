//! [`RemoteData`] — l'idiome Elm pour une donnée **chargée de façon asynchrone**.
//!
//! Une valeur venue du réseau (ou de tout effet [`crate::Command`]) traverse quatre états
//! bien distincts. Les mélanger dans un `Option<Result<T, E>>` (ou pire, deux booléens
//! `loading`/`error` désynchronisables) est une source classique de bugs. `RemoteData` rend
//! ces états **exclusifs** et force la `view` à traiter chacun :
//!
//! ```ignore
//! use frus::{Command, RemoteData, Request};
//!
//! struct App { user: RemoteData<User> }         // E = String par défaut
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
//!         // Un `Result` d'effet devient directement un `RemoteData`.
//!         Msg::Loaded(res) => self.user = RemoteData::from_result(res),
//!     }
//!     Command::none()
//! }
//! ```
//!
//! Dans la `view`, on **replie** les quatre cas — le compilateur garantit qu'aucun n'est
//! oublié.

/// Les quatre états d'une donnée chargée de façon asynchrone.
///
/// `E` (le type d'erreur) vaut `String` par défaut — le cas courant après
/// `FetchError::to_string()`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RemoteData<T, E = String> {
    /// Rien n'a encore été demandé (état initial).
    #[default]
    NotAsked,
    /// Une requête est en vol.
    Loading,
    /// La donnée est arrivée.
    Success(T),
    /// La requête a échoué.
    Failure(E),
}

impl<T, E> RemoteData<T, E> {
    /// Construit depuis le `Result` d'un effet : `Ok` → [`Success`](RemoteData::Success),
    /// `Err` → [`Failure`](RemoteData::Failure). Le pont idéal dans `update`.
    pub fn from_result(res: Result<T, E>) -> Self {
        match res {
            Ok(value) => RemoteData::Success(value),
            Err(err) => RemoteData::Failure(err),
        }
    }

    /// Une requête est-elle en vol ?
    pub fn is_loading(&self) -> bool {
        matches!(self, RemoteData::Loading)
    }

    /// La donnée est-elle arrivée ?
    pub fn is_success(&self) -> bool {
        matches!(self, RemoteData::Success(_))
    }

    /// La requête a-t-elle échoué ?
    pub fn is_failure(&self) -> bool {
        matches!(self, RemoteData::Failure(_))
    }

    /// La donnée, si elle est arrivée (sinon `None`).
    pub fn value(&self) -> Option<&T> {
        match self {
            RemoteData::Success(value) => Some(value),
            _ => None,
        }
    }

    /// L'erreur, si la requête a échoué (sinon `None`).
    pub fn error(&self) -> Option<&E> {
        match self {
            RemoteData::Failure(err) => Some(err),
            _ => None,
        }
    }

    /// Emprunte l'intérieur : `RemoteData<T, E>` → `RemoteData<&T, &E>`, pour replier
    /// dans une `view` sans consommer l'état.
    pub fn as_ref(&self) -> RemoteData<&T, &E> {
        match self {
            RemoteData::NotAsked => RemoteData::NotAsked,
            RemoteData::Loading => RemoteData::Loading,
            RemoteData::Success(value) => RemoteData::Success(value),
            RemoteData::Failure(err) => RemoteData::Failure(err),
        }
    }

    /// Transforme la donnée en cas de succès, sans toucher aux autres états
    /// (ex. décoder un corps JSON en type métier).
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> RemoteData<U, E> {
        match self {
            RemoteData::NotAsked => RemoteData::NotAsked,
            RemoteData::Loading => RemoteData::Loading,
            RemoteData::Success(value) => RemoteData::Success(f(value)),
            RemoteData::Failure(err) => RemoteData::Failure(err),
        }
    }

    /// Transforme l'erreur en cas d'échec, sans toucher aux autres états.
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
        // `ok` est toujours utilisable ensuite.
        assert!(ok.is_success());
    }
}
