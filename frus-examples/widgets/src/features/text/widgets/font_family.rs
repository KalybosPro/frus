//! `FontFamily` — which typeface a text is set in: the default sans-serif, a serif, the
//! monospace face, or a font you registered by name.

use frus::{Align, Column, FontFamily, Text, TextStyle, Widget};

use crate::common::shared::{section, sections};

const SAMPLE: &str = "The quick brown fox jumps over the lazy dog";

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // What text gets when it says nothing.
        Box::new(section(
            "SansSerif (the default)",
            Text::new(SAMPLE).family(FontFamily::SansSerif),
        )),
        // A serif face, where the platform or the application has one; otherwise the
        // default is used.
        Box::new(section(
            "Serif",
            Text::new(SAMPLE).family(FontFamily::Serif),
        )),
        // Monospace: every character the same width, so columns line up — code, keys,
        // tables of numbers.
        Box::new(section(
            "Monospace",
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(2.0)
                .child(Text::new("fn main() {").family(FontFamily::Monospace))
                .child(Text::new("    println!(\"hi\");").family(FontFamily::Monospace))
                .child(Text::new("}").family(FontFamily::Monospace))
                .child(Text::new("1 111.00").family(FontFamily::Monospace))
                .child(Text::new("   42.50").family(FontFamily::Monospace)),
        )),
        // Your own font: register its bytes once at start-up with
        // `frus::fonts::add_font(include_bytes!("../res/fonts/Brand.ttf").to_vec())`, then
        // name its family. A name that matches no font falls back to the default rather
        // than showing nothing — as this one does.
        Box::new(section(
            "Named (falls back when missing)",
            Text::styled(
                SAMPLE,
                TextStyle::NONE.family(FontFamily::Named("Brand Sans")),
            ),
        )),
    ])
}
