//! Tests for `http::models::comment`, moved out of `src/` (milestone 662).

use http::models::comment::*;

#[test]
fn fields_the_model_does_not_name_are_ignored() {
    let json = r#"{ "postId": 1, "id": 3, "name": "n", "email": "a@b.c", "body": "hi" }"#;
    let comment: Comment = serde_json::from_str(json).expect("a valid comment");
    assert_eq!(comment.name, "n");
    assert_eq!(comment.email, "a@b.c");
}
