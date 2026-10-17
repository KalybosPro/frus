use frus::{
    Align, AppBar, Button, CircularProgressIndicator, Column, Icon, Icons, Scaffold,
    SingleChildScrollView, State, StateContext, StatefulWidget, StatelessWidget, Text, TextField,
    Variant, Widget,
};

use crate::common::api;
use crate::common::shared::back_button;
use crate::features::new_post::states::newpoststate::{NewPostState, SendStatus, AUTHOR_ID};
use crate::features::new_post::widgets::created_post::CreatedPost;

/// The form that creates a post with `POST /posts`.
pub struct NewPostPage;

impl StatefulWidget for NewPostPage {
    type State = NewPostState;

    fn create_state(&self) -> NewPostState {
        NewPostState::new()
    }
}

impl State for NewPostState {
    type Widget = NewPostPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();

        // A request the user starts, rather than one the page needs to show itself: the
        // same hook, keyed on `attempt`, which only Send changes. Before the first send
        // there is nothing to carry, and the future answers `None` at once.
        //
        // The future owns what it sends (`draft` is moved in): it may still be running
        // after the user has typed something else.
        let draft = self.sending.clone();
        let outcome = cx.use_future(self.attempt, move || async move {
            match draft {
                Some(draft) => Some(api::create_post(draft).await),
                None => None,
            }
        });
        let status = self.status(outcome.data);
        let busy = status == SendStatus::Sending;

        let title = TextField::new(self.title.as_str())
            .label("Title")
            .enabled(!busy)
            .on_input(cx.handler(NewPostState::set_title));
        let title = match self.error("title") {
            Some(message) => title.error(message),
            None => title,
        };

        let body = TextField::new(self.body.as_str())
            .label("Body")
            .rows(5)
            .enabled(!busy)
            .on_input(cx.handler(NewPostState::set_body));
        let body = match self.error("body") {
            Some(message) => body.error(message),
            None => body,
        };

        // While the request is out the button is disabled and says why — a second press
        // would create a second post.
        let send = if busy {
            Button::new("Sending…")
                .icon(CircularProgressIndicator::new().size(16.0))
                .enabled(false)
        } else {
            Button::new("Send")
                .icon(Icon::new(Icons::SEND))
                .on_press(cx.callback(move |state: &mut NewPostState| state.submit(busy)))
        };

        let mut content = Column::new()
            .cross_axis_alignment(Align::Start)
            .spacing(16.0)
            .child(Text::new(format!("Posting as user {AUTHOR_ID}.")).color(theme.muted))
            .child(title)
            .child(body)
            .child(send);

        content = match status {
            SendStatus::Idle | SendStatus::Sending => content,
            // The form stays filled in: the user fixes the connection and presses Send
            // again, without typing the post a second time.
            SendStatus::Failed(error) => content
                .child(Text::new(format!("Not sent. {}", error.message())).color(theme.error)),
            SendStatus::Sent(post) => content.child(CreatedPost::new(post).into_widget()).child(
                Button::new("Write another")
                    .variant(Variant::Outlined)
                    .on_press(cx.callback(NewPostState::reset)),
            ),
        };

        let app_bar = AppBar::new()
            .leading(back_button(cx))
            .title(Text::new("New post"))
            .build();

        Scaffold::new()
            .background(theme.background)
            .app_bar(app_bar)
            .body(
                SingleChildScrollView::new()
                    .padding_each(16.0, 16.0, 32.0, 16.0)
                    .child(content),
            )
            .build()
    }
}
