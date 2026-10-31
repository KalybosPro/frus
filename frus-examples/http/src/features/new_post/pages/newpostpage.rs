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

        let status = self.status.clone();
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
            // A request the user starts, written as what happens, in order: the errors show,
            // the form locks, the post goes out, and the answer is shown. The handler runs on
            // the interface's thread and waits for the server without holding it; if the
            // page is left before the answer comes, the handler ends with it.
            Button::new("Send")
                .icon(Icon::new(Icons::SEND))
                .on_press(cx.async_callback(|state| async move {
                    state.set_state(|s| s.submitted = true);
                    let Some(draft) = state.read(NewPostState::draft) else {
                        return;
                    };
                    state.set_state(|s| s.status = SendStatus::Sending);
                    let answer = api::create_post(draft).await;
                    state.set_state(|s| s.finish(answer));
                }))
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
