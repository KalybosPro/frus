use frus::{BuildContext, Button, Icons, Scaffold, StatelessWidget, Widget};

use crate::common::{
    routes::HOME,
    shared::{empty_state, page_app_bar},
};

/// What the router builds for a location **no route answers** — given to it with
/// `GoRouter::error_builder`. Without one the router shows a plain line of text.
///
/// It is told the location, so it can say which one was wrong.
pub(crate) struct NotFoundPage {
    location: String,
}

impl NotFoundPage {
    pub(crate) fn new(location: &str) -> Self {
        Self {
            location: location.to_string(),
        }
    }
}

impl StatelessWidget for NotFoundPage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let router = cx.router();
        // `go`, not `pop`: a page opened at a wrong address has nothing beneath it.
        let home = Button::new("Back to the shop").on_press(move || router.go(HOME));

        Scaffold::new()
            .background(cx.theme().background)
            .app_bar(page_app_bar(cx, "Page not found").build())
            .body(empty_state(
                cx,
                Icons::LINK_OFF,
                format!("Nothing lives at {}", self.location),
                Some(home),
            ))
            .build()
    }
}
