//! [`fetch`] : un **GET HTTP cross-plateforme**, derrière la feature `net`.
//!
//! Une seule signature — `async fn fetch(url) -> Result<String, FetchError>` — pour les
//! trois plateformes ; l'implémentation diffère mais l'app ne voit qu'une future à
//! `await`, typiquement dans un [`crate::Command::perform_async`] :
//!
//! ```ignore
//! Msg::Load => Command::perform_async(async {
//!     match frus_shell::fetch("https://example.com/api").await {
//!         Ok(body) => Msg::Loaded(body),
//!         Err(err) => Msg::Failed(err.to_string()),
//!     }
//! }),
//! ```
//!
//! - **Web** : le `fetch` du navigateur (`window.fetch`) via `web-sys` — asynchrone natif.
//! - **Natif** : le client bloquant `ureq`, mené à terme dans le corps de la future (sur
//!   le thread dédié de `perform_async` — bloquer ce thread est sans risque). TLS inclus.
//!
//! GET texte volontairement minimal ; en-têtes/POST/flux viendront si le besoin s'en fait
//! sentir.

use std::fmt;

/// Pourquoi un [`fetch`] a échoué.
#[derive(Debug, Clone)]
pub enum FetchError {
    /// Échec réseau / transport (DNS, connexion, TLS, requête invalide…).
    Network(String),
    /// La réponse a un **code d'état** non-2xx.
    Status(u16),
    /// La réponse n'a pas pu être lue en texte (UTF-8 invalide, corps non textuel…).
    Decode(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Network(m) => write!(f, "network error: {m}"),
            FetchError::Status(c) => write!(f, "HTTP status {c}"),
            FetchError::Decode(m) => write!(f, "decode error: {m}"),
        }
    }
}

impl std::error::Error for FetchError {}

/// GET l'URL et renvoie le corps en texte (**natif** : client `ureq` bloquant, exécuté
/// dans la future). Voir la doc du module.
#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch(url: impl Into<String>) -> Result<String, FetchError> {
    let url = url.into();
    match ureq::get(&url).call() {
        Ok(resp) => resp.into_string().map_err(|e| FetchError::Decode(e.to_string())),
        Err(ureq::Error::Status(code, _)) => Err(FetchError::Status(code)),
        Err(e) => Err(FetchError::Network(e.to_string())),
    }
}

/// GET l'URL et renvoie le corps en texte (**Web** : `window.fetch`, asynchrone natif).
/// Voir la doc du module.
#[cfg(target_arch = "wasm32")]
pub async fn fetch(url: impl Into<String>) -> Result<String, FetchError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = url.into();
    let window = web_sys::window().ok_or_else(|| FetchError::Network("no window".into()))?;
    let resp_val = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| FetchError::Network(format!("{e:?}")))?;
    let resp: web_sys::Response = resp_val
        .dyn_into()
        .map_err(|_| FetchError::Network("invalid response".into()))?;
    if !resp.ok() {
        return Err(FetchError::Status(resp.status()));
    }
    let text_promise = resp.text().map_err(|e| FetchError::Decode(format!("{e:?}")))?;
    let text = JsFuture::from(text_promise)
        .await
        .map_err(|e| FetchError::Decode(format!("{e:?}")))?;
    text.as_string().ok_or_else(|| FetchError::Decode("response body is not text".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_readable() {
        assert_eq!(FetchError::Status(404).to_string(), "HTTP status 404");
        assert!(FetchError::Network("dns".into()).to_string().contains("dns"));
        assert!(FetchError::Decode("utf8".into()).to_string().contains("utf8"));
    }
}
