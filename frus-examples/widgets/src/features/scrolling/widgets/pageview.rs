//! `PageView` — a scrollable that always comes to rest on a page: onboarding screens, a
//! photo gallery, swipeable panels. Pages are built on demand from their index.

use frus::{Align, Axis, Center, Color, Colors, Column, Container, PageView, Text, Widget};

use crate::common::shared::{section, sections};

const HUES: [&str; 5] = ["#6366F1", "#0EA5E9", "#10B981", "#F59E0B", "#EF4444"];

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let page = cx.use_state(|| 0usize);

        // `on_page_changed` reports the page a reader would name as the finger moves,
        // so a caption follows the picture rather than trailing it.
        let follow = page.clone();
        let full = PageView::new(HUES.len(), |i| card(i, "Swipe sideways"))
            .height(180.0)
            .width(320.0)
            .on_page_changed(move |i: usize| follow.set(i));

        // `viewport_fraction` below 1 shows a sliver of the neighbours — a hint that
        // there is more to swipe to.
        let peek = PageView::new(HUES.len(), |i| {
            Container::new().padding(6.0).child(card(i, "Peek"))
        })
        .viewport_fraction(0.8)
        .height(160.0)
        .width(320.0);

        // Pages can run down instead of across.
        let vertical = PageView::new(HUES.len(), |i| card(i, "Swipe up"))
            .axis(Axis::Vertical)
            .height(160.0)
            .width(320.0);

        sections(vec![
            Box::new(section(
                "Full pages",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Start)
                    .child(full)
                    .child(
                        Text::new(format!("Page {} of {}", page.get() + 1, HUES.len()))
                            .color(cx.theme().muted),
                    ),
            )),
            Box::new(section("Neighbours peeking in", peek)),
            Box::new(section("Vertical", vertical)),
        ])
    }))
}

/// One page: a plain widget made from its index.
fn card(i: usize, hint: &str) -> Container {
    Container::new()
        .radius(12.0)
        .color(Color::hex(HUES[i % HUES.len()]))
        .child(Center::new(
            Text::new(format!("{} · page {}", hint, i + 1))
                .size(18.0)
                .color(Colors::WHITE),
        ))
}
