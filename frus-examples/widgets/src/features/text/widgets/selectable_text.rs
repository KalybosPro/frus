//! `Text::selectable` and `SelectionArea` — text the reader can select and copy: one
//! text on its own, or a selection that runs across several.
//!
//! Text is not selectable by default: a label on a button should take the tap, not
//! start a selection. Make selectable what a reader may want to copy — an order
//! number, an address, an error message, an article.

use frus::{Align, Color, Column, Container, FontFamily, SelectionArea, Text, ToolbarItem, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // One selectable text: drag across it, double-click a word, or press Ctrl+A
        // once it has focus, then Ctrl+C. On a touch screen, long-press.
        Box::new(section(
            "One selectable text",
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(6.0)
                .child(Text::new("Your order number:"))
                .child(
                    Text::new("ORDER-4417-XK")
                        .family(FontFamily::Monospace)
                        .size(18.0)
                        .selectable(),
                ),
        )),
        // `SelectionArea` wraps a subtree, and the selection runs from one text into the
        // next, in reading order — select from the title into the second paragraph.
        Box::new(section(
            "A selection across several texts",
            Container::new()
                .width(320.0)
                .padding(12.0)
                .border(1.0, Color::hex("#CBD5E1"))
                .child(SelectionArea::around(
                    Column::new()
                        .cross_axis_alignment(Align::Start)
                        .spacing(8.0)
                        .child(Text::new("Release notes").size(18.0))
                        .child(Text::new(
                            "The first paragraph of an article. A selection can start here…",
                        ))
                        .child(Text::new("…and end in this one, as on a web page.")),
                )),
        )),
        // The bar over a selection can be changed: here it offers Copy only, since a
        // one-line code has nothing worth "Select all".
        Box::new(section(
            "Custom selection bar",
            SelectionArea::around(Text::new("WIFI-PASSWORD-2026").family(FontFamily::Monospace))
                .selection_toolbar(|_, items: Vec<ToolbarItem>| {
                    items
                        .into_iter()
                        .filter(|item| item.label() == "Copy")
                        .collect()
                }),
        )),
    ])
}
