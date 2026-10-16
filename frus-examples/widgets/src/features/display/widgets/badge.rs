//! `Badge` — the small mark that says "there is something here": a count on an inbox,
//! a dot on a bell.

use frus::{
    Badge, Button, Color, Colors, Icon, Icons, Positioned, Row, Stack, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let unread = cx.use_state(|| 3u32);

        // A badge is a standalone mark. To pin one to an icon's corner, lay both in a
        // `Stack` and place the badge with `Positioned`.
        let inbox = Stack::new()
            .width(36.0)
            .height(32.0)
            .layer(
                Positioned::new(Icon::new(Icons::MAIL))
                    .left(0.0)
                    .bottom(0.0),
            )
            .layer(
                Positioned::new(
                    // A count of zero hides the label and leaves a dot — or, as here, the
                    // application drops the badge altogether.
                    if unread.get() > 0 {
                        Box::new(Badge::new(count_label(unread.get()))) as Box<dyn Widget>
                    } else {
                        Box::new(Row::new())
                    },
                )
                .right(0.0)
                .top(0.0),
            );

        let more = unread.clone();
        let clear = unread.clone();

        sections(vec![
            // With a label it is a pill carrying a number or a word.
            Box::new(section(
                "Labelled",
                Row::new()
                    .spacing(12.0)
                    .child(Badge::new("1"))
                    .child(Badge::new("42"))
                    .child(Badge::new("999+"))
                    .child(Badge::new("NEW")),
            )),
            // Without one it is a dot: something happened, without saying how much.
            Box::new(section(
                "Dot",
                Row::new()
                    .spacing(12.0)
                    .child(Badge::dot())
                    .child(Badge::dot().small_size(10.0)),
            )),
            // Colours and sizes are the theme's by default and yours to override.
            Box::new(section(
                "Custom colours and size",
                Row::new()
                    .spacing(12.0)
                    .child(
                        Badge::new("PRO")
                            .background_color(Color::hex("#7C3AED"))
                            .text_color(Colors::WHITE),
                    )
                    .child(
                        Badge::new("12")
                            .background_color(Color::hex("#16A34A"))
                            .large_size(22.0)
                            .padding(8.0),
                    ),
            )),
            // A live count on an icon.
            Box::new(section(
                "On an icon",
                Row::new()
                    .spacing(16.0)
                    .child(inbox)
                    .child(
                        Button::new("New mail")
                            .variant(Variant::Tonal)
                            .on_press(move || more.update(|n| *n += 1)),
                    )
                    .child(
                        Button::new("Mark all read")
                            .variant(Variant::Text)
                            .on_press(move || clear.set(0)),
                    ),
            )),
            Box::new(Text::new(format!("{} unread", unread.get()))),
        ])
    }))
}

/// Counts above 99 are shown as "99+", so the pill stays small.
fn count_label(count: u32) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::count_label;

    #[test]
    fn large_counts_are_capped() {
        assert_eq!(count_label(7), "7");
        assert_eq!(count_label(99), "99");
        assert_eq!(count_label(100), "99+");
    }
}
