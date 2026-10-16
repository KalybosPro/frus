//! `CircleAvatar` — a circle that stands for a person: their picture, or failing that
//! their initials or an icon, on a colour.

use frus::{CircleAvatar, Color, Colors, Icon, Icons, Row, Widget, Wrap};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    // `asset!` embeds a file in the binary at compile time (the path is relative to this
    // source file), and decodes it once per process. `handle()` is the decoded picture,
    // or `None` if the bytes were not an image — the avatar then shows its colour.
    let photo = frus::asset!("../../../../res/mipmap-xxxhdpi/ic_launcher.png").handle();

    sections(vec![
        // `initials` takes a full name and shows the first letters of its first two words.
        Box::new(section(
            "Initials",
            Row::new()
                .spacing(12.0)
                .child(CircleAvatar::initials("Ada Lovelace"))
                .child(CircleAvatar::initials("Grace Hopper"))
                .child(CircleAvatar::initials("Linus")),
        )),
        // Any widget can sit in the circle; an icon is the usual anonymous placeholder.
        Box::new(section(
            "Icon and custom colours",
            Row::new()
                .spacing(12.0)
                .child(CircleAvatar::new().child(Icon::new(Icons::PERSON)))
                .child(
                    CircleAvatar::initials("Alan Turing")
                        .background_color(Color::hex("#0F766E"))
                        .foreground_color(Colors::WHITE),
                )
                .child(
                    CircleAvatar::new()
                        .background_color(Color::hex("#FDE68A"))
                        .foreground_color(Color::hex("#92400E"))
                        .child(Icon::new(Icons::PETS)),
                ),
        )),
        // A picture covers the circle. `foreground_image` is the photo; a background image
        // is the fallback shown under it.
        Box::new(section(
            "Picture",
            Row::new()
                .spacing(12.0)
                .child(CircleAvatar::new().foreground_image(photo.clone()))
                .child(CircleAvatar::new().radius(32.0).foreground_image(photo)),
        )),
        // `radius` (or `size`, the diameter) sets how big it is, whatever it holds.
        Box::new(section(
            "Sizes",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(CircleAvatar::initials("Ada Lovelace").radius(12.0))
                .child(CircleAvatar::initials("Ada Lovelace").radius(20.0))
                .child(CircleAvatar::initials("Ada Lovelace").size(56.0))
                .child(CircleAvatar::initials("Ada Lovelace").size(80.0)),
        )),
    ])
}
