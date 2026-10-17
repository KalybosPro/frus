//! The author of a post.

use serde::Deserialize;

/// A user from `GET /users/{id}`: the fields the detail page shows, nested objects
/// included — serde follows the nesting of the JSON.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct User {
    pub name: String,
    pub username: String,
    pub email: String,
    pub website: String,
    pub company: Company,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    pub name: String,
    pub catch_phrase: String,
}
