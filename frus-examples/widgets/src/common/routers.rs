use frus::{GoRoute, GoRouter, StatefulWidget, StatelessWidget};

use crate::common::routes::{CATEGORY, DEMO, HOME};
use crate::features::catalog::pages::categorypage::CategoryPage;
use crate::features::catalog::pages::demopage::DemoPage;
use crate::features::home::pages::homepage::HomePage;

/// Every page of the gallery, by address. A page reads what the address said through
/// `state.param(..)`.
///
/// On the desktop, a location given on the command line opens the gallery there:
/// `cargo run -- /widget/container`, or `/widget/container?tab=code` for its code.
pub(crate) fn router() -> GoRouter {
    let start = std::env::args()
        .nth(1)
        .filter(|location| location.starts_with('/'))
        .unwrap_or_else(|| HOME.to_string());

    GoRouter::new(vec![
        GoRoute::new(HOME, |_, _| HomePage.into_widget()),
        GoRoute::new(CATEGORY, |_, state| {
            CategoryPage::new(state.param("id").unwrap_or_default()).into_widget()
        }),
        GoRoute::new(DEMO, |_, state| {
            DemoPage::new(state.param("id").unwrap_or_default(), state.query("tab")).into_widget()
        }),
    ])
    .initial_location(start)
}
