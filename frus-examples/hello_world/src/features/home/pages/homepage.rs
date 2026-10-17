use frus::{
    Align, AppBar, BuildContext, Center, Column, FontWeight, Justify, Scaffold, StatelessWidget,
    Text, TextAlign, Widget,
};

use crate::common::colors::{primary_color, text_muted};

/// The only page: a `StatelessWidget`, because it has nothing to remember.
pub struct HomePage;

impl StatelessWidget for HomePage {
    /// `build` says what the page is made of. It runs again whenever the framework needs
    /// the page — a resize, a theme change — so it must only describe, never change.
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let greeting = Column::new()
            .spacing(12.0)
            .main_axis_alignment(Justify::Center)
            .cross_axis_alignment(Align::Center)
            .child(
                Text::new("Hello, world!")
                    .size(36.0)
                    .weight(FontWeight::Bold)
                    .color(primary_color()),
            )
            .child(
                Text::new("The same code runs on the desktop, Android and the Web.")
                    .size(16.0)
                    .color(text_muted())
                    .align(TextAlign::Center),
            );

        // `Scaffold` lays out a page: an app bar on top, the body below it, and the safe
        // area (notches, system bars) taken care of.
        Scaffold::new()
            // Unset, a scaffold is transparent: the page paints the theme's background.
            .background(cx.theme().background)
            .app_bar(AppBar::new().title(Text::new("hello_world")).build())
            .body(Center::new(greeting))
            .build()
    }
}
