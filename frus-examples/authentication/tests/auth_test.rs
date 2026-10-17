//! Tests for `authentication::common::auth`, moved out of `src/` (milestone 664).

use authentication::common::session::Session;
use authentication::features::auth::controllers::user_store::{AuthError, UserStore};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use authentication::common::auth::*;

mod support;
use authentication::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};
use support::{answer_now, signed_out};

/// Runs a request and hands back its reply.
fn reply_of(request: impl FnOnce(Box<dyn Fn(Reply)>)) -> Option<Reply> {
    let got: Rc<RefCell<Option<Reply>>> = Rc::default();
    let into = got.clone();
    request(Box::new(move |reply| *into.borrow_mut() = Some(reply)));
    answer_now();
    got.take()
}

#[test]
fn signing_in_waits_for_the_server_then_sets_the_user() {
    let auth = signed_out();
    let replied = Rc::new(Cell::new(false));
    let flag = replied.clone();
    auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, move |reply| {
        assert_eq!(reply, Ok(()));
        flag.set(true);
    });
    assert!(
        !auth.is_signed_in(),
        "nothing happens before the server answers"
    );
    answer_now();
    assert!(replied.get());
    assert_eq!(auth.user().map(|u| u.email), Some(DEMO_EMAIL.to_string()));
}

#[test]
fn a_refusal_reaches_the_page_and_signs_nobody_in() {
    let auth = signed_out();
    let reply = reply_of(|done| auth.sign_in(DEMO_EMAIL, "wrong-password", done));
    assert_eq!(reply, Some(Err(AuthError::WrongPassword)));
    assert!(!auth.is_signed_in());
}

#[test]
fn registering_signs_the_new_account_in() {
    let auth = signed_out();
    let reply = reply_of(|done| auth.register("Ada", "ada@frus.dev", "analytical", done));
    assert_eq!(reply, Some(Ok(())));
    assert_eq!(auth.user().map(|u| u.name), Some("Ada".to_string()));
}

#[test]
fn a_new_name_is_what_every_page_reads() {
    let auth = signed_out();
    reply_of(|done| auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, done));
    let reply = reply_of(|done| auth.update_name("Grace", done));
    assert_eq!(reply, Some(Ok(())));
    assert_eq!(auth.user().map(|u| u.name), Some("Grace".to_string()));
}

#[test]
fn signing_out_tells_the_listeners() {
    let auth = signed_out();
    reply_of(|done| auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, done));
    let told = Rc::new(Cell::new(0));
    let count = told.clone();
    let _subscription =
        frus::Listenable::add_listener(auth.changes(), move || count.set(count.get() + 1));
    auth.sign_out();
    assert!(!auth.is_signed_in());
    assert_eq!(told.get(), 1);
}

#[test]
fn a_saved_session_signs_in_at_start_up_until_signing_out() {
    let path = std::env::temp_dir()
        .join("frus-authentication-test")
        .join("session-restore");
    let first = Auth::with(UserStore::seeded(), Session::at(path.clone()));
    first.sign_out();
    reply_of(|done| first.sign_in(DEMO_EMAIL, DEMO_PASSWORD, done));

    // A restart: a new service, a new (seeded) server, the same file.
    let second = Auth::with(UserStore::seeded(), Session::at(path.clone()));
    assert_eq!(second.user().map(|u| u.email), Some(DEMO_EMAIL.to_string()));

    second.sign_out();
    let third = Auth::with(UserStore::seeded(), Session::at(path));
    assert!(!third.is_signed_in());
}

#[test]
fn a_session_for_an_account_the_server_lost_is_dropped() {
    let path = std::env::temp_dir()
        .join("frus-authentication-test")
        .join("session-lost");
    Session::at(path.clone()).save("u42");
    let auth = Auth::with(UserStore::seeded(), Session::at(path.clone()));
    assert!(!auth.is_signed_in());
    assert_eq!(Session::at(path).load(), None, "and the file is cleared");
}
