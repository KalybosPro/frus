//! Tests for `http::models::post`, moved out of `src/` (milestone 664).

use http::models::post::*;

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
