//! `AppBar` — the bar at the top of a screen: a leading button, a title, actions, and
//! optionally a widget under it (tabs, a search field). Actions can fold into an
//! overflow menu when the bar runs out of width.

use frus::{
    AppBar, Badge, Color, Colors, Column, Container, IconButton, Icons, TabBar, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let last = cx.use_state(|| "Nothing yet".to_string());
        let overflow_open = cx.use_state(|| false);
        let tab = cx.use_state(|| 0usize);

        // Every handler here records what was pressed, so the demo shows it.
        let said = |what: &'static str| {
            let last = last.clone();
            move || last.set(what.to_string())
        };

        // `primary(false)` because these bars sit inside a page: a primary bar is the one
        // at the very top of the screen, and makes room for the status bar on a phone.
        let standard = AppBar::new()
            .primary(false)
            .leading(
                IconButton::new(Icons::MENU)
                    .label("Menu")
                    .on_press(said("Menu")),
            )
            .title(Text::new("Inbox"))
            .action(
                IconButton::new(Icons::SEARCH)
                    .label("Search")
                    .on_press(said("Search")),
            )
            .action(Badge::new("3"))
            .build();

        // A centred title, and custom colours and depth.
        let centred = AppBar::new()
            .primary(false)
            .center_title(true)
            .background(Color::hex("#312E81"))
            .foreground(Colors::WHITE)
            .elevation(4.0)
            .leading(
                IconButton::new(Icons::ARROW_BACK)
                    .label("Back")
                    .on_press(said("Back")),
            )
            .title(Text::new("Profile"))
            .build();

        // Foldable actions are text buttons shown inline while they fit, and moved into
        // the `⋯` menu when the bar is too narrow. The menu's open state is the app's.
        let toggle = overflow_open.clone();
        let close_after = |what: &'static str| {
            let last = last.clone();
            let open = overflow_open.clone();
            frus::Callback::new(move || {
                last.set(what.to_string());
                open.set(false);
            })
        };
        let folding = AppBar::new()
            .primary(false)
            // The folding budget is the bar's width; a bar outside a full-width
            // screen says how wide it is.
            .width(320.0)
            .title(Text::new("Report"))
            .overflow(
                overflow_open.get(),
                frus::Callback::new(move || toggle.update(|o| *o = !*o)),
            )
            .foldable_action("Export", close_after("Export"))
            .foldable_action("Share", close_after("Share"))
            .foldable_action("Print", close_after("Print"))
            .foldable_action("Archive", close_after("Archive"))
            .build();

        // `bottom` puts a widget under the bar, inside its background — usually tabs.
        let pick = tab.clone();
        let tabs = TabBar::new(tab.get(), move |i: usize| {
            let pick = pick.clone();
            frus::Callback::new(move || pick.set(i))
        })
        .tab("Mail", Column::new())
        .tab("Chat", Column::new())
        .tab("Meet", Column::new());
        let with_bottom = AppBar::new()
            .primary(false)
            .title(Text::new("Messages"))
            .bottom(tabs)
            .build();

        sections(vec![
            Box::new(section("Leading, title and actions", framed(standard))),
            Box::new(section("Centred title, custom colours", framed(centred))),
            Box::new(section("Foldable actions", framed(folding))),
            Box::new(section("With a bottom", framed(with_bottom))),
            Box::new(Text::new(format!("Last pressed: {}", last.get())).color(cx.theme().muted)),
        ])
    }))
}

/// A bar is as wide as its screen; here the "screen" is a 320-pixel frame.
fn framed(bar: Box<dyn Widget>) -> Container {
    Container::new().width(320.0).child(bar)
}
