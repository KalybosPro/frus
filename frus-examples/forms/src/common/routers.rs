use frus::{GoRoute, GoRouter, StatefulWidget, StatelessWidget};

use crate::common::routes::{SIGNUP, SUMMARY};
use crate::features::signup::pages::signuppage::SignupPage;
use crate::features::summary::pages::summarypage::SummaryPage;
use crate::models::profile::Profile;

/// The two pages, by address.
///
/// On the desktop, a location given on the command line opens the application there —
/// handy while working on one page:
///
/// - `cargo run -- "/?errors"` — the form as it looks after a failed submit, to style the
///   error state without clicking through it each time;
/// - `cargo run -- /summary` — the summary page, with sample data.
pub fn router() -> GoRouter {
    let start = std::env::args()
        .nth(1)
        .filter(|location| location.starts_with('/'))
        .unwrap_or_else(|| SIGNUP.to_string());

    GoRouter::new(vec![
        GoRoute::new(SIGNUP, |_, state| {
            SignupPage::new(state.query("errors").is_some()).into_widget()
        }),
        // The page reads the profile the form sent with `push_with_extra`. Reached any other
        // way (a typed address, a deep link) there is none, and the page says so.
        GoRoute::new(SUMMARY, |_, state| {
            SummaryPage::new(state.extra::<Profile>()).into_widget()
        }),
    ])
    .initial_location(start)
}
