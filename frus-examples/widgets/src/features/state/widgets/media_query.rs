//! `MediaQuery` — what the screen is like: its size, density, the system's insets (status
//! bar, notch, keyboard), the reader's text size, light or dark, orientation and size
//! class. `MediaScope` changes part of it for a subtree.
//!
//! Resize the window: the values below follow.

use frus::{component, Align, Column, Container, Insets, MediaQuery, MediaScope, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        // The description in force where this is built. Read it in `build`, never cache
        // it: a resize or a rotation rebuilds, and it is read afresh.
        let media = MediaQuery::of();
        let scheme = &cx.theme().scheme;

        let insets = |i: Insets| {
            format!(
                "top {:.0}, right {:.0}, bottom {:.0}, left {:.0}",
                i.top, i.right, i.bottom, i.left
            )
        };
        let lines = [
            (
                "size",
                format!(
                    "{:.0} × {:.0} logical px",
                    media.size.width, media.size.height
                ),
            ),
            (
                "device_pixel_ratio",
                format!("{:.2}", media.device_pixel_ratio),
            ),
            ("orientation()", format!("{:?}", media.orientation())),
            ("size_class()", format!("{:?}", media.size_class())),
            ("padding (system bars)", insets(media.padding)),
            ("view_insets (keyboard)", insets(media.view_insets)),
            ("safe()", insets(media.safe())),
            ("text_scaler", format!("{:.2}", media.text_scaler)),
            (
                "platform_brightness",
                format!("{:?}", media.platform_brightness),
            ),
            (
                "accessibility.disable_animations",
                format!("{}", media.accessibility.disable_animations),
            ),
        ];
        let table = lines.into_iter().fold(
            Column::new()
                .spacing(6.0)
                .cross_axis_alignment(Align::Start),
            |column, (name, value)| {
                column.child(
                    Column::new()
                        .cross_axis_alignment(Align::Start)
                        .child(Text::new(name).size(12.0).color(scheme.on_surface_variant))
                        .child(Text::new(value)),
                )
            },
        );

        // A subtree that believes the reader asked for bigger text.
        let scaled = MediaScope::tweak(
            |media| media.text_scaler = 1.5,
            Container::new()
                .padding(12.0)
                .radius(8.0)
                .color(scheme.surface_container)
                .child(Text::new("Laid out at a text scale of 1.5")),
        );

        sections(vec![
            Box::new(section("MediaQuery::of()", table)),
            Box::new(section("MediaScope::tweak", scaled)),
        ])
    }))
}
