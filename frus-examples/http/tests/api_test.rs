//! Tests for `http::common::api`, moved out of `src/` (milestone 662).

use frus::FetchError;
use http::common::api::*;

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
