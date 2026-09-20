//! The pieces more than one screen draws. Nothing arrives here on suspicion that
//! it might be shared later; it moves in when the second screen asks for it.

use crate::prelude::*;
use frus_widgets::column;

/// The surface this screen is being built for — the window, as the shell described it.
///
/// A screen that **is** the window still has to say so somewhere; what it no longer does
/// is take the number from its caller, who took it from *its* caller, back to a `view`
/// that was handed it by the framework. It asks the description in force, which is what
/// the reference's `MediaQuery.of(context).size` is for.
pub(crate) fn surface() -> Size {
    MediaQuery::of().size
}

/// A handler that does its work and returns nothing, as the callback every widget's setter takes.
///
/// A bare closure would do for most setters, but one that returns nothing could stand for a
/// message of any type, and where nothing else says which the compiler cannot choose. Naming
/// the callback is what says it.
pub(crate) fn on(work: impl Fn() + 'static) -> Callback {
    Callback::new(work)
}

/// The same for a widget that reports a value: `on_value(move |text| demo.rename(text))`.
pub(crate) fn on_value<T: Clone + 'static>(
    work: impl Fn(T) + 'static,
) -> impl Fn(T) -> Callback + 'static {
    let work = Rc::new(work);
    move |value| {
        let work = work.clone();
        Callback::new(move || work(value.clone()))
    }
}

/// The same for a widget that reports two values: `on_values(move |from, to| demo.move(from, to))`.
pub(crate) fn on_values<A: Clone + 'static, B: Clone + 'static>(
    work: impl Fn(A, B) + 'static,
) -> impl Fn(A, B) -> Callback + 'static {
    let work = Rc::new(work);
    move |a, b| {
        let work = work.clone();
        Callback::new(move || work(a.clone(), b.clone()))
    }
}

/// A statistic tile (a big number + a label) for the grid.
pub(crate) fn stat_tile(theme: &Theme, label: &str, value: usize) -> Container {
    Container::new()
        .height(64.0)
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.outline_variant)
        .padding_each(10.0, 12.0, 10.0, 12.0)
        .child(column![
            text(value.to_string()).size(24.0),
            text(label.to_string()).size(13.0).color(theme.muted),
        ])
}

/// Day of the week (0 = Sunday … 6 = Saturday) of a date (Sakamoto) — milestone 238.
pub(crate) fn weekday(y: i32, m: u32, d: u32) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    (((yy + yy / 4 - yy / 100 + yy / 400 + T[(m - 1) as usize] + d as i32) % 7 + 7) % 7) as u32
}

/// True when the date falls on a **weekend** (Saturday or Sunday).
pub(crate) fn is_weekend(y: i32, m: u32, d: u32) -> bool {
    matches!(weekday(y, m, d), 0 | 6)
}

/// The "About" section: static introductory content.
///
/// **Nothing here counts pixels.** The column fills the card it sits in, and the only
/// number is the one a designer would give: a measure no wider than 560, because a
/// line of prose across a desktop is unreadable.
///
/// It used to subtract the paddings by hand — the container's 24×2 plus the card's
/// 20×2 — and forgot that a card carries a margin of its own. Eight pixels out, on
/// every phone, drawn past the card it was inside. Milestone 392 is why that showed.
pub(crate) fn about_section(theme: &Theme) -> Container {
    Container::new().padding(24.0).child(
        Card::new().padding(20.0).child(
            ConstrainedBox::new(
                column![
                    text("About frus").size(24.0),
                    // Rich text: mixed styles on one line, with cascading inheritance.
                    RichText::new(
                        TextSpan::new("A ")
                            .child(TextSpan::new("fast").bold())
                            .child(TextSpan::new(", "))
                            .child(TextSpan::new("portable").italic().underline())
                            .child(TextSpan::new(" Rust UI framework — "))
                            .child(TextSpan::new("no GC").bold().color(theme.primary))
                            .child(TextSpan::new(".")),
                    )
                    .base_style(theme.text.body_medium.color(theme.muted))
                    .wrap(),
                    Divider::new(),
                    Timeline::new()
                        .event("Responsive primitives", "Milestone 42")
                        .event("Adaptive navigation", "Milestone 43"),
                    // A paragraph: it wraps at the card's width.
                    text(
                        "Layout, painting, typography and animation are engine-level \
                     foundations shared by every widget in this gallery.",
                    )
                    .size(13.0)
                    .color(theme.muted)
                    .wrap(),
                ]
                .gap(12.0),
            )
            .max_width(560.0),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays_are_where_the_calendar_says() {
        // 2026-07-01 is a Wednesday; the 4th a Saturday.
        assert_eq!(weekday(2026, 7, 1), 3);
        assert!(is_weekend(2026, 7, 4));
        assert!(is_weekend(2026, 7, 5));
        assert!(!is_weekend(2026, 7, 6));
    }
}
