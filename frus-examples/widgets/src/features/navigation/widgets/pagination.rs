//! `Pagination` — a page picker: previous, a window of page numbers, next. Pages are
//! counted from 1, and the current page is the app's state.

use frus::{Align, Column, Pagination, Text, Widget};

use crate::common::shared::{section, sections};

const TOTAL: usize = 20;

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let page = cx.use_state(|| 1usize);
        let few = cx.use_state(|| 2usize);

        // A picker reports the page clicked; the app stores it and shows that page.
        let go = page.clone();
        let many = Pagination::new(page.get(), TOTAL, move |p: usize| {
            let go = go.clone();
            frus::Callback::new(move || go.set(p))
        });

        let go_few = few.clone();
        let short = Pagination::new(few.get(), 3, move |p: usize| {
            let go = go_few.clone();
            frus::Callback::new(move || go.set(p))
        });

        let first = (page.get() - 1) * 10 + 1;
        sections(vec![
            Box::new(section(
                "Many pages",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Start)
                    .child(many)
                    .child(
                        Text::new(format!("Showing results {first}–{}", first + 9))
                            .color(cx.theme().muted),
                    ),
            )),
            Box::new(section("Three pages", short)),
            // Disabled: still shows where you are, but nothing can be picked.
            Box::new(section(
                "Disabled",
                Pagination::new(4, 9, |_| frus::Callback::new(|| {})).enabled(false),
            )),
        ])
    }))
}
