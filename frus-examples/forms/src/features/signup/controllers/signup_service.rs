//! A stand-in for the server that creates accounts.
//!
//! A real application would send the profile over the network here (see the `http`
//! example). This one waits a moment, the way a request does, so that the page has a
//! "submitting" state to show — and turns one address down, so that it has a server-side
//! error to show as well.

use std::time::Duration;

use frus::host;

use crate::models::profile::Profile;

/// How long the pretend request takes.
pub const LATENCY: Duration = Duration::from_millis(1500);

/// The address the pretend server says is already registered. Try it to see a server error
/// land on the email field.
pub const TAKEN_EMAIL: &str = "taken@example.com";

/// Why the server refused the account.
#[derive(Clone, Debug, PartialEq)]
pub enum SignupError {
    /// The address belongs to another account.
    EmailTaken(String),
}

/// Sends `profile` to the pretend server; `done` gets its answer, on the interface's thread,
/// once [`LATENCY`] has passed.
///
/// `host::after` asks the shell for a timer: it is not a sleep, so the window keeps
/// drawing — and the page keeps showing its spinner — while the request is "in flight".
pub fn register(profile: Profile, done: impl Fn(Result<Profile, SignupError>) + 'static) {
    host::after(LATENCY, move || done(answer(profile.clone())));
}

/// What the server decides, apart from the waiting — so it can be tested.
pub fn answer(profile: Profile) -> Result<Profile, SignupError> {
    if profile.email.eq_ignore_ascii_case(TAKEN_EMAIL) {
        Err(SignupError::EmailTaken(profile.email))
    } else {
        Ok(profile)
    }
}
