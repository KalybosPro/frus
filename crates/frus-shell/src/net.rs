//! Cross-platform HTTP, behind the `net` feature.
//!
//! Two levels of API, one of each for all three platforms — desktop, Android, Web:
//!
//! - [`fetch(url)`](fetch) — the **shortcut**: a text GET, to be `await`ed directly.
//! - [`Request`] — the **builder** for when more is needed: a method (`POST`, `PUT`,
//!   …), **headers**, a **body**, a **timeout**. Finish with [`Request::send`].
//!
//! ```ignore
//! use frus_shell::{Command, Request};
//!
//! // A plain GET.
//! Msg::Load => Command::perform_async(async {
//!     match frus_shell::fetch("https://example.com/api").await {
//!         Ok(body) => Msg::Loaded(body),
//!         Err(err) => Msg::Failed(err.to_string()),
//!     }
//! }),
//!
//! // A JSON POST with a header and a deadline.
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
//! - **Web**: the browser's `fetch` (`window.fetch`) through `web-sys`, natively
//!   asynchronous; the timeout is armed by an `AbortController` plus `setTimeout`.
//! - **Native**: the blocking `ureq` client, driven to completion inside the future's
//!   body, on `perform_async`'s own thread — blocking that thread is harmless. TLS
//!   included.
//!
//! **JSON** (the `json` feature): [`Request::json_body`] posts a `Serialize` value and
//! [`Request::send_json`] deserialises the response into a `T: DeserializeOwned` —
//! enough to surface a typed `RemoteData<T>` rather than a `RemoteData<String>` to
//! re-parse.

use std::fmt;
use std::time::Duration;

/// A [`Request`]'s HTTP method.
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
    /// The HTTP verb spelled out (`"GET"`, `"POST"`, …).
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

/// Why a request failed.
#[derive(Debug, Clone)]
pub enum FetchError {
    /// A network or transport failure: DNS, connection, TLS, timeout, a bad request.
    Network(String),
    /// The response carries a non-2xx **status code**.
    Status(u16),
    /// The response could not be read as text: invalid UTF-8, a non-textual body.
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

/// An HTTP request to build, then [`send`](Request::send).
///
/// Built by chaining — the method first ([`get`](Request::get),
/// [`post`](Request::post), …), then headers, body and timeout as needed:
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
    /// A **deferred** error left behind by a fallible builder step — for instance
    /// [`json_body`](Request::json_body) when serialisation fails. [`send`](Request::send)
    /// surfaces it as is, which keeps the chaining fluent; `reqwest`'s builder does
    /// the same.
    error: Option<FetchError>,
}

impl Request {
    /// A request for `method` to `url`, with no header, body or timeout.
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

    /// Shortcut: a `GET` request.
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(Method::Get, url)
    }
    /// Shortcut: a `POST` request.
    pub fn post(url: impl Into<String>) -> Self {
        Self::new(Method::Post, url)
    }
    /// Shortcut: a `PUT` request.
    pub fn put(url: impl Into<String>) -> Self {
        Self::new(Method::Put, url)
    }
    /// Shortcut: a `DELETE` request.
    pub fn delete(url: impl Into<String>) -> Self {
        Self::new(Method::Delete, url)
    }

    /// Adds a header; they accumulate, so calling this again overwrites nothing.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets the request's body, as text. The last call wins.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// The deadline before giving up (`FetchError::Network`). Without this call the
    /// client imposes no deadline of its own.
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Serialises `body` to **JSON** as the request's body, and sets the
    /// `Content-Type: application/json` header (the `json` feature).
    ///
    /// The chaining stays fluent: a serialisation error, which is rare, is **deferred**
    /// and comes out of [`send`](Request::send) rather than breaking the call.
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

    /// Sends the request and **deserialises** the JSON response into `T` (the `json`
    /// feature).
    ///
    /// Equivalent to [`send`](Request::send) followed by `serde_json::from_str`; a body
    /// that cannot be read, or does not match `T`, yields a [`FetchError::Decode`].
    ///
    /// ```ignore
    /// let user: User = Request::get(url).send_json().await?;
    /// ```
    #[cfg(feature = "json")]
    pub async fn send_json<T: serde::de::DeserializeOwned>(self) -> Result<T, FetchError> {
        let body = self.send().await?;
        decode_json(&body)
    }

    /// Runs the request and returns the response's body as text.
    ///
    /// **Native**: the blocking `ureq` client, run inside the future.
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

    /// Runs the request and returns the response's body as text.
    ///
    /// **Web**: `window.fetch`; the timeout is armed by an `AbortController` plus
    /// `setTimeout`, and disarmed as soon as the response arrives.
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

        // The timeout: an AbortController whose signal is handed to the request, and
        // a setTimeout that calls `abort()` once the deadline passes.
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

        // Arm the timer just before sending, and keep the `Closure` alive until the
        // request ends; it is disarmed before being freed.
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

        // Disarm the timer whatever the outcome, after which the `Closure` may die.
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

/// Shortcut: GETs `url` and returns the body as text. Equivalent to
/// `Request::get(url).send().await`. See the module documentation.
pub async fn fetch(url: impl Into<String>) -> Result<String, FetchError> {
    Request::get(url).send().await
}

/// Deserialises a JSON body into `T`, a parse error becoming [`FetchError::Decode`].
/// Kept — and tested — apart from the network I/O. See [`Request::send_json`].
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
        // `fetch(url)` must add nothing: a GET, with no header, body or timeout.
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
        let ok: Point = decode_json(r#"{"x":3,"y":4}"#).expect("a valid JSON body");
        assert_eq!(ok, Point { x: 3, y: 4 });

        let bad = decode_json::<Point>("not json");
        assert!(
            matches!(bad, Err(FetchError::Decode(_))),
            "an unreadable body -> Decode"
        );
    }
}
