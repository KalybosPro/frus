use std::rc::Rc;

use frus::{
    Align, BuildContext, Button, Callback, Column, Container, Expanded, KeyboardType, Row,
    StatelessWidget, Text, TextField, Variant, Widget,
};

/// The panel under the app bar where the server's address is changed.
pub struct ServerSettings {
    url_draft: String,
    on_input: Rc<dyn Fn(String) -> Callback>,
    on_apply: Callback,
}

impl ServerSettings {
    pub fn new(
        url_draft: impl Into<String>,
        on_input: impl Fn(String) -> Callback + 'static,
        on_apply: Callback,
    ) -> Self {
        Self {
            url_draft: url_draft.into(),
            on_input: Rc::new(on_input),
            on_apply,
        }
    }
}

impl StatelessWidget for ServerSettings {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let on_input = self.on_input.clone();
        let field = TextField::new(self.url_draft.as_str())
            .label("Server address")
            .placeholder("wss://…")
            .keyboard_type(KeyboardType::Url)
            .on_input(move |text| on_input(text))
            .on_submit(self.on_apply.clone());

        Box::new(
            Container::new()
                .color(theme.scheme.surface_container_low)
                .padding(16.0)
                .child(
                    Column::new()
                        .spacing(8.0)
                        .cross_axis_alignment(Align::Start)
                        .child(
                            Row::new()
                                .spacing(8.0)
                                .cross_axis_alignment(Align::Center)
                                .child(Expanded::new(field))
                                .child(
                                    Button::new("Use")
                                        .variant(Variant::Tonal)
                                        .on_press(self.on_apply.clone()),
                                ),
                        )
                        .child(
                            Text::new(
                                "Any ws:// or wss:// server. A new address reconnects at once.",
                            )
                            .size(12.0)
                            .color(theme.muted),
                        ),
                ),
        )
    }
}
