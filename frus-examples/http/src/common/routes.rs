//! Every address in the app, in one place.
//!
//! The two pages below the list are **sub-routes** of it: their paths are relative to
//! `/`, and opening one puts the list beneath it. That is what gives a deep link —
//! `/posts/1` typed in a browser, or passed on the command line — a page to go back to.

/// The list of posts.
pub(crate) const HOME: &str = "/";

/// The form that creates a post, under [`HOME`]. Listed before [`POST`] in the router:
/// routes are tried in order, and `posts/:id` would otherwise take `new` for an id.
pub(crate) const NEW_POST: &str = "posts/new";

/// One post, by id, under [`HOME`]. The page reads `:id` back with `state.param("id")`.
pub(crate) const POST: &str = "posts/:id";

/// The address of the form, for `router.push(..)`.
pub(crate) fn new_post_location() -> String {
    format!("{HOME}{NEW_POST}")
}

/// The address of one post, for `router.push(..)`.
pub(crate) fn post_location(id: u32) -> String {
    format!("{HOME}{}", POST.replace(":id", &id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_are_full_addresses() {
        assert_eq!(new_post_location(), "/posts/new");
        assert_eq!(post_location(42), "/posts/42");
    }
}
