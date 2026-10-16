//! `IconButton` — a square button holding one icon, for compact actions in toolbars,
//! list rows and app bars.

use frus::{
    Color, IconButton, IconButtonVariant, Icons, Row, State, StateContext, StatefulWidget, Text,
    Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(IconButtonDemo.into_widget())
}

struct IconButtonDemo;

struct IconButtonDemoState {
    favorite: bool,
    bookmarked: bool,
    likes: u32,
}

impl StatefulWidget for IconButtonDemo {
    type State = IconButtonDemoState;

    fn create_state(&self) -> IconButtonDemoState {
        IconButtonDemoState {
            favorite: false,
            bookmarked: true,
            likes: 0,
        }
    }
}

impl State for IconButtonDemoState {
    type Widget = IconButtonDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let like = || cx.callback(|s: &mut IconButtonDemoState| s.likes += 1);

        sections(vec![
            // `label` is what a screen reader announces and what the tooltip says: an
            // icon alone means nothing to someone who cannot see it, so always give one.
            Box::new(section(
                "Variants",
                Wrap::new()
                    .gap(8.0)
                    .run_gap(8.0)
                    .child(
                        IconButton::new(Icons::THUMB_UP)
                            .label("Like")
                            .on_press(like()),
                    )
                    .child(
                        IconButton::new(Icons::THUMB_UP)
                            .label("Like")
                            .variant(IconButtonVariant::Filled)
                            .on_press(like()),
                    )
                    .child(
                        IconButton::new(Icons::THUMB_UP)
                            .label("Like")
                            .variant(IconButtonVariant::Tonal)
                            .on_press(like()),
                    )
                    .child(
                        IconButton::new(Icons::THUMB_UP)
                            .label("Like")
                            .variant(IconButtonVariant::Outlined)
                            .on_press(like()),
                    ),
            )),
            Box::new(Text::new(format!("Liked {} times", self.likes))),
            // A toggle: `selected` swaps to the filled look. The state is yours — the
            // button only shows it — so the press flips a field.
            Box::new(section(
                "Selected (toggle)",
                Row::new()
                    .spacing(8.0)
                    .child(
                        IconButton::new(if self.favorite {
                            Icons::FAVORITE
                        } else {
                            Icons::FAVORITE_BORDER
                        })
                        .label("Favourite")
                        .selected(self.favorite)
                        .on_press(
                            cx.callback(|s: &mut IconButtonDemoState| s.favorite = !s.favorite),
                        ),
                    )
                    .child(
                        IconButton::new(if self.bookmarked {
                            Icons::BOOKMARK
                        } else {
                            Icons::BOOKMARK_BORDER
                        })
                        .label("Bookmark")
                        .variant(IconButtonVariant::Filled)
                        .selected(self.bookmarked)
                        .on_press(
                            cx.callback(|s: &mut IconButtonDemoState| s.bookmarked = !s.bookmarked),
                        ),
                    ),
            )),
            // Disabled: no press to answer, greyed out.
            Box::new(section(
                "Disabled",
                Row::new()
                    .spacing(8.0)
                    .child(
                        IconButton::new(Icons::DELETE)
                            .label("Delete")
                            .enabled(false),
                    )
                    .child(
                        IconButton::new(Icons::DELETE)
                            .label("Delete")
                            .variant(IconButtonVariant::Filled)
                            .enabled(false),
                    ),
            )),
            // Size, icon size, colours and corners are all yours to change.
            Box::new(section(
                "Custom size and colours",
                Row::new()
                    .spacing(8.0)
                    .child(
                        IconButton::new(Icons::SHARE)
                            .label("Share")
                            .size(56.0)
                            .icon_size(32.0)
                            .color(Color::hex("#DCFCE7"))
                            .icon_color(Color::hex("#166534"))
                            .radius(12.0)
                            .on_press(like()),
                    )
                    // A glyph works where the icon set has nothing that fits.
                    .child(IconButton::glyph("½").label("Half").on_press(like())),
            )),
        ])
    }
}
