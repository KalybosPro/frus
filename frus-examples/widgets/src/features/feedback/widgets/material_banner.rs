//! `MaterialBanner` — a message across the top of the content that stays until one of its
//! actions is taken. Between a snack bar (gone in seconds) and a dialog (blocks
//! everything): for something important that can wait, like "you are offline".

use frus::{
    Align, Button, Color, Column, Icon, Icons, MaterialBanner, State, StateContext, StatefulWidget,
    Text, TextStyle, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(BannerDemo.into_widget())
}

struct BannerDemo;

struct BannerDemoState {
    offline: bool,
    update: bool,
}

impl StatefulWidget for BannerDemo {
    type State = BannerDemoState;

    fn create_state(&self) -> BannerDemoState {
        BannerDemoState {
            offline: true,
            update: true,
        }
    }
}

impl State for BannerDemoState {
    type Widget = BannerDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // A banner is in the flow, not over it: showing it is simply building it.
        let offline: Box<dyn Widget> = if self.offline {
            MaterialBanner::new("You are offline. Changes will sync when you reconnect.")
                .leading(Icon::new(Icons::CLOUD_OFF))
                .action(
                    Button::new("Dismiss")
                        .variant(Variant::Text)
                        .on_press(cx.callback(|s: &mut BannerDemoState| s.offline = false)),
                )
                .action(
                    Button::new("Retry")
                        .variant(Variant::Text)
                        .on_press(cx.callback(|s: &mut BannerDemoState| s.offline = false)),
                )
                .build()
        } else {
            Box::new(
                Button::new("Show again")
                    .variant(Variant::Outlined)
                    .on_press(cx.callback(|s: &mut BannerDemoState| s.offline = true)),
            )
        };

        // Colours, padding and where the actions go are yours to change.
        let update: Box<dyn Widget> = if self.update {
            MaterialBanner::new("A new version is ready.")
                .leading(Icon::new(Icons::DOWNLOAD).color(Color::hex("#065F46")))
                .background(Color::hex("#D1FAE5"))
                .content_text_style(TextStyle::new(14.0).color(Color::hex("#064E3B")))
                .force_actions_below(true)
                .action(
                    Button::new("Later")
                        .variant(Variant::Text)
                        .on_press(cx.callback(|s: &mut BannerDemoState| s.update = false)),
                )
                .action(
                    Button::new("Update")
                        .on_press(cx.callback(|s: &mut BannerDemoState| s.update = false)),
                )
                .build()
        } else {
            Box::new(
                Button::new("Show again")
                    .variant(Variant::Outlined)
                    .on_press(cx.callback(|s: &mut BannerDemoState| s.update = true)),
            )
        };

        sections(vec![
            Box::new(section("Actions beside the message", offline)),
            Box::new(section(
                "Actions below, custom colours",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Stretch)
                    .child_boxed(update)
                    .child(Text::new("The content goes on under the banner.")),
            )),
        ])
    }
}
