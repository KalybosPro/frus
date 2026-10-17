use std::rc::Rc;

use frus::{
    Align, BuildContext, Button, Callback, Column, DatePicker, IconButton, Icons, Insets, Justify,
    Padding, Row, StatelessWidget, Text, Variant, Widget,
};

use crate::models::profile::Date;

/// The content of the date-of-birth dialog: a year switcher over a month calendar.
///
/// The calendar steps a month at a time, which is right for an appointment and slow for a
/// birthday twenty years back — so the year has its own arrows, which move the same
/// calendar twelve months at a time. Days after `latest` (today) are greyed out and cannot
/// be picked.
pub struct BirthDateCalendar {
    /// The month on screen.
    year: i32,
    month: u32,
    selected: Option<Date>,
    latest: Option<Date>,
    on_day: Rc<dyn Fn(u32) -> Callback>,
    on_shift: Rc<dyn Fn(i32) -> Callback>,
    on_cancel: Callback,
}

impl BirthDateCalendar {
    pub fn new(
        (year, month): (i32, u32),
        selected: Option<Date>,
        latest: Option<Date>,
        on_day: impl Fn(u32) -> Callback + 'static,
        on_shift: impl Fn(i32) -> Callback + 'static,
        on_cancel: Callback,
    ) -> Self {
        Self {
            year,
            month,
            selected,
            latest,
            on_day: Rc::new(on_day),
            on_shift: Rc::new(on_shift),
            on_cancel,
        }
    }
}

impl StatelessWidget for BirthDateCalendar {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();

        let years = Row::new()
            .main_axis_alignment(Justify::Center)
            .cross_axis_alignment(Align::Center)
            .spacing(8.0)
            .child(
                IconButton::new(Icons::KEYBOARD_DOUBLE_ARROW_LEFT)
                    .label("Previous year")
                    .on_press((self.on_shift)(-12)),
            )
            .child(Text::styled(self.year.to_string(), theme.text.title_medium))
            .child(
                IconButton::new(Icons::KEYBOARD_DOUBLE_ARROW_RIGHT)
                    .label("Next year")
                    .on_press((self.on_shift)(12)),
            );

        // The calendar marks a day of the month it shows, so the chosen date is passed on
        // only when it falls in that month.
        let selected_day = self
            .selected
            .filter(|d| (d.year, d.month) == (self.year, self.month))
            .map(|d| d.day);
        let (on_day, on_shift) = (self.on_day.clone(), self.on_shift.clone());
        let calendar = DatePicker::bounded(
            self.year,
            self.month,
            selected_day,
            None,
            self.latest.map(|d| (d.year, d.month, d.day)),
            move |day| on_day(day),
            move |months| on_shift(months),
        );

        let actions = Row::new().main_axis_alignment(Justify::End).child(
            Button::new("Cancel")
                .variant(Variant::Text)
                .on_press(self.on_cancel.clone()),
        );

        // A dialog is a bare surface: the room inside its edges is the content's to give.
        Box::new(Padding::new(
            Insets::new(24.0, 16.0, 12.0, 16.0),
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Center)
                .child(Text::styled("Date of birth", theme.text.headline_small))
                .child(years)
                .child(calendar)
                .child(actions),
        ))
    }
}
