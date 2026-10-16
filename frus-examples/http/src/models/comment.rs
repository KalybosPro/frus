//! A comment under a post.

use serde::Deserialize;

/// One comment from `GET /posts/{id}/comments`.
///
/// The payload also carries `postId` and `id`; a field the struct does not name is skipped, so
/// a model only declares what the app shows.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct Comment {
    pub name: String,
    pub email: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_the_model_does_not_name_are_ignored() {
        let json = r#"{ "postId": 1, "id": 3, "name": "n", "email": "a@b.c", "body": "hi" }"#;
        let comment: Comment = serde_json::from_str(json).expect("a valid comment");
        assert_eq!(comment.name, "n");
        assert_eq!(comment.email, "a@b.c");
    }
}
