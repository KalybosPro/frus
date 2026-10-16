//! `Breadcrumb` — the trail from the top of a hierarchy to the current page. Every
//! segment but the last can be clicked to go back up.

use frus::{Align, Breadcrumb, Button, Column, Text, Variant, Widget};

const PATH: [&str; 5] = ["Home", "Documents", "Projects", "frus", "Roadmap"];

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        // How deep we are: the trail shows `PATH[..=depth]`.
        let depth = cx.use_state(|| PATH.len() - 1);

        // `on_select(i)` reports the segment clicked; going there means cutting the trail.
        let go_up = depth.clone();
        let trail = PATH[..=depth.get()].iter().fold(
            Breadcrumb::new(move |i: usize| go_up.set(i)),
            |trail, label| trail.crumb(*label),
        );

        let reset = depth.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(trail)
                .child(
                    Text::new(format!("Current page: {}", PATH[depth.get()]))
                        .color(cx.theme().muted),
                )
                .child(
                    Button::new("Back to Roadmap")
                        .variant(Variant::Outlined)
                        .enabled(depth.get() + 1 < PATH.len())
                        .on_press(move || reset.set(PATH.len() - 1)),
                ),
        )
    }))
}
