//! Tests for `authentication::features::auth::controllers::user_store`, moved out of `src/` (milestone 664).

use authentication::features::auth::controllers::user_store::*;

#[test]
fn the_demo_account_signs_in() {
    let store = UserStore::seeded();
    let user = store.sign_in(DEMO_EMAIL, DEMO_PASSWORD);
    assert_eq!(user.map(|u| u.name), Ok(DEMO_NAME.to_string()));
}

#[test]
fn the_email_is_matched_however_it_is_typed() {
    let store = UserStore::seeded();
    assert!(store.sign_in("  Demo@FRUS.dev ", DEMO_PASSWORD).is_ok());
}

#[test]
fn each_refusal_says_why() {
    let mut store = UserStore::seeded();
    assert_eq!(
        store.sign_in("nobody@frus.dev", DEMO_PASSWORD),
        Err(AuthError::UnknownEmail)
    );
    assert_eq!(
        store.sign_in(DEMO_EMAIL, "not-the-password"),
        Err(AuthError::WrongPassword)
    );
    assert_eq!(
        store.register("Someone", DEMO_EMAIL, "whatever1"),
        Err(AuthError::EmailTaken)
    );
    assert_eq!(store.rename("u999", "Ghost"), Err(AuthError::AccountGone));
}

#[test]
fn a_new_account_can_sign_in_and_has_its_own_id() {
    let mut store = UserStore::seeded();
    let ada = store.register(" Ada ", "ada@frus.dev", "analytical");
    let Ok(ada) = ada else {
        panic!("registering a new address should succeed: {ada:?}");
    };
    assert_eq!(ada.name, "Ada");
    assert_ne!(ada.id, "u1", "the demo account already has u1");
    assert_eq!(store.sign_in("ada@frus.dev", "analytical"), Ok(ada.clone()));
    assert_eq!(store.user(&ada.id), Some(ada));
}

#[test]
fn a_reset_link_only_goes_to_a_known_address() {
    let mut store = UserStore::seeded();
    store.request_password_reset("nobody@frus.dev");
    store.request_password_reset(" DEMO@frus.dev");
    assert_eq!(store.outbox(), [DEMO_EMAIL.to_string()]);
}

#[test]
fn renaming_changes_what_the_server_answers() {
    let mut store = UserStore::seeded();
    assert!(store.rename("u1", "Grace").is_ok());
    assert_eq!(store.user("u1").map(|u| u.name), Some("Grace".to_string()));
}
