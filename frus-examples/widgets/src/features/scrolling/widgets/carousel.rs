//! `CarouselView` — one slide at a time between ‹ and › arrows. It is controlled: the
//! app keeps the index and supplies the slide for it, so only one slide is ever built.

use frus::{CarouselView, Center, Color, Colors, Container, Text, Widget};

use crate::common::shared::{section, sections};

const SLIDES: [(&str, &str); 4] = [
    ("Mountains", "#0F766E"),
    ("Desert", "#B45309"),
    ("Forest", "#15803D"),
    ("Ocean", "#1D4ED8"),
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let index = cx.use_state(|| 0usize);

        // The arrows report the index to go to (an arrow at an end is disabled); the
        // slide is whatever the app builds for the current index.
        let go = index.clone();
        let (name, hue) = SLIDES[index.get()];
        let carousel = CarouselView::new(
            index.get(),
            SLIDES.len(),
            move |i: usize| {
                let go = go.clone();
                frus::Callback::new(move || go.set(i))
            },
            Container::new()
                .height(160.0)
                .radius(12.0)
                .color(Color::hex(hue))
                .child(Center::new(Text::new(name).size(22.0).color(Colors::WHITE))),
        );

        sections(vec![
            Box::new(section(
                "Controlled carousel",
                Container::new().width(320.0).child(carousel),
            )),
            Box::new(Text::new(format!(
                "Slide {} of {}",
                index.get() + 1,
                SLIDES.len()
            ))),
        ])
    }))
}
