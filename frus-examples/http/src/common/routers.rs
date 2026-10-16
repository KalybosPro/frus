use frus::{GoRoute, GoRouter, StatefulWidget};

use crate::common::routes::{HOME, NEW_POST, POST};
use crate::features::new_post::pages::newpostpage::NewPostPage;
use crate::features::post::pages::postpage::PostPage;
use crate::features::posts::pages::postspage::PostsPage;

/// Every page of the app, by address.
///
/// On the desktop, a location given on the command line opens the app there:
/// `cargo run -- /posts/1`, or `cargo run -- /posts/new`. (On the Web the address bar
/// does the same.)
pub(crate) fn router() -> GoRouter {
    let start = std::env::args()
        .nth(1)
        .filter(|location| location.starts_with('/'))
        .unwrap_or_else(|| HOME.to_string());

    GoRouter::new(vec![GoRoute::new(HOME, |_, _| PostsPage.into_widget())
        .routes(vec![
            GoRoute::new(NEW_POST, |_, _| NewPostPage.into_widget()),
            GoRoute::new(POST, |_, state| {
                // An address is text, and text from outside is not trusted: `/posts/abc`
                // gets the page's "not found" rather than a panic.
                let id = state.param("id").and_then(|id| id.parse().ok());
                PostPage::new(id).into_widget()
            }),
        ])])
    .initial_location(start)
}
