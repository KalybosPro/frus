//! Tests for `authentication::common::routers`, moved out of `src/` (milestone 662).

use authentication::common::routers::*;
use authentication::common::routes::*;
mod support;
use authentication::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};
use support::{answer_now, signed_out};

fn guard(signed_in: bool, location: &str) -> Option<String> {
    guard_redirect(signed_in, location)
}

#[test]
fn signed_out_may_see_the_public_pages() {
    for page in PUBLIC_ROUTES {
        assert_eq!(guard(false, page), None, "{page}");
    }
    assert_eq!(guard(false, "/login?from=/profile"), None);
}

#[test]
fn signed_out_is_sent_to_login_remembering_where_it_was_going() {
    assert_eq!(guard(false, HOME), Some(LOGIN.to_string()));
    assert_eq!(
        guard(false, PROFILE),
        Some("/login?from=/profile".to_string())
    );
}

#[test]
fn signed_in_may_see_the_private_pages() {
    assert_eq!(guard(true, HOME), None);
    assert_eq!(guard(true, PROFILE), None);
}

#[test]
fn signed_in_is_sent_away_from_the_public_pages() {
    for page in PUBLIC_ROUTES {
        assert_eq!(guard(true, page), Some(HOME.to_string()), "{page}");
    }
    assert_eq!(
        guard(true, "/register?from=/profile"),
        Some(PROFILE.to_string())
    );
}

#[test]
fn from_survives_the_round_trip_query_and_all() {
    let heading = "/profile?tab=security&note=a b+c";
    let login = guard(false, heading);
    assert_eq!(
        login.as_deref(),
        Some("/login?from=/profile%3Ftab%3Dsecurity%26note%3Da%20b%2Bc")
    );
    let back = login.and_then(|login| guard(true, &login));
    assert_eq!(back.as_deref(), Some(heading));
}

#[test]
fn from_never_leaves_the_application_nor_loops() {
    for from in [
        "//evil.example",
        "https://evil.example",
        "/login",
        "/register?x=1",
        "",
    ] {
        let location = format!("/login?from={}", encode(from));
        assert_eq!(guard(true, &location), Some(HOME.to_string()), "{from}");
    }
}

/// The whole loop with a real router: the guard, the query, and the refresh when the
/// service changes.
#[test]
fn the_router_follows_signing_in_and_out() {
    let auth = signed_out();
    let router = routes(auth.clone(), PROFILE);
    assert_eq!(router.location(), "/login?from=/profile");
    assert_eq!(
        router
            .state()
            .and_then(|s| s.query(FROM).map(str::to_string)),
        Some(PROFILE.to_string())
    );

    auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, |_| {});
    answer_now();
    assert_eq!(router.location(), PROFILE, "back where it was heading");
    assert_eq!(
        router.locations(),
        [HOME, PROFILE],
        "with home to go back to"
    );

    auth.sign_out();
    assert_eq!(router.location(), "/login?from=/profile");
}
