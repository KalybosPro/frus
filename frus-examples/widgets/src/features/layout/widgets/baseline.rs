//! `Baseline`, `IgnoreBaseline` and `Align::Baseline` — line text up on the line its
//! letters sit on, not on the tops or middles of the boxes.

use frus::{
    Align, Baseline, Border, BorderSide, Color, Container, IgnoreBaseline, MainAxisSize, Row,
    Stack, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Centred, a big number and its small unit do not read as one line: the unit
        // floats halfway up.
        Box::new(section("Centred (the default)", price(Align::Center))),
        // On their baselines they share a line, whatever their sizes.
        Box::new(section("Align::Baseline", price(Align::Baseline))),
        // A row aligned on baselines follows the lowest one it finds. `IgnoreBaseline`
        // takes a piece out of the reckoning — here, the badge's text, which would
        // otherwise drag the line down to its own.
        Box::new(section(
            "IgnoreBaseline",
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(Align::Baseline)
                .spacing(8.0)
                .child(Text::new("Release notes").size(22.0))
                .child(
                    IgnoreBaseline::new().child(
                        Container::new()
                            .padding_each(2.0, 6.0, 2.0, 6.0)
                            .radius(4.0)
                            .color(Color::hex("#F59E0B"))
                            .child(Text::new("beta").size(11.0).color(Color::hex("#FFFFFF"))),
                    ),
                ),
        )),
        // `Baseline` puts its child's baseline a set distance below its top — to pin a
        // label to a rule drawn elsewhere. The grey line is 32 px down; so is the text's
        // baseline.
        Box::new(section(
            "Baseline at 32 px",
            Stack::new()
                .layer(
                    Container::new()
                        .width(220.0)
                        .height(32.0)
                        .box_border(Border {
                            bottom: BorderSide::new(Color::hex("#94A3B8"), 1.0),
                            ..Border::NONE
                        }),
                )
                .layer(Baseline::new(32.0).child(Text::new("Sits on the rule").size(18.0))),
        )),
    ])
}

fn price(align: Align) -> Row {
    Row::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(align)
        .spacing(4.0)
        .child(Text::new("42").size(48.0))
        .child(Text::new(".99").size(20.0))
        .child(Text::new("EUR / month").size(13.0))
}
