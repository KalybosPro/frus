//! The application's addresses, named once.

/// Home: only for someone signed in.
pub const HOME: &str = "/";
/// The profile, a page above home (its route is declared under home's, so it has home
/// beneath it to go back to).
pub const PROFILE: &str = "/profile";
/// Sign in.
pub const LOGIN: &str = "/login";
/// Create an account.
pub const REGISTER: &str = "/register";
/// Ask for a password reset link.
pub const FORGOT_PASSWORD: &str = "/forgot-password";

/// The pages someone signed out may see. Everything else needs a session; and someone
/// signed in has no business on these.
pub const PUBLIC_ROUTES: [&str; 3] = [LOGIN, REGISTER, FORGOT_PASSWORD];

/// The query parameter that carries where the person was heading before being asked to
/// sign in: `/login?from=/profile`.
pub const FROM: &str = "from";
