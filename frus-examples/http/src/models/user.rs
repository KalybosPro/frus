//! The author of a post.

use serde::Deserialize;

/// A user from `GET /users/{id}`: the fields the detail page shows, nested objects
/// included — serde follows the nesting of the JSON.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct User {
    pub name: String,
    pub username: String,
    pub email: String,
    pub website: String,
    pub company: Company,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Company {
    pub name: String,
    pub catch_phrase: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed copy of what `GET /users/1` answers, address and phone included: the
    /// model takes what it names and skips the rest.
    const SAMPLE: &str = r#"{
        "id": 1,
        "name": "Leanne Graham",
        "username": "Bret",
        "email": "Sincere@april.biz",
        "address": { "street": "Kulas Light", "city": "Gwenborough" },
        "phone": "1-770-736-8031 x56442",
        "website": "hildegard.org",
        "company": {
            "name": "Romaguera-Crona",
            "catchPhrase": "Multi-layered client-server neural-net",
            "bs": "harness real-time e-markets"
        }
    }"#;

    #[test]
    fn a_user_decodes_with_its_company() {
        let user: User = serde_json::from_str(SAMPLE).expect("a valid user");
        assert_eq!(user.username, "Bret");
        assert_eq!(user.company.name, "Romaguera-Crona");
        assert_eq!(
            user.company.catch_phrase,
            "Multi-layered client-server neural-net"
        );
    }
}
