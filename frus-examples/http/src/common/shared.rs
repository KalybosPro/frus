//! Small widgets every page uses: the loading view, the error view, the back button.

use frus::{
    Align, BackButton, BuildContext, Button, Callback, Center, CircularProgressIndicator, Column,
    FontWeight, Icon, Icons, Padding, StatelessWidget, Text, TextAlign, Variant, Widget,
};

use crate::common::api::ApiError;

/// A spinner and what is being waited for.
pub(crate) struct LoadingView {
    label: &'static str,
}

impl LoadingView {
    pub(crate) fn new(label: &'static str) -> Self {
        Self { label }
    }
}

impl StatelessWidget for LoadingView {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        Box::new(Center::new(
            Column::new()
                .spacing(16.0)
                .cross_axis_alignment(Align::Center)
                .child(CircularProgressIndicator::new())
                .child(Text::new(self.label).color(cx.theme().muted)),
        ))
    }
}

/// Why a call failed, in words the user understands, and a way to try again.
///
/// Every failure gets one: a screen that only ever shows a spinner when the network is
/// gone leaves its user waiting for something that is not coming.
pub(crate) struct ErrorView {
    error: ApiError,
    on_retry: Callback,
}

impl ErrorView {
    pub(crate) fn new(error: ApiError, on_retry: Callback) -> Self {
        Self { error, on_retry }
    }
}

impl StatelessWidget for ErrorView {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let icon = match self.error {
            ApiError::Offline => Icons::WIFI_OFF,
            ApiError::Timeout => Icons::TIMER,
            _ => Icons::ERROR_OUTLINE,
        };
        Box::new(Center::new(Padding::new(
            24.0,
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                .child(Icon::new(icon).size(48.0).color(theme.error))
                .child(
                    Text::new(self.error.title())
                        .size(18.0)
                        .weight(FontWeight::SemiBold)
                        .align(TextAlign::Center),
                )
                .child(
                    Text::new(self.error.message())
                        .color(theme.muted)
                        .align(TextAlign::Center),
                )
                .child(
                    Button::new("Retry")
                        .icon(Icon::new(Icons::REFRESH))
                        .variant(Variant::Tonal)
                        .on_press(self.on_retry.clone()),
                ),
        )))
    }
}

/// The app bar's back arrow: it pops the router's stack.
///
/// The bar does not add one by itself — the router, not the bar, knows whether there is a
/// page to go back to — so a page that can be left says so.
pub(crate) fn back_button(cx: &BuildContext) -> BackButton {
    let router = cx.router();
    BackButton::new().on_press(Callback::new(move || {
        router.pop();
    }))
}
