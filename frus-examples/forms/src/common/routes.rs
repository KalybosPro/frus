//! The application's routes.

/// The sign-up form. `/?errors` opens it as if a submit had already failed.
pub(crate) const SIGNUP: &str = "/";
/// What was submitted. The profile travels with the navigation (see `routers.rs`).
pub(crate) const SUMMARY: &str = "/summary";
