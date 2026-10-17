//! `TabPageSelector` — the row of dots that says which of several pages you are on:
//! under an onboarding walkthrough, a gallery or a carousel.

use frus::{
    Align, Button, Center, Color, Column, Container, Icon, Icons, PageView, Row, TabPageSelector,
    Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

const SLIDES: [(&str, &str); 3] = [
    ("Welcome", "#6366F1"),
    ("Sync everywhere", "#0EA5E9"),
    ("You're all set", "#22C55E"),
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let page = cx.use_state(|| 0usize);

        // The page view reports the page as the finger moves; the dots follow it, and the
        // buttons move the view by changing the page it is given.
        let follow = page.clone();
        let pages = PageView::new(SLIDES.len(), |i| {
            let (title, color) = SLIDES[i];
            Container::new().color(Color::hex(color)).child(Center::new(
                Text::new(title).size(22.0).color(frus::Colors::WHITE),
            ))
        })
        .height(180.0)
        .page(page.get())
        .on_page_changed(move |i: usize| follow.set(i));

        let back = page.clone();
        let next = page.clone();
        let walkthrough = Container::new().width(320.0).child(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Stretch)
                .child(pages)
                .child(
                    Row::new()
                        .main_axis_alignment(frus::Justify::SpaceBetween)
                        .child(
                            Button::new("Back")
                                .variant(Variant::Text)
                                .enabled(page.get() > 0)
                                .on_press(move || back.update(|p| *p = p.saturating_sub(1))),
                        )
                        .child(TabPageSelector::new(SLIDES.len(), page.get()))
                        .child(
                            Button::new("Next")
                                .variant(Variant::Text)
                                .enabled(page.get() + 1 < SLIDES.len())
                                .on_press(move || next.update(|p| *p += 1)),
                        ),
                ),
        );

        sections(vec![
            Box::new(section(
                "Under a page view (swipe or use the buttons)",
                walkthrough,
            )),
            // Size, gap and colours are the theme's by default.
            Box::new(section(
                "Custom size and colours",
                Row::new()
                    .spacing(16.0)
                    .main_axis_size(frus::MainAxisSize::Min)
                    .child(Icon::new(Icons::PHOTO_LIBRARY))
                    .child(
                        TabPageSelector::new(5, 3)
                            .size(12.0)
                            .gap(10.0)
                            .color(Color::hex("#F97316"))
                            .border_color(Color::hex("#FDBA74")),
                    ),
            )),
        ])
    }))
}
