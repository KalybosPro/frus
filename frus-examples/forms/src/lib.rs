//! `forms` — a real form: input, validation, submission.
//!
//! - Desktop: `cargo run`  (`cargo run -- "/?errors"` opens the form with its errors
//!   showing, `cargo run -- /summary` the page that follows a submit)
//! - Android: `cargo apk run --lib`
//!
//! What to look at:
//!
//! - `features/signup/controllers/validation.rs` — the rules, built with `frus::form`'s
//!   `Form` and `Rule`: required fields, an email shape, a cross-field password check, and
//!   rules over values that are not text. Pure functions, tested without a window;
//! - `features/signup/states/signupstate.rs` — the state: text in
//!   `TextEditingController`s, when errors show, the submit-in-flight flag;
//! - `features/signup/pages/signuppage.rs` — the page: every kind of input, each field
//!   reading its error from the report, the error summary, focus on the first invalid
//!   field, autofill hints, the date dialog;
//! - `features/signup/controllers/signup_service.rs` — a pretend server, answering after a
//!   delay through `host::after`;
//! - `common/routers.rs` — two routes, the profile carried from one to the other.

use frus::FrusApp;

use crate::common::routers::router;

pub mod common;
pub mod features;
pub mod models;

frus::main!(FrusApp::router(router()).title("Create your account"));
