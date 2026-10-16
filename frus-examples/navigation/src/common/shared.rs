//! Small pieces several pages use.

use frus::{
    Align, AppBar, BackButton, BuildContext, Callback, Center, Column, FontWeight, Icon, IconData,
    Text, TextAlign,
};

/// `1234` cents → `$12.34`.
pub(crate) fn format_price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// The app bar of a page that is **not** a section: a title, and a back button whenever
/// the router has a page underneath to go back to.
///
/// The router owns the stack, so it is the one to ask: `can_pop()` is true for a page
/// pushed on top of another, and for a nested location (`/products/3` has home beneath
/// it) even when the app was opened at that address. The button just pops. Returned
/// unbuilt so a page can add actions before `.build()`.
pub(crate) fn page_app_bar(cx: &BuildContext, title: impl Into<String>) -> AppBar {
    let router = cx.router();
    let bar = AppBar::new().title(Text::new(title.into()));
    if router.can_pop() {
        bar.leading(BackButton::new().on_press(Callback::new(move || {
            router.pop();
        })))
    } else {
        bar
    }
}

/// A centred icon, a line and an optional action: an empty cart, no search results, a
/// page that does not exist.
pub(crate) fn empty_state(
    cx: &BuildContext,
    icon: IconData,
    message: impl Into<String>,
    action: Option<frus::Button>,
) -> Center {
    let theme = cx.theme();
    let mut column = Column::new()
        .spacing(16.0)
        .cross_axis_alignment(Align::Center)
        .child(Icon::new(icon).size(56.0).color(theme.muted))
        .child(
            Text::new(message.into())
                .size(16.0)
                .weight(FontWeight::Medium)
                .align(TextAlign::Center),
        );
    if let Some(action) = action {
        column = column.child(action);
    }
    Center::new(column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_are_written_in_dollars_and_cents() {
        assert_eq!(format_price(0), "$0.00");
        assert_eq!(format_price(5), "$0.05");
        assert_eq!(format_price(8900), "$89.00");
        assert_eq!(format_price(123456), "$1234.56");
    }
}
