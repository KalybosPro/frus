//! The app's HTTP client: one place that knows the server's address, the headers every
//! call sends, how long to wait, and how a failure is told to the user.
//!
//! Pages never build a URL or read a status code. They call a typed function —
//! `api::posts()`, `api::user(id)` — and get a `Result<T, ApiError>` back.
//!
//! The same code runs on every platform: `frus::Request` is the browser's `fetch` on the
//! Web and a native client on the desktop and Android, behind one API. (For a quick text
//! GET with nothing to configure, `frus::fetch(url).await` is the one-line version; every
//! call here wants headers and a timeout, so it builds a `Request`.)

use std::fmt;
use std::time::Duration;

use frus::{FetchError, Request};

use crate::models::{comment::Comment, post::NewPost, post::Post, user::User};

/// The server. Every path below is relative to it.
///
/// It answers browser requests from any origin (CORS), which is what lets the Web build
/// of this app call it too.
pub(crate) const BASE_URL: &str = "https://jsonplaceholder.typicode.com";

/// How long a call may take before it is given up. Without one, a request on a network
/// that swallows packets (a captive portal, a dead Wi-Fi) waits for as long as the OS
/// lets it: minutes, with a spinner that never ends.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------------------
// Addresses — plain functions, tested without a network
// ---------------------------------------------------------------------------------------

pub(crate) fn posts_url() -> String {
    format!("{BASE_URL}/posts")
}

pub(crate) fn post_url(id: u32) -> String {
    format!("{BASE_URL}/posts/{id}")
}

pub(crate) fn comments_url(post_id: u32) -> String {
    format!("{BASE_URL}/posts/{post_id}/comments")
}

pub(crate) fn user_url(id: u32) -> String {
    format!("{BASE_URL}/users/{id}")
}

// ---------------------------------------------------------------------------------------
// Calls
// ---------------------------------------------------------------------------------------

/// What every call starts from: the method and address, the `Accept` header, the
/// timeout. A request is a value, built by chaining, and nothing is sent until `send`.
fn request(request: Request) -> Request {
    request
        .header("Accept", "application/json")
        .timeout(TIMEOUT)
}

/// `GET /posts` — every post.
///
/// `send_json` sends the request and decodes the body into the type asked for, here a
/// `Vec<Post>`. A body that does not match the model is a `FetchError::Decode`, which
/// `?`/`map_err` turns into an `ApiError` like any other failure.
pub(crate) async fn posts() -> Result<Vec<Post>, ApiError> {
    Ok(request(Request::get(posts_url())).send_json().await?)
}

/// `GET /posts/{id}` — one post.
pub(crate) async fn post(id: u32) -> Result<Post, ApiError> {
    Ok(request(Request::get(post_url(id))).send_json().await?)
}

/// `GET /posts/{id}/comments` — the comments under a post.
pub(crate) async fn comments(post_id: u32) -> Result<Vec<Comment>, ApiError> {
    Ok(request(Request::get(comments_url(post_id)))
        .send_json()
        .await?)
}

/// `GET /users/{id}` — a post's author.
pub(crate) async fn user(id: u32) -> Result<User, ApiError> {
    Ok(request(Request::get(user_url(id))).send_json().await?)
}

/// `POST /posts` — creates a post, and answers what the server stored.
///
/// `json_body` serialises the draft and sets `Content-Type: application/json`.
pub(crate) async fn create_post(draft: NewPost) -> Result<Post, ApiError> {
    Ok(request(Request::post(posts_url()))
        .json_body(&draft)
        .send_json()
        .await?)
}

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Why a call failed, in the terms the app talks to its user in.
///
/// `frus::FetchError` says what went wrong *technically*; this says what the user can do
/// about it. The mapping lives here, once, rather than in every page.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ApiError {
    /// The server could not be reached at all: no network, airplane mode, DNS, refused.
    Offline,
    /// The server was reached, or might have been, but did not answer in time.
    Timeout,
    /// `404`: what was asked for does not exist.
    NotFound,
    /// `5xx`: the server failed. Trying again later may work.
    Server(u16),
    /// Any other status the app did not expect (`400`, `401`, …).
    Status(u16),
    /// The server answered, but not with what the model expects.
    Decode(String),
}

impl ApiError {
    /// A short title for an error view.
    pub(crate) fn title(&self) -> &'static str {
        match self {
            ApiError::Offline => "You're offline",
            ApiError::Timeout => "The server is taking too long",
            ApiError::NotFound => "Not found",
            ApiError::Server(_) => "The server had a problem",
            ApiError::Status(_) => "The request was refused",
            ApiError::Decode(_) => "Unexpected answer",
        }
    }

    /// One sentence telling the user what happened and what to do next.
    pub(crate) fn message(&self) -> String {
        match self {
            ApiError::Offline => {
                "Couldn't reach the server. Check your connection — is airplane mode on? — \
                 then try again."
                    .to_string()
            }
            ApiError::Timeout => format!(
                "No answer after {} seconds. The connection may be slow; try again.",
                TIMEOUT.as_secs()
            ),
            ApiError::NotFound => "What you asked for doesn't exist (HTTP 404).".to_string(),
            ApiError::Server(code) => {
                format!("Something went wrong on the server (HTTP {code}). Try again later.")
            }
            ApiError::Status(code) => format!("The server refused the request (HTTP {code})."),
            ApiError::Decode(detail) => {
                format!("The server's answer couldn't be read ({detail}).")
            }
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.title(), self.message())
    }
}

impl std::error::Error for ApiError {}

/// What lets `?` turn a `FetchError` into an `ApiError` in the calls above.
impl From<FetchError> for ApiError {
    fn from(error: FetchError) -> Self {
        match error {
            FetchError::Status(404) => ApiError::NotFound,
            FetchError::Status(code) if (500..600).contains(&code) => ApiError::Server(code),
            FetchError::Status(code) => ApiError::Status(code),
            FetchError::Decode(detail) => ApiError::Decode(detail),
            // A transport failure carries only the client's own wording, which differs by
            // platform: "timeout: …" natively, an `AbortError` in a browser, where the
            // deadline is enforced by aborting the request. Everything else — DNS,
            // refused, no route, TLS — is "could not reach the server".
            FetchError::Network(detail) => {
                let detail = detail.to_ascii_lowercase();
                if detail.contains("timeout")
                    || detail.contains("timed out")
                    || detail.contains("abort")
                {
                    ApiError::Timeout
                } else {
                    ApiError::Offline
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_built_from_the_base_url() {
        assert_eq!(posts_url(), "https://jsonplaceholder.typicode.com/posts");
        assert_eq!(post_url(3), "https://jsonplaceholder.typicode.com/posts/3");
        assert_eq!(
            comments_url(3),
            "https://jsonplaceholder.typicode.com/posts/3/comments"
        );
        assert_eq!(
            user_url(10),
            "https://jsonplaceholder.typicode.com/users/10"
        );
    }

    #[test]
    fn statuses_map_to_what_the_user_can_do() {
        assert_eq!(ApiError::from(FetchError::Status(404)), ApiError::NotFound);
        assert_eq!(
            ApiError::from(FetchError::Status(503)),
            ApiError::Server(503)
        );
        assert_eq!(
            ApiError::from(FetchError::Status(401)),
            ApiError::Status(401)
        );
    }

    #[test]
    fn a_deadline_is_a_timeout_on_every_platform() {
        // Native wording, then the browser's.
        let native = FetchError::Network("timeout: global".into());
        let browser = FetchError::Network("JsValue(AbortError: signal is aborted)".into());
        assert_eq!(ApiError::from(native), ApiError::Timeout);
        assert_eq!(ApiError::from(browser), ApiError::Timeout);
    }

    #[test]
    fn any_other_transport_failure_means_offline() {
        for detail in [
            "host not found",
            "connection failed",
            "io: No such host is known. (os error 11001)",
            "JsValue(TypeError: Failed to fetch)",
        ] {
            assert_eq!(
                ApiError::from(FetchError::Network(detail.into())),
                ApiError::Offline,
                "{detail}"
            );
        }
    }

    #[test]
    fn a_body_that_does_not_match_the_model_is_a_decode_error() {
        let error = ApiError::from(FetchError::Decode("expected a sequence".into()));
        assert!(matches!(error, ApiError::Decode(_)));
        assert!(error.message().contains("expected a sequence"));
    }

    #[test]
    fn the_timeout_message_names_the_deadline() {
        assert!(ApiError::Timeout.message().contains("10 seconds"));
    }
}
