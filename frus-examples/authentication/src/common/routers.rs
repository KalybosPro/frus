//! The router, and the guard that decides who may see which page.

use frus::*;

use crate::common::auth::Auth;
use crate::common::routes::*;
use crate::features::auth::pages::forgotpassword::ForgotPasswordPage;
use crate::features::auth::pages::loginpage::LoginPage;
use crate::features::auth::pages::register::RegisterPage;
use crate::features::home::pages::homepage::HomePage;
use crate::features::profile::pages::profilepage::ProfilePage;

/// The application's router.
///
/// On the desktop, a location given on the command line is where it starts — try
/// `cargo run -- /profile` while signed out: the guard sends you to
/// `/login?from=/profile`, and signing in brings you back to the profile.
pub(crate) fn router(auth: Auth) -> GoRouter {
    let start = std::env::args()
        .nth(1)
        .filter(|location| location.starts_with('/'))
        .unwrap_or_else(|| HOME.to_string());
    routes(auth, &start)
}

/// The routes, the guard and the start, without reading the command line (the tests
/// call this).
pub(crate) fn routes(auth: Auth, start: &str) -> GoRouter {
    // Each route builder keeps its own handle on the service: cloning `Auth` is cloning
    // a few `Rc`s, and every clone is the same service.
    let (for_home, for_profile, for_login, for_register, for_forgot, guard) = (
        auth.clone(),
        auth.clone(),
        auth.clone(),
        auth.clone(),
        auth.clone(),
        auth.clone(),
    );

    // A page reads what its location said from the `GoRouterState`: here, where the person
    // was heading (`?from=..`), which the auth pages pass along to each other.
    let from = |state: &GoRouterState| state.query(FROM).map(str::to_string);

    GoRouter::new(vec![
        GoRoute::new(HOME, move |_, _| {
            HomePage::new(for_home.clone()).into_widget()
        })
        .routes(vec![
            // Relative to its parent: this is `/profile`, with home underneath.
            GoRoute::new("profile", move |_, _| {
                ProfilePage::new(for_profile.clone()).into_widget()
            }),
        ]),
        GoRoute::new(LOGIN, move |_, state| {
            LoginPage::new(for_login.clone(), from(state)).into_widget()
        }),
        GoRoute::new(REGISTER, move |_, state| {
            RegisterPage::new(for_register.clone(), from(state)).into_widget()
        }),
        GoRoute::new(FORGOT_PASSWORD, move |_, state| {
            ForgotPasswordPage::new(for_forgot.clone(), from(state)).into_widget()
        }),
    ])
    .initial_location(start)
    // Asked about every navigation — and about the current page whenever `auth` changes.
    .redirect(move |state| guard_redirect(guard.is_signed_in(), state.location()))
    .refresh_listenable(auth.changes())
}

/// Where a navigation to `location` must go instead, or `None` to let it through.
///
/// A plain function of two values, so every case is tested below without a router:
///
/// | signed in | page    | goes to                                  |
/// |-----------|---------|------------------------------------------|
/// | no        | public  | where it was heading                     |
/// | no        | private | `/login?from=<where it was heading>`     |
/// | yes       | public  | the `from` it carries, or home           |
/// | yes       | private | where it was heading                     |
pub(crate) fn guard_redirect(signed_in: bool, location: &str) -> Option<String> {
    let (path, query) = location.split_once('?').unwrap_or((location, ""));
    let public = PUBLIC_ROUTES.contains(&path);
    match (signed_in, public) {
        (false, true) | (true, false) => None,
        (false, false) => Some(login_from(location)),
        (true, true) => Some(return_to(query)),
    }
}

/// The login page, remembering `location` — unless that is home, where signing in goes
/// anyway.
fn login_from(location: &str) -> String {
    if location == HOME {
        LOGIN.to_string()
    } else {
        with_from(LOGIN, Some(location))
    }
}

/// `path`, carrying `from` along: `/register?from=/profile`. How the auth pages link to
/// each other without losing where the person was going.
pub(crate) fn with_from(path: &str, from: Option<&str>) -> String {
    match from {
        Some(from) => format!("{path}?{FROM}={}", encode(from)),
        None => path.to_string(),
    }
}

/// Where to go once signed in: the `from` of the query when it is a page of this
/// application, home otherwise.
fn return_to(query: &str) -> String {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == FROM)
        .map(|(_, value)| decode(value))
        // Only a path of this application: never `//elsewhere.com` or `https://..`, and
        // never back to a sign-in page, which would bounce straight here again.
        .filter(|from| from.starts_with('/') && !from.starts_with("//"))
        .filter(|from| {
            let path = from.split('?').next().unwrap_or(from);
            !PUBLIC_ROUTES.contains(&path)
        })
        .unwrap_or_else(|| HOME.to_string())
}

/// Percent-encodes a location so it travels inside another location's query. `/` is
/// left as it is, so the address stays readable: `?from=/profile`.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// The reverse of [`encode`], also reading `+` as a space as browsers write it.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (b'+', _) => {
                out.push(b' ');
                i += 1;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::auth::tests::{answer_now, signed_out};
    use crate::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};

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
}
