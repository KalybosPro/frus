//! Tests for `http::common::remote`, moved out of `src/` (milestone 664).

use frus::RemoteData;
use http::common::api::ApiError;
use http::common::remote::*;

#[test]
fn the_first_call_is_loading() {
    let loaded = settle::<u8>(None, None);
    assert_eq!(loaded.data, RemoteData::Loading);
    assert!(!loaded.reloading);
}

#[test]
fn a_reload_keeps_the_previous_answer_on_screen() {
    let loaded = settle(None, Some(vec![1, 2, 3]));
    assert_eq!(loaded.data, RemoteData::Success(vec![1, 2, 3]));
    assert!(loaded.reloading);
}

#[test]
fn what_arrives_replaces_the_previous_answer() {
    let loaded = settle(Some(Ok(2)), Some(1));
    assert_eq!(loaded.data, RemoteData::Success(2));
    assert!(!loaded.reloading);

    let failed = settle(Some(Err(ApiError::Offline)), Some(1));
    assert_eq!(failed.data, RemoteData::Failure(ApiError::Offline));
}
