//! `ExpansionTile` — a list row that opens to show more underneath. It is controlled:
//! it shows the open state it is given and sends a message asking to change it, which
//! is what lets a column of them act as an accordion.

use frus::{Align, Callback, Color, Column, Container, ControlAffinity, Icon, Icons, Text, Widget};

use crate::common::shared::{section, sections};

const FAQ: [(&str, &str); 3] = [
    (
        "How long is delivery?",
        "Two to four working days in most of Europe.",
    ),
    (
        "Can I return an item?",
        "Yes, within thirty days, in its original packaging.",
    ),
    (
        "Do you ship abroad?",
        "We ship to forty countries; the fee is shown at checkout.",
    ),
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let open = cx.use_state(|| [true, false, false]);
        let accordion = cx.use_state(|| None::<usize>);

        // Independent tiles: each keeps its own open flag.
        let independent = FAQ.iter().enumerate().fold(
            Column::new().cross_axis_alignment(Align::Stretch),
            |column, (i, (question, answer))| {
                let toggle = open.clone();
                column.child(
                    frus::ExpansionTile::new(
                        *question,
                        open.get()[i],
                        Callback::new(move || toggle.update(|o| o[i] = !o[i])),
                    )
                    .content(Text::new(*answer).wrap()),
                )
            },
        );

        // An accordion: the app keeps one index, so opening one closes the others.
        let exclusive = FAQ.iter().enumerate().fold(
            Column::new().cross_axis_alignment(Align::Stretch),
            |column, (i, (question, answer))| {
                let pick = accordion.clone();
                let is_open = accordion.get() == Some(i);
                column.child(
                    frus::ExpansionTile::new(
                        *question,
                        is_open,
                        Callback::new(move || pick.set(if is_open { None } else { Some(i) })),
                    )
                    .leading(Icon::new(Icons::HELP_OUTLINE))
                    .content(Text::new(*answer).wrap()),
                )
            },
        );

        // The chevron in front, a subtitle, and colours for the open state.
        let toggle = open.clone();
        let styled = frus::ExpansionTile::new(
            "Advanced settings",
            open.get()[1],
            Callback::new(move || toggle.update(|o| o[1] = !o[1])),
        )
        .subtitle("Proxy, cache, logs")
        .control_affinity(ControlAffinity::Leading)
        .background_color(Color::hex("#EEF2FF"))
        .radius(12.0)
        .content(Text::new(
            "Shares its open flag with the second question above.",
        ));

        sections(vec![
            Box::new(section(
                "Independent",
                Container::new().width(320.0).child(independent),
            )),
            Box::new(section(
                "Accordion (one open at a time)",
                Container::new().width(320.0).child(exclusive),
            )),
            Box::new(section(
                "Leading chevron, subtitle, colours",
                Container::new().width(320.0).child(styled),
            )),
        ])
    }))
}
