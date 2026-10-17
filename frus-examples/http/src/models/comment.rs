//! A comment under a post.

use serde::Deserialize;

/// One comment from `GET /posts/{id}/comments`.
///
/// The payload also carries `postId` and `id`; a field the struct does not name is skipped, so
/// a model only declares what the app shows.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Comment {
    pub name: String,
    pub email: String,
    pub body: String,
}
