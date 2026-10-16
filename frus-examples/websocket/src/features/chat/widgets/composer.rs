use std::rc::Rc;

use frus::{
    Align, BuildContext, Callback, Container, Expanded, IconButton, Icons, Row, StatelessWidget,
    TextField, TextInputAction, Widget,
};

/// The bottom bar: a field for the message and a Send button. Enter sends too.
///
/// It holds no text of its own: the draft lives in the page's state, and the widget is
/// handed what to show and what to call — the page decides, the widget only draws.
pub(crate) struct Composer {
    draft: String,
    /// Whether there is a connection to type for.
    enabled: bool,
    /// Whether Send would send anything right now.
    can_send: bool,
    on_input: Rc<dyn Fn(String) -> Callback>,
    on_send: Callback,
}

impl Composer {
    pub(crate) fn new(
        draft: impl Into<String>,
        enabled: bool,
        can_send: bool,
        on_input: impl Fn(String) -> Callback + 'static,
        on_send: Callback,
    ) -> Self {
        Self {
            draft: draft.into(),
            enabled,
            can_send,
            on_input: Rc::new(on_input),
            on_send,
        }
    }
}

impl StatelessWidget for Composer {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let on_input = self.on_input.clone();
        let field = TextField::new(self.draft.as_str())
            .placeholder(if self.enabled {
                "Type a message"
            } else {
                "Connect to send messages"
            })
            .enabled(self.enabled)
            // On a phone the keyboard shows a Send key, which submits like Enter.
            .action(TextInputAction::Send)
            .on_input(move |text| on_input(text))
            .on_submit(self.on_send.clone());

        let send = IconButton::new(Icons::SEND)
            .label("Send")
            .enabled(self.can_send)
            .on_press(self.on_send.clone());

        Box::new(
            Container::new()
                .color(cx.theme().surface)
                .padding(12.0)
                .child(
                    Row::new()
                        .spacing(8.0)
                        .cross_axis_alignment(Align::Center)
                        .child(Expanded::new(field))
                        .child(send),
                ),
        )
    }
}
