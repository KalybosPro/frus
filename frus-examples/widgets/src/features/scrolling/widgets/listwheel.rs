//! `ListWheel` — the spinning cylinder of values you turn to pick one: the control
//! behind time and duration pickers, and the one to reach for a picker of your own.

use frus::{Center, Color, Container, ListWheel, Row, Text, Widget};

use crate::common::shared::{section, sections};

const UNITS: [&str; 5] = ["mm", "cm", "m", "km", "in"];

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let hours = cx.use_state(|| 7usize);
        let minutes = cx.use_state(|| 30usize);
        let unit = cx.use_state(|| 2usize);

        // `selected` is the app's value (applied on the first frame and on every change);
        // `on_selected` reports the row the wheel came to rest on. Rows are built from
        // their index, and `label` says what each is to a screen reader.
        let set_hours = hours.clone();
        let hour_wheel = ListWheel::new(24, 36.0, |i| {
            Center::new(Text::new(format!("{i:02}")).size(20.0))
        })
        .selected(hours.get())
        .on_selected(move |i: usize| set_hours.set(i))
        .label(|i| format!("{i} hours"))
        .width(72.0)
        .height(180.0);

        let set_minutes = minutes.clone();
        let minute_wheel = ListWheel::new(60, 36.0, |i| {
            Center::new(Text::new(format!("{i:02}")).size(20.0))
        })
        .selected(minutes.get())
        .on_selected(move |i: usize| set_minutes.set(i))
        .label(|i| format!("{i} minutes"))
        .width(72.0)
        .height(180.0);

        // A flatter cylinder: a larger `diameter_ratio` curves the rows less.
        let set_unit = unit.clone();
        let unit_wheel = ListWheel::new(UNITS.len(), 36.0, |i| {
            Center::new(Text::new(UNITS[i]).size(18.0))
        })
        .selected(unit.get())
        .on_selected(move |i: usize| set_unit.set(i))
        .label(|i| UNITS[i].to_string())
        .diameter_ratio(3.0)
        .width(96.0)
        .height(180.0);

        sections(vec![
            Box::new(section(
                "Time",
                Container::new()
                    .border(1.0, Color::hex("#A1A1AA"))
                    .radius(12.0)
                    .child(
                        Row::new()
                            .main_axis_size(frus::MainAxisSize::Min)
                            .child(hour_wheel)
                            .child(Text::new(":").size(20.0))
                            .child(minute_wheel),
                    ),
            )),
            Box::new(Text::new(format!(
                "Alarm at {:02}:{:02}",
                hours.get(),
                minutes.get()
            ))),
            Box::new(section("A flatter wheel", unit_wheel)),
            Box::new(Text::new(format!("Unit: {}", UNITS[unit.get()]))),
        ])
    }))
}
