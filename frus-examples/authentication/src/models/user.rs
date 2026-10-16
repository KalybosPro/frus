//! The signed-in person, as the application sees them.

/// Who is signed in.
///
/// There is no password here: once the server has said yes, the application has no
/// business keeping it. What identifies the session is the `id` the server handed back.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct User {
    pub id: String,
    /// The display name, which the person can change from the profile page.
    pub name: String,
    pub email: String,
}
