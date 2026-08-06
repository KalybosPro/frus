//! HTTP cross-plateforme, derrière la feature `net`.
//!
//! Deux niveaux d'API, une seule pour les trois plateformes (bureau, Android, Web) :
//!
//! - [`fetch(url)`](fetch) — le **raccourci** : un GET texte, à `await` directement.
//! - [`Request`] — le **constructeur** quand il faut plus : méthode (`POST`, `PUT`…),
//!   **en-têtes**, **corps**, **timeout**. On termine par [`Request::send`].
//!
//! ```ignore
//! use frus_shell::{Command, Request};
//!
//! // GET simple.
//! Msg::Load => Command::perform_async(async {
//!     match frus_shell::fetch("https://example.com/api").await {
//!         Ok(body) => Msg::Loaded(body),
//!         Err(err) => Msg::Failed(err.to_string()),
//!     }
//! }),
//!
//! // POST JSON avec en-tête et délai maximal.
//! Msg::Save(json) => Command::perform_async(async move {
//!     let res = Request::post("https://example.com/api")
//!         .header("Content-Type", "application/json")
//!         .body(json)
//!         .timeout(std::time::Duration::from_secs(10))
//!         .send()
//!         .await;
//!     match res { Ok(_) => Msg::Saved, Err(e) => Msg::Failed(e.to_string()) }
//! }),
//! ```
//!
//! - **Web** : le `fetch` du navigateur (`window.fetch`) via `web-sys` — asynchrone natif ;
//!   le timeout est armé par un `AbortController` + `setTimeout`.
//! - **Natif** : le client bloquant `ureq`, mené à terme dans le corps de la future (sur
//!   le thread dédié de `perform_async` — bloquer ce thread est sans risque). TLS inclus.
//!
//! **JSON** (feature `json`) : [`Request::json_body`] poste une valeur `Serialize`, et
//! [`Request::send_json`] désérialise la réponse en un `T: DeserializeOwned` — de quoi
//! remonter un `RemoteData<T>` typé plutôt qu'un `RemoteData<String>` à re-parser.

use std::fmt;
use std::time::Duration;

/// Méthode HTTP d'une [`Request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
}

impl Method {
    /// Le verbe HTTP en toutes lettres (`"GET"`, `"POST"`…).
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
        }
    }
}

/// Pourquoi une requête a échoué.
#[derive(Debug, Clone)]
pub enum FetchError {
    /// Échec réseau / transport (DNS, connexion, TLS, timeout, requête invalide…).
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

/// Une requête HTTP à construire puis [`send`](Request::send).
///
/// Bâtie par chaînage — méthode d'abord ([`get`](Request::get), [`post`](Request::post)…),
/// puis en-têtes / corps / timeout au besoin :
///
/// ```ignore
/// Request::post(url).header("Accept", "application/json").body(payload).send().await
/// ```
#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    timeout: Option<Duration>,
    /// Erreur **différée** posée par un constructeur faillible (ex. [`json_body`](Request::json_body)
    /// si la sérialisation échoue). Surfacée telle quelle par [`send`](Request::send), pour que
    /// le chaînage reste fluide (motif du builder de `reqwest`).
    error: Option<FetchError>,
}

impl Request {
    /// Une requête pour `method` vers `url` (sans en-tête, corps ni timeout).
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout: None,
            error: None,
        }
    }

    /// Raccourci : requête `GET`.
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(Method::Get, url)
    }
    /// Raccourci : requête `POST`.
    pub fn post(url: impl Into<String>) -> Self {
        Self::new(Method::Post, url)
    }
    /// Raccourci : requête `PUT`.
    pub fn put(url: impl Into<String>) -> Self {
        Self::new(Method::Put, url)
    }
    /// Raccourci : requête `DELETE`.
    pub fn delete(url: impl Into<String>) -> Self {
        Self::new(Method::Delete, url)
    }

    /// Ajoute un en-tête (cumulable : appeler plusieurs fois n'écrase rien).
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Fixe le corps de la requête (texte). Le dernier appel gagne.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Délai maximal avant abandon (`FetchError::Network`). Sans cet appel, aucun
    /// délai n'est imposé côté client.
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Sérialise `body` en **JSON** comme corps de la requête, et pose l'en-tête
    /// `Content-Type: application/json` (feature `json`).
    ///
    /// Le chaînage reste fluide : une erreur de sérialisation (rare) est **différée** et
    /// ressort à [`send`](Request::send) plutôt que de rompre l'appel.
    ///
    /// ```ignore
    /// Request::post(url).json_body(&payload).send().await
    /// ```
    #[cfg(feature = "json")]
    pub fn json_body<B: serde::Serialize>(mut self, body: &B) -> Self {
        match serde_json::to_string(body) {
            Ok(json) => {
                self.headers
                    .push(("Content-Type".to_string(), "application/json".to_string()));
                self.body = Some(json);
            }
            Err(err) => self.error = Some(FetchError::Decode(err.to_string())),
        }
        self
    }

    /// Envoie la requête et **désérialise** la réponse JSON en `T` (feature `json`).
    ///
    /// Équivaut à [`send`](Request::send) suivi de `serde_json::from_str` ; un corps
    /// illisible ou non conforme à `T` donne un [`FetchError::Decode`].
    ///
    /// ```ignore
    /// let user: User = Request::get(url).send_json().await?;
    /// ```
    #[cfg(feature = "json")]
    pub async fn send_json<T: serde::de::DeserializeOwned>(self) -> Result<T, FetchError> {
        let body = self.send().await?;
        decode_json(&body)
    }

    /// Exécute la requête et renvoie le corps de la réponse en texte.
    ///
    /// **Natif** : client `ureq` bloquant, exécuté dans la future.
    #[cfg(not(web))]
    pub async fn send(self) -> Result<String, FetchError> {
        if let Some(err) = self.error {
            return Err(err);
        }
        let mut req = ureq::request(self.method.as_str(), &self.url);
        for (name, value) in &self.headers {
            req = req.set(name, value);
        }
        if let Some(dur) = self.timeout {
            req = req.timeout(dur);
        }
        let result = match &self.body {
            Some(b) => req.send_string(b),
            None => req.call(),
        };
        match result {
            Ok(resp) => resp
                .into_string()
                .map_err(|e| FetchError::Decode(e.to_string())),
            Err(ureq::Error::Status(code, _)) => Err(FetchError::Status(code)),
            Err(e) => Err(FetchError::Network(e.to_string())),
        }
    }

    /// Exécute la requête et renvoie le corps de la réponse en texte.
    ///
    /// **Web** : `window.fetch` ; le timeout est armé par un `AbortController` +
    /// `setTimeout`, désarmé dès la réponse reçue.
    #[cfg(web)]
    pub async fn send(self) -> Result<String, FetchError> {
        if let Some(err) = self.error {
            return Err(err);
        }
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::{JsCast, JsValue};
        use wasm_bindgen_futures::JsFuture;

        let window = web_sys::window().ok_or_else(|| FetchError::Network("no window".into()))?;

        let init = web_sys::RequestInit::new();
        init.set_method(self.method.as_str());

        if !self.headers.is_empty() {
            let headers =
                web_sys::Headers::new().map_err(|e| FetchError::Network(format!("{e:?}")))?;
            for (name, value) in &self.headers {
                headers
                    .append(name, value)
                    .map_err(|e| FetchError::Network(format!("{e:?}")))?;
            }
            init.set_headers(&headers);
        }

        if let Some(b) = &self.body {
            init.set_body(&JsValue::from_str(b));
        }

        // Timeout : un AbortController dont le signal est passé à la requête ; un
        // setTimeout déclenche `abort()` au-delà du délai.
        let controller = if self.timeout.is_some() {
            let c = web_sys::AbortController::new()
                .map_err(|e| FetchError::Network(format!("{e:?}")))?;
            init.set_signal(Some(&c.signal()));
            Some(c)
        } else {
            None
        };

        let request = web_sys::Request::new_with_str_and_init(&self.url, &init)
            .map_err(|e| FetchError::Network(format!("{e:?}")))?;

        // Arme le minuteur juste avant l'envoi ; on garde le `Closure` vivant jusqu'à
        // la fin de la requête (il est désarmé avant d'être libéré).
        let mut _timer_closure = None;
        let timeout_handle = if let (Some(dur), Some(controller)) = (self.timeout, controller) {
            let ms = dur.as_millis().min(i32::MAX as u128) as i32;
            let closure = Closure::<dyn FnMut()>::new(move || controller.abort());
            let handle = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    ms,
                )
                .map_err(|e| FetchError::Network(format!("{e:?}")))?;
            _timer_closure = Some(closure);
            Some(handle)
        } else {
            None
        };

        let resp_val = JsFuture::from(window.fetch_with_request(&request)).await;

        // Désarme le minuteur quel que soit le résultat (le `Closure` peut alors mourir).
        if let Some(handle) = timeout_handle {
            window.clear_timeout_with_handle(handle);
        }

        let resp_val = resp_val.map_err(|e| FetchError::Network(format!("{e:?}")))?;
        let resp: web_sys::Response = resp_val
            .dyn_into()
            .map_err(|_| FetchError::Network("invalid response".into()))?;
        if !resp.ok() {
            return Err(FetchError::Status(resp.status()));
        }
        let text_promise = resp
            .text()
            .map_err(|e| FetchError::Decode(format!("{e:?}")))?;
        let text = JsFuture::from(text_promise)
            .await
            .map_err(|e| FetchError::Decode(format!("{e:?}")))?;
        text.as_string()
            .ok_or_else(|| FetchError::Decode("response body is not text".into()))
    }
}

/// Raccourci : GET l'`url` et renvoie le corps en texte. Équivaut à
/// `Request::get(url).send().await`. Voir la doc du module.
pub async fn fetch(url: impl Into<String>) -> Result<String, FetchError> {
    Request::get(url).send().await
}

/// Désérialise un corps JSON en `T`, une erreur de parsing devenant [`FetchError::Decode`].
/// Isolé (et testé) à part de l'E/S réseau. Voir [`Request::send_json`].
#[cfg(feature = "json")]
fn decode_json<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, FetchError> {
    serde_json::from_str(body).map_err(|err| FetchError::Decode(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_readable() {
        assert_eq!(FetchError::Status(404).to_string(), "HTTP status 404");
        assert!(FetchError::Network("dns".into())
            .to_string()
            .contains("dns"));
        assert!(FetchError::Decode("utf8".into())
            .to_string()
            .contains("utf8"));
    }

    #[test]
    fn method_verbs() {
        assert_eq!(Method::Get.as_str(), "GET");
        assert_eq!(Method::Post.as_str(), "POST");
        assert_eq!(Method::Delete.as_str(), "DELETE");
    }

    #[test]
    fn builder_accumulates_headers_body_and_timeout() {
        let r = Request::post("https://example.com/api")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body("{}")
            .timeout(Duration::from_secs(5));

        assert_eq!(r.method, Method::Post);
        assert_eq!(r.url, "https://example.com/api");
        assert_eq!(
            r.headers,
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Accept".to_string(), "application/json".to_string()),
            ]
        );
        assert_eq!(r.body.as_deref(), Some("{}"));
        assert_eq!(r.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn fetch_shortcut_is_a_bare_get() {
        // `fetch(url)` ne doit rien ajouter : GET, sans en-tête ni corps ni timeout.
        let r = Request::get("https://example.com");
        assert_eq!(r.method, Method::Get);
        assert!(r.headers.is_empty());
        assert!(r.body.is_none());
        assert!(r.timeout.is_none());
    }

    #[cfg(feature = "json")]
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[cfg(feature = "json")]
    #[test]
    fn json_body_serializes_and_sets_content_type() {
        let r = Request::post("https://example.com").json_body(&Point { x: 1, y: 2 });
        assert_eq!(r.body.as_deref(), Some(r#"{"x":1,"y":2}"#));
        assert!(r
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
        assert!(r.error.is_none());
    }

    #[cfg(feature = "json")]
    #[test]
    fn decode_json_maps_valid_and_invalid_bodies() {
        let ok: Point = decode_json(r#"{"x":3,"y":4}"#).expect("corps JSON valide");
        assert_eq!(ok, Point { x: 3, y: 4 });

        let bad = decode_json::<Point>("not json");
        assert!(
            matches!(bad, Err(FetchError::Decode(_))),
            "corps illisible -> Decode"
        );
    }
}
