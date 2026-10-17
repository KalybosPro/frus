//! `FittedBox` — scales its child to fit its box, following a `BoxFit` rule.
//!
//! Unlike `Transform::scale`, the scale is worked out from the layout: put the same
//! child in a bigger box and it grows with it.

use frus::{Align, Alignment, BoxFit, Color, Column, Container, FittedBox, Text, Widget, Wrap};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // The same 80 × 40 badge in a 140 × 90 box under each rule. `Contain` shows all
        // of it, `Cover` fills the box and crops, `Fill` stretches it out of shape,
        // `ScaleDown` only ever shrinks, `None` leaves it at its own size.
        Box::new(section(
            "Each BoxFit",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(fitted("Contain", BoxFit::Contain))
                .child(fitted("Cover", BoxFit::Cover))
                .child(fitted("Fill", BoxFit::Fill))
                .child(fitted("FitWidth", BoxFit::FitWidth))
                .child(fitted("FitHeight", BoxFit::FitHeight))
                .child(fitted("ScaleDown", BoxFit::ScaleDown))
                .child(fitted("None", BoxFit::None)),
        )),
        // The everyday use: a label scaled to fill a slot whatever its own size — a big
        // figure on a dashboard tile.
        Box::new(section(
            "Growing a label to its slot",
            Container::new().border(1.0, Color::hex("#A5B4FC")).child(
                FittedBox::new(BoxFit::Contain)
                    .width(240.0)
                    .height(64.0)
                    .child(Text::new("1 234").size(14.0)),
            ),
        )),
    ])
}

fn fitted(name: &str, fit: BoxFit) -> Column {
    Column::new()
        .cross_axis_alignment(Align::Start)
        .spacing(4.0)
        .child(Text::new(name).size(12.0))
        .child(
            Container::new()
                .color(Color::hex("#EEF2FF"))
                .border(1.0, Color::hex("#A5B4FC"))
                .clip()
                .child(FittedBox::new(fit).width(140.0).height(90.0).child(badge())),
        )
}

fn badge() -> Container {
    Container::new()
        .width(80.0)
        .height(40.0)
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .alignment(Alignment::CENTER)
        .child(Text::new("80 × 40").size(13.0).color(Color::hex("#FFFFFF")))
}
