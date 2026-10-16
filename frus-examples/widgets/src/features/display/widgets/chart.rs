//! `BarChart` and `LineChart` — self-painted, themed views of a series of
//! `(label, value)` pairs: bars to compare amounts, a line to read a trend.

use frus::{BarChart, Color, LineChart, Text, Widget};

use crate::common::shared::{section, sections};

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
const WEB: [f32; 6] = [12.0, 18.0, 15.0, 22.0, 28.0, 31.0];
const MOBILE: [f32; 6] = [8.0, 11.0, 17.0, 19.0, 21.0, 30.0];

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let picked = cx.use_state(|| None::<(usize, usize)>);
        let hidden = cx.use_state(Vec::<usize>::new);
        let stacked = cx.use_state(|| false);

        let series = || MONTHS.iter().copied().zip(WEB);
        let mobile_color = Color::hex("#F59E0B");

        // Clicking a bar reports `(category, series)`; `selected` rings the one the app
        // pinned. The chart fills the width it is given.
        let pick = picked.clone();
        let clickable = BarChart::new(series())
            .height(180.0)
            .grid(4)
            .on_point(move |category: usize, series: usize| pick.set(Some((category, series))))
            .selected(picked.get());
        let caption = match picked.get() {
            Some((category, _)) => format!("{}: {} visits", MONTHS[category], WEB[category]),
            None => "Click a bar".to_string(),
        };

        // Several series share one scale: grouped side by side, or stacked. A clickable
        // legend lets the reader hide one — the hidden list is the app's state.
        let toggle_series = hidden.clone();
        let flip = stacked.clone();
        let grouped = BarChart::new(series())
            .name("Web")
            .series("Mobile", mobile_color, MOBILE)
            .height(180.0)
            .grid(4)
            .legend(true)
            .stacked(stacked.get())
            .hidden(hidden.get())
            .on_legend(move |index: usize| {
                toggle_series.update(|hidden| {
                    if let Some(at) = hidden.iter().position(|&h| h == index) {
                        hidden.remove(at);
                    } else {
                        hidden.push(index);
                    }
                })
            });

        sections(vec![
            Box::new(section("Bar chart", clickable)),
            Box::new(Text::new(caption).color(cx.theme().muted)),
            Box::new(section("Two series, clickable legend", grouped)),
            Box::new(
                frus::Button::new(if stacked.get() {
                    "Group the bars"
                } else {
                    "Stack the bars"
                })
                .variant(frus::Variant::Outlined)
                .on_press(move || flip.update(|s| *s = !*s)),
            ),
            // The same data as a trend line; `area` fills under it.
            Box::new(section(
                "Line chart",
                LineChart::new(series()).height(160.0).grid(4).area(true),
            )),
            Box::new(section(
                "Two lines",
                LineChart::new(series())
                    .name("Web")
                    .series("Mobile", mobile_color, MOBILE)
                    .height(160.0)
                    .grid(4)
                    .legend(true),
            )),
        ])
    }))
}
