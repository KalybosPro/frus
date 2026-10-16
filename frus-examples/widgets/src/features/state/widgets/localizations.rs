//! `Locale` and `Localizations` — the language the interface is in, and the words the
//! framework itself says in it: button labels a screen reader announces, month names in a
//! date picker, "1–10 of 100" under a table, digit grouping.
//!
//! The application states which languages it has and, optionally, the one the reader
//! picked; `locale::of()` and `localizations::of()` answer from anywhere. Your own
//! strings come from your own translation files, chosen by `locale::of()`.

use std::rc::Rc;

use frus::{
    component, host, locale, localizations, Align, Callback, Column, English, French, Locale, Row,
    SegmentedButton, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let scheme = &cx.theme().scheme;
        let current = locale::of();
        let french = current.language_code() == "fr";

        // Switching changes the whole application, on the next frame.
        let picker = SegmentedButton::new(usize::from(french), |index| {
            Callback::new(move || {
                let app = host::app();
                app.set_supported_locales(vec![Locale::new("en"), Locale::new("fr")]);
                if index == 1 {
                    app.set_locale(Some(Locale::new("fr")));
                    app.set_localizations(Some(Rc::new(French)));
                } else {
                    app.set_locale(Some(Locale::new("en")));
                    app.set_localizations(Some(Rc::new(English)));
                }
            })
        })
        .segment("English")
        .segment("Français");

        // The table in force. It always answers: English when nothing is installed.
        let words = localizations::of();
        let months = words.months();
        let lines = [
            ("locale::of()", current.to_string()),
            (
                "ok / cancel",
                format!(
                    "{} / {}",
                    words.ok_button_label(),
                    words.cancel_button_label()
                ),
            ),
            ("back button", words.back_button_label().to_string()),
            ("first months", months[..3].join(", ")),
            ("narrow weekdays", words.narrow_weekdays().join(" ")),
            (
                "page_range_label(1, 10, 1234)",
                words.page_range_label(1, 10, 1234),
            ),
            ("group_digits(1234567)", words.group_digits(1_234_567)),
        ];
        let table = lines.into_iter().fold(
            Column::new()
                .spacing(6.0)
                .cross_axis_alignment(Align::Start),
            |column, (name, value)| {
                column.child(
                    Row::new()
                        .spacing(8.0)
                        .child(Text::new(name).size(12.0).color(scheme.on_surface_variant))
                        .child(Text::new(value)),
                )
            },
        );

        sections(vec![
            Box::new(section("The application's language", picker)),
            Box::new(section("What the framework says", table)),
        ])
    }))
}
