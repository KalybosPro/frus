//! A post, as the API sends it — and the one the app sends to create one.

use serde::{Deserialize, Serialize};

/// A post read from `GET /posts` or `GET /posts/{id}`, or echoed back by `POST /posts`.
///
/// `Deserialize` is all `Request::send_json` needs: the response body becomes this struct
/// in one step. The API spells its fields in camelCase (`userId`); `rename_all` maps them
/// onto Rust's snake_case names, so the struct reads like Rust and the wire stays as it is.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: u32,
    pub user_id: u32,
    pub title: String,
    pub body: String,
}

/// What the "New post" page sends. It has no `id`: the server gives one.
///
/// `Serialize` is all `Request::json_body` needs to make the request body.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPost {
    pub user_id: u32,
    pub title: String,
    pub body: String,
}
