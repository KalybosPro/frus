//! `StickyHeader` — a section header that stays at the top of a scroll view while its
//! section is in view, and is pushed off by the next section's header.

use frus::{
    Align, Color, Column, Container, ListTile, SingleChildScrollView, StickyHeader, Text, Widget,
};

use crate::common::shared::{section, sections};

const CONTACTS: [(&str, &[&str]); 5] = [
    ("A", &["Ada", "Alan", "Alonzo", "Anita"]),
    ("B", &["Barbara", "Bjarne", "Brian"]),
    ("D", &["Dennis", "Donald", "Dorothy"]),
    ("G", &["Grace", "Guido"]),
    ("K", &["Ken", "Katherine", "Kristen"]),
];

pub(crate) fn example() -> Box<dyn Widget> {
    // Each section is `StickyHeader::new(header, content)`, one after another in a
    // column inside a vertical scroll view.
    let list = CONTACTS.iter().fold(
        Column::new().cross_axis_alignment(Align::Stretch),
        |column, (letter, names)| column.child(StickyHeader::new(header(letter), entries(names))),
    );

    sections(vec![Box::new(section(
        "Contacts (scroll inside the box)",
        Container::new().border(1.0, Color::hex("#A1A1AA")).child(
            SingleChildScrollView::new()
                .width(320.0)
                .height(300.0)
                .child(list),
        ),
    ))])
}

/// The header needs a background: the section scrolls under it.
fn header(letter: &str) -> Container {
    Container::new()
        .color(Color::hex("#E0E7FF"))
        .padding_each(6.0, 16.0, 6.0, 16.0)
        .child(Text::new(letter).color(Color::hex("#312E81")))
}

fn entries(names: &[&str]) -> Column {
    names.iter().fold(
        Column::new().cross_axis_alignment(Align::Stretch),
        |column, name| column.child(ListTile::new().title(*name)),
    )
}
