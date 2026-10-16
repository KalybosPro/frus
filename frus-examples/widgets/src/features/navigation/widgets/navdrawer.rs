//! `NavigationDrawer` — the content of a side panel for an app with many destinations:
//! a header, every destination with its full label (and a badge), dividers, a footer.
//! Put it in a `Drawer` (modal or docked) or let `NavScaffold` dock it on wide screens.

use frus::{
    Center, Color, Container, Divider, Drawer, Expanded, Icons, NavigationDrawer, Row, State,
    StateContext, StatefulWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(MailboxDemo.into_widget())
}

struct MailboxDemo;

#[derive(Default)]
struct MailboxDemoState {
    selected: usize,
}

const BOXES: [&str; 5] = ["Inbox", "Outbox", "Favourites", "Trash", "Spam"];

impl StatefulWidget for MailboxDemo {
    type State = MailboxDemoState;

    fn create_state(&self) -> MailboxDemoState {
        MailboxDemoState::default()
    }
}

impl State for MailboxDemoState {
    type Widget = MailboxDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // Destinations are counted in the order they are added; dividers and other
        // `child` widgets between them are not destinations and do not count.
        let drawer = NavigationDrawer::new(
            self.selected,
            cx.handler(|s: &mut Self, i: usize| s.selected = i),
        )
        .header(
            Container::new()
                .padding(16.0)
                .child(Text::new("Mail").size(18.0)),
        )
        .item(Icons::INBOX, "Inbox")
        .badge(24)
        .item(Icons::SEND, "Outbox")
        .item(Icons::FAVORITE_BORDER, "Favourites")
        .selected_icon(Icons::FAVORITE)
        .child(Divider::new())
        .item(Icons::DELETE, "Trash")
        .item(Icons::REPORT, "Spam")
        .disabled()
        .footer(
            Container::new()
                .padding(16.0)
                .child(Text::new("v1.0").size(12.0)),
        )
        .width(200.0);

        // Docked with a permanent `Drawer`, beside the page it navigates.
        let window = Container::new()
            .width(320.0)
            .height(440.0)
            .border(1.0, Color::hex("#A1A1AA"))
            .clip()
            .child(
                Drawer::new(true)
                    .permanent(true)
                    .width(200.0)
                    .panel(drawer)
                    .body(
                        Row::new()
                            .child(Expanded::new(Center::new(Text::new(BOXES[self.selected])))),
                    ),
            );

        sections(vec![Box::new(section("Docked beside the page", window))])
    }
}
