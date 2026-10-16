use frus::{
    Align, AppBar, Button, Center, Column, Container, Expanded, IconButton, Icons, Row, Scaffold,
    SingleChildScrollView, State, StateContext, StatefulWidget, StatelessWidget, Text, Variant,
    Widget,
};

use crate::features::chat::controllers::chat_controller::connect;
use crate::features::chat::states::chatstate::{ChatState, Status};
use crate::features::chat::widgets::{
    composer::Composer, message_bubble::MessageBubble, server_settings::ServerSettings,
    status_chip::StatusChip,
};
use crate::models::message::TimeOfDay;

/// The chat page's configuration: the server it connects to first.
#[derive(Clone)]
pub(crate) struct ChatPage {
    url: String,
}

impl ChatPage {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl StatefulWidget for ChatPage {
    type State = ChatState;

    fn create_state(&self) -> ChatState {
        ChatState::new(self.url.as_str())
    }
}

impl State for ChatState {
    type Widget = ChatPage;

    /// The connection lives as long as the page: it opens here, once, when the page
    /// appears…
    fn init_state(&mut self, cx: &StateContext<Self>) {
        // `init_state` already holds the state mutably, so it is passed in directly; the
        // handle is for the events that will arrive later.
        connect(self, &cx.handle());
    }

    /// …and closes here, when it goes away. Without this the socket thread would keep
    /// the connection open for a page nobody can see.
    fn dispose(&mut self) {
        self.close_connection();
    }

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();

        let app_bar = AppBar::new()
            .title(Text::new("Echo chat"))
            .action(
                IconButton::new(Icons::DELETE_SWEEP)
                    .label("Clear the conversation")
                    .enabled(!self.messages.is_empty())
                    .on_press(cx.callback(ChatState::clear_messages)),
            )
            .action(
                IconButton::new(Icons::SETTINGS)
                    .label("Server")
                    .selected(self.settings_open)
                    .on_press(cx.callback(ChatState::toggle_settings)),
            );

        // On the Web, the browser delivers the socket's events between frames. Waiting on
        // the connection (a future that ends with it) keeps frames coming, so each event
        // shows at the next one. `use_future` starts it again whenever the session changes.
        #[cfg(target_arch = "wasm32")]
        cx.use_future(self.session, || self.watch());

        let body = self.chat(cx);

        Scaffold::new()
            .background(theme.background)
            .app_bar(app_bar.build())
            .body(body)
            .build()
    }
}

impl ChatState {
    /// The page itself: settings (when open), the status strip, the conversation, the
    /// composer.
    fn chat(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let mut page = Column::new();

        if self.settings_open {
            // Applying a new address reconnects. The callback needs the state's handle to
            // pass on to the connection, so it captures one.
            let handle = cx.handle();
            let apply = cx.callback(move |state: &mut ChatState| {
                if state.apply_url() {
                    connect(state, &handle);
                }
            });
            page = page.child(
                ServerSettings::new(
                    self.url_draft.as_str(),
                    cx.handler(ChatState::set_url_draft),
                    apply,
                )
                .into_widget(),
            );
        }

        page = page
            .child(self.status_strip(cx))
            // `Expanded`: the conversation takes whatever height the strip and the
            // composer leave.
            .child(Expanded::new(self.conversation(cx)))
            .child(
                Composer::new(
                    self.draft.as_str(),
                    self.status == Status::Connected,
                    self.can_send(),
                    cx.handler(ChatState::set_draft),
                    cx.callback(|state: &mut ChatState| state.send_draft(TimeOfDay::now())),
                )
                .into_widget(),
            );

        Box::new(page)
    }

    /// The chip, a line saying what is going on, and the button that changes it.
    fn status_strip(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();

        let detail = match &self.status {
            Status::Connecting => format!("Connecting to {}…", self.url),
            Status::Connected => self.url.clone(),
            Status::Closed => "Not connected".to_string(),
            // The error itself is in the conversation; here, what happens next.
            Status::Failed {
                retry_in: Some(wait),
                ..
            } => format!(
                "Retrying in {} s (attempt {})",
                wait.as_secs(),
                self.attempt + 1
            ),
            Status::Failed {
                error,
                retry_in: None,
            } => error.clone(),
        };

        let handle = cx.handle();
        let connect_now = cx.callback(move |state: &mut ChatState| connect(state, &handle));
        let action = match self.status {
            Status::Connecting | Status::Connected => Button::new("Disconnect")
                .variant(Variant::Outlined)
                .on_press(cx.callback(|state: &mut ChatState| state.disconnect(TimeOfDay::now()))),
            Status::Failed {
                retry_in: Some(_), ..
            } => Button::new("Retry now")
                .variant(Variant::Tonal)
                .on_press(connect_now),
            _ => Button::new("Connect").on_press(connect_now),
        };

        Box::new(
            Container::new()
                .padding_each(8.0, 16.0, 8.0, 16.0)
                .color(theme.surface)
                .child(
                    Row::new()
                        .spacing(12.0)
                        .cross_axis_alignment(Align::Center)
                        .child(StatusChip::new(self.status.clone()).into_widget())
                        .child(Expanded::new(
                            Text::new(detail)
                                .size(13.0)
                                .color(theme.muted)
                                .max_lines(2)
                                .ellipsis(),
                        ))
                        .child(action),
                ),
        )
    }

    /// The messages, newest at the bottom.
    fn conversation(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        if self.messages.is_empty() {
            return Box::new(Center::new(
                Text::new("No messages yet")
                    .size(14.0)
                    .color(cx.theme().muted),
            ));
        }

        let bubbles = self
            .messages
            .iter()
            .fold(Column::new().spacing(8.0), |column, message| {
                column.child(MessageBubble::new(message.clone()).into_widget())
            });

        // `reverse()` anchors the content to the bottom and keeps it there as messages
        // arrive — what a conversation wants, with no scrolling by hand after each one.
        Box::new(
            SingleChildScrollView::new()
                .reverse()
                .padding(16.0)
                .child(bubbles),
        )
    }
}
