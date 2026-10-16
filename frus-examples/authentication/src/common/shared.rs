//! Small pieces every auth page shares: the logo, the page frame, the submit button and
//! the server's error.

use frus::{
    Alert, Align, BorderRadius, BoxDecoration, BuildContext, Button, Callback, Center,
    CircularProgressIndicator, Column, ConstrainedBox, Container, FontWeight, Icon, Icons, Justify,
    Row, Scaffold, SingleChildScrollView, Text, Widget,
};

/// The application's logo: an icon in a rounded tile, and the name.
pub(crate) fn app_logo(cx: &BuildContext) -> Box<dyn Widget> {
    let theme = cx.theme();
    let tile = Container::new()
        .width(40.0)
        .height(40.0)
        .decoration(BoxDecoration::filled(theme.primary).radius(BorderRadius::uniform(12.0)))
        .child(Center::new(
            Icon::new(Icons::LOCK_OUTLINE)
                .size(22.0)
                .color(theme.on_primary),
        ));
    Box::new(
        Row::new()
            .spacing(10.0)
            .cross_axis_alignment(Align::Center)
            .child(tile)
            .child(
                Text::new("frus auth")
                    .size(20.0)
                    .weight(FontWeight::SemiBold)
                    .color(theme.on_surface),
            ),
    )
}

/// The frame of an auth page: the theme's background, a scrollable body (a phone's
/// keyboard covers half the screen), and a column no wider than a form should be.
pub(crate) fn auth_frame(cx: &BuildContext, content: Column) -> Box<dyn Widget> {
    let body = SingleChildScrollView::new()
        .padding_each(32.0, 20.0, 32.0, 20.0)
        .child(Center::new(
            ConstrainedBox::new(content.cross_axis_alignment(Align::Stretch)).max_width(420.0),
        ));
    Scaffold::new()
        // Without a background the Scaffold is transparent and the shell shows through.
        .background(cx.theme().background)
        .body(body)
        .build()
}

/// A page title and the sentence under it.
pub(crate) fn heading(cx: &BuildContext, title: &str, subtitle: &str) -> Box<dyn Widget> {
    Box::new(
        Column::new()
            .spacing(6.0)
            .cross_axis_alignment(Align::Start)
            .child(
                Text::new(title)
                    .size(28.0)
                    .weight(FontWeight::SemiBold)
                    .color(cx.theme().on_surface),
            )
            .child(
                Text::new(subtitle)
                    .size(14.0)
                    .color(cx.theme().muted)
                    .wrap(),
            ),
    )
}

/// The form's main button, as wide as the form. While the request is on its way it shows
/// a spinner and the `busy` label, and is disabled — a second press would send a second
/// request.
pub(crate) fn submit_button(
    cx: &BuildContext,
    label: &str,
    busy: &str,
    loading: bool,
    on_press: Callback,
) -> Box<dyn Widget> {
    let content: Box<dyn Widget> = if loading {
        Box::new(
            Row::new()
                .spacing(12.0)
                .main_axis_alignment(Justify::Center)
                .cross_axis_alignment(Align::Center)
                .child(
                    CircularProgressIndicator::new()
                        .size(18.0)
                        .stroke_width(2.0)
                        .color(cx.theme().muted),
                )
                .child(Text::new(busy)),
        )
    } else {
        Box::new(Text::new(label))
    };
    // `with_child` rather than `Button::new(label)`: a button made from a label sizes
    // itself to that label, while one holding a child takes the width the column gives it.
    Box::new(
        Button::with_child(content)
            .height(48.0)
            .enabled(!loading)
            .on_press(on_press),
    )
}

/// What the server said when it refused, above the form — or nothing.
pub(crate) fn server_error(message: Option<&str>) -> Option<Box<dyn Widget>> {
    message.map(|message| Box::new(Alert::new(message).error()) as Box<dyn Widget>)
}
