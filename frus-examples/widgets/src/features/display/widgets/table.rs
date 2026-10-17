//! `Table` — a grid of rows and columns that only shows what it is handed: the app
//! sorts its own data and decides what is selected. Cells can be text or widgets.

use frus::{Badge, Color, Colors, Icons, Table, Text, Widget};

use crate::common::shared::{section, sections};

/// One planet: name, moons, and how far from the sun (in astronomical units).
pub const PLANETS: [(&str, u32, f32); 5] = [
    ("Mercury", 0, 0.39),
    ("Venus", 0, 0.72),
    ("Earth", 1, 1.0),
    ("Mars", 2, 1.52),
    ("Jupiter", 95, 5.2),
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let sort = cx.use_state(|| (0usize, true));
        let selected = cx.use_state(|| None::<usize>);

        // The base `Table` does not reorder anything: on a header click the app sorts its
        // own rows and hands them back, with `sorted` to draw the arrow.
        let (column, ascending) = sort.get();
        let rows = sorted_planets(column, ascending);

        let on_sort = sort.clone();
        let on_select = selected.clone();
        let sortable = rows
            .iter()
            .fold(
                Table::new(3)
                    .header(&["Planet", "Moons", "Distance (AU)"])
                    .header_icons(&[Some(Icons::PUBLIC), None, None])
                    .column_widths(&[120.0, 80.0, 120.0]),
                |table, &(name, moons, au)| {
                    table.row(&[
                        name,
                        moons.to_string().as_str(),
                        format!("{au:.2}").as_str(),
                    ])
                },
            )
            .sorted(column, ascending)
            .on_sort(move |clicked: usize| {
                on_sort.update(|(c, asc)| {
                    *asc = if *c == clicked { !*asc } else { true };
                    *c = clicked;
                })
            })
            // A row click reports the row as displayed; this table shows the app's order.
            .on_select_row(move |row: usize| on_select.set(Some(row)))
            .selected(&selected.get().into_iter().collect::<Vec<_>>());

        let picked = selected
            .get()
            .and_then(|row| rows.get(row))
            .map_or("Click a row".to_string(), |(name, ..)| {
                format!("Selected: {name}")
            });

        sections(vec![
            Box::new(section("Sortable and selectable", sortable)),
            Box::new(Text::new(picked).color(cx.theme().muted)),
            // A cell can be any widget: build it in a closure, which the table calls again
            // on every rebuild.
            Box::new(section(
                "Widget cells",
                Table::new(2)
                    .header(&["Service", "Status"])
                    .column_widths(&[160.0, 120.0])
                    .widget_row(vec![
                        Box::new(|| Box::new(Text::new("API")) as Box<dyn Widget>),
                        Box::new(|| Box::new(status("Up", "#16A34A")) as Box<dyn Widget>),
                    ])
                    .widget_row(vec![
                        Box::new(|| Box::new(Text::new("Billing")) as Box<dyn Widget>),
                        Box::new(|| Box::new(status("Degraded", "#D97706")) as Box<dyn Widget>),
                    ])
                    .widget_row(vec![
                        Box::new(|| Box::new(Text::new("Search")) as Box<dyn Widget>),
                        Box::new(|| Box::new(status("Down", "#DC2626")) as Box<dyn Widget>),
                    ]),
            )),
        ])
    }))
}

/// A coloured label for a status cell.
pub fn status(label: &str, hex: &str) -> Badge {
    Badge::new(label)
        .background_color(Color::hex(hex))
        .text_color(Colors::WHITE)
}

/// The planets ordered by one column — the sorting the app owns.
pub fn sorted_planets(column: usize, ascending: bool) -> Vec<(&'static str, u32, f32)> {
    let mut rows = PLANETS.to_vec();
    rows.sort_by(|a, b| match column {
        1 => a.1.cmp(&b.1),
        2 => a.2.total_cmp(&b.2),
        _ => a.0.cmp(b.0),
    });
    if !ascending {
        rows.reverse();
    }
    rows
}
