//! Small pieces both pages use: the centred column a page's content sits in, and the
//! error line under a control that is not a text field.

use frus::{
    Align, BuildContext, Center, Column, ConstrainedBox, MediaQuery, SingleChildScrollView, Text,
    Widget,
};

/// The widest the content gets. A form stretched across a desktop monitor is hard to read:
/// past this width the column stops growing and stays centred.
pub(crate) const MAX_CONTENT_WIDTH: f32 = 560.0;

/// The room on either side of the content on a phone.
pub(crate) const GUTTER: f32 = 16.0;

/// How wide the content column comes out: the window minus the gutters, up to
/// [`MAX_CONTENT_WIDTH`]. For the few widgets that take a width in pixels rather than the
/// room on offer (a dropdown menu sizes its list to it).
pub(crate) fn content_width() -> f32 {
    (MediaQuery::of().size.width - 2.0 * GUTTER).clamp(0.0, MAX_CONTENT_WIDTH)
}

/// A scrolling page whose content is a centred column, as wide as the window on a phone and
/// never wider than [`MAX_CONTENT_WIDTH`] on a desktop.
///
/// `Center` centres; the `ConstrainedBox` caps. A field inside takes the width on offer, so
/// the fields follow the column without being told a width each.
pub(crate) fn centered_page(content: impl Widget + 'static) -> SingleChildScrollView {
    SingleChildScrollView::new()
        .padding_each(24.0, GUTTER, 32.0, GUTTER)
        .child(Center::new(
            ConstrainedBox::new(content).max_width(MAX_CONTENT_WIDTH),
        ))
}

/// The error line under a control that has no `error(..)` of its own — a dropdown, a group
/// of chips, a checkbox — in the colour and size a text field uses for its own.
pub(crate) fn control_error(cx: &BuildContext, message: &str) -> Text {
    Text::styled(message, cx.theme().text.body_small).color(cx.theme().error)
}

/// A control with the error line under it when there is one.
pub(crate) fn with_error(
    cx: &BuildContext,
    control: impl Widget + 'static,
    error: Option<&str>,
) -> Box<dyn Widget> {
    match error {
        None => Box::new(control),
        Some(message) => Box::new(
            Column::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Stretch)
                .child(control)
                .child(control_error(cx, message)),
        ),
    }
}
