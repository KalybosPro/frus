//! A post, as the API sends it — and the one the app sends to create one.

use serde::{Deserialize, Serialize};

/// A post read from `GET /posts` or `GET /posts/{id}`, or echoed back by `POST /posts`.
///
/// `Deserialize` is all `Request::send_json` needs: the response body becomes this struct
/// in one step. The API spells its fields in camelCase (`userId`); `rename_all` maps them
/// onto Rust's snake_case names, so the struct reads like Rust and the wire stays as it is.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Post {
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
pub(crate) struct NewPost {
    pub user_id: u32,
    pub title: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_post_decodes_from_the_api_spelling() {
        let json = r#"{ "userId": 1, "id": 7, "title": "Hello", "body": "First line\nSecond" }"#;
        let post: Post = serde_json::from_str(json).expect("a valid post");
        assert_eq!(
            post,
            Post {
                id: 7,
                user_id: 1,
                title: "Hello".into(),
                body: "First line\nSecond".into(),
            }
        );
    }

    #[test]
    fn a_list_of_posts_decodes_as_a_vec() {
        let json = r#"[
            { "userId": 1, "id": 1, "title": "a", "body": "x" },
            { "userId": 2, "id": 2, "title": "b", "body": "y" }
        ]"#;
        let posts: Vec<Post> = serde_json::from_str(json).expect("a valid list");
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[1].user_id, 2);
    }

    #[test]
    fn a_new_post_encodes_in_the_api_spelling() {
        let draft = NewPost {
            user_id: 1,
            title: "T".into(),
            body: "B".into(),
        };
        let json = serde_json::to_string(&draft).expect("serialisable");
        assert_eq!(json, r#"{"userId":1,"title":"T","body":"B"}"#);
    }
}
