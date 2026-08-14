//! [`TimePicker`]: a **controlled** time picker, the counterpart of [`crate::DatePicker`].
//! Two grids of clickable cells (hours, minutes) plus a preview; 24-hour by default, or
//! 12-hour with an AM/PM toggle ([`hour12`](TimePicker::hour12)). The minute step is
//! adjustable ([`minute_step`](TimePicker::minute_step)). The time comes from the
//! application state; the widget emits on click, always as a **24-hour** time.

use std::rc::Rc;

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::grid::Grid;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

const CELL: f32 = 34.0;
const SIZE: f32 = 15.0;

/// A clickable number cell (hour, minute or AM/PM), highlighted when selected.
struct TimeCell<Msg> {
    label: String,
    selected: bool,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for TimeCell<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(CELL),
            height: Dimension::Length(CELL),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (bg, fg) = if self.selected {
            (theme.primary, theme.on_primary)
        } else {
            (
                theme.state_layer(theme.surface, theme.on_surface, &status),
                theme.on_surface,
            )
        };
        scene.draw_rect(bounds, bg.fade(o), CELL * 0.5, 0.0, Color::TRANSPARENT);
        let w = frus_text::measure(&self.label, SIZE).width;
        scene.text(
            Point::new(
                bounds.x + (CELL - w) * 0.5,
                bounds.y + (CELL - frus_text::line_height(SIZE)) * 0.5,
            ),
            self.label.clone(),
            SIZE,
            fg.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }
}

/// A time picker.
pub struct TimePicker<Msg> {
    hour: u32,
    minute: u32,
    on_hour: Box<dyn Fn(u32) -> Msg>,
    on_minute: Box<dyn Fn(u32) -> Msg>,
    hour12: bool,
    minute_step: u32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

/// The 12-hour clock digit (1–12) for a 24-hour hour.
fn digit12(hour24: u32) -> u32 {
    if hour24.is_multiple_of(12) {
        12
    } else {
        hour24 % 12
    }
}

impl<Msg: Clone + 'static> TimePicker<Msg> {
    /// Creates a picker for `hour` (0–23) and `minute` (0–59). `on_hour(h)` is emitted
    /// when an hour is clicked (always **24-hour**), `on_minute(m)` when a minute is.
    /// Defaults: 24-hour, minutes in steps of 5.
    pub fn new(
        hour: u32,
        minute: u32,
        on_hour: impl Fn(u32) -> Msg + 'static,
        on_minute: impl Fn(u32) -> Msg + 'static,
    ) -> Self {
        let mut picker = Self {
            hour: hour.min(23),
            minute: minute.min(59),
            on_hour: Box::new(on_hour),
            on_minute: Box::new(on_minute),
            hour12: false,
            minute_step: 5,
            children: Vec::new(),
        };
        picker.rebuild();
        picker
    }

    /// Switches to **12-hour**: a 1-to-12 grid + an AM/PM toggle; the preview shows
    /// 12-hour time. The messages emitted stay in 24-hour time.
    pub fn hour12(mut self) -> Self {
        self.hour12 = true;
        self.rebuild();
        self
    }

    /// Sets the **minute step** offered (1–60, clamped). Defaults to 5.
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = step.clamp(1, 60);
        self.rebuild();
        self
    }

    /// Assembles the preview and the grids from the current state.
    fn rebuild(&mut self) {
        let (hour, minute) = (self.hour, self.minute);
        let pm = hour >= 12;

        // HH:MM preview (+ AM/PM in 12-hour mode).
        let preview = if self.hour12 {
            let suffix = if pm { "PM" } else { "AM" };
            Text::new(format!("{}:{minute:02} {suffix}", digit12(hour))).size(28.0)
        } else {
            Text::new(format!("{hour:02}:{minute:02}")).size(28.0)
        };

        // The hours section.
        let hours_section = if self.hour12 {
            // AM/PM toggle: clicking a half moves the current hour into it.
            let am_target = if pm { hour - 12 } else { hour };
            let pm_target = if pm { hour } else { hour + 12 };
            let ampm = Flex::row()
                .gap(4.0)
                .child(TimeCell {
                    label: "AM".into(),
                    selected: !pm,
                    message: Some((self.on_hour)(am_target)),
                })
                .child(TimeCell {
                    label: "PM".into(),
                    selected: pm,
                    message: Some((self.on_hour)(pm_target)),
                });
            // A 1–12 grid; each cell targets the 24-hour hour of the current half.
            let current12 = digit12(hour);
            let mut grid = Grid::new(6).gap(4.0);
            for d in 1..=12u32 {
                let target24 = (d % 12) + if pm { 12 } else { 0 };
                grid = grid.cell(TimeCell {
                    label: format!("{d}"),
                    selected: d == current12,
                    message: Some((self.on_hour)(target24)),
                });
            }
            Flex::column()
                .gap(6.0)
                .child(Text::new("Hour").size(13.0))
                .child(ampm)
                .child(grid)
        } else {
            let mut grid = Grid::new(6).gap(4.0);
            for h in 0..24u32 {
                grid = grid.cell(TimeCell {
                    label: format!("{h:02}"),
                    selected: h == hour,
                    message: Some((self.on_hour)(h)),
                });
            }
            Flex::column()
                .gap(6.0)
                .child(Text::new("Hour").size(13.0))
                .child(grid)
        };

        // The minutes section (adjustable step). The selection only lights up if the
        // current minute falls on a step.
        let mut minutes = Grid::new(6).gap(4.0);
        let mut m = 0;
        while m < 60 {
            minutes = minutes.cell(TimeCell {
                label: format!("{m:02}"),
                selected: m == minute,
                message: Some((self.on_minute)(m)),
            });
            m += self.minute_step;
        }
        let minutes_section = Flex::column()
            .gap(6.0)
            .child(Text::new("Minute").size(13.0))
            .child(minutes);

        self.children = vec![
            Box::new(preview),
            Box::new(hours_section),
            Box::new(minutes_section),
        ];
    }
}

impl<Msg: Clone> Widget<Msg> for TimePicker<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(6.0 * (CELL + 4.0)),
            flex_direction: FlexDirection::Column,
            gap: 12.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Which endpoint of a time range ([`TimeRange`]) is targeted.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    /// The start time.
    Start,
    /// The end time.
    End,
}

/// Which field of a time ([`TimeRange`]) is changing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeField {
    Hour,
    Minute,
}

/// A **time range** (a start → end slot), the temporal counterpart of the dual calendar
/// ([`crate::DatePicker::range_dual`]): two [`TimePicker`]s labelled "Start" and "End",
/// side by side. A **single** `on_change(endpoint, field, value)` callback receives every
/// change (values always **24-hour**); the application decides how to update its state.
pub struct TimeRange<Msg> {
    start: (u32, u32),
    end: (u32, u32),
    on_change: Rc<dyn Fn(Endpoint, TimeField, u32) -> Msg>,
    hour12: bool,
    minute_step: u32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> TimeRange<Msg> {
    /// Creates a `start`/`end` range (each `(hour 0–23, minute 0–59)`). `on_change` is
    /// called on every click with the **endpoint**, the **field** and the new value (24-hour).
    pub fn new(
        start: (u32, u32),
        end: (u32, u32),
        on_change: impl Fn(Endpoint, TimeField, u32) -> Msg + 'static,
    ) -> Self {
        let mut range = Self {
            start: (start.0.min(23), start.1.min(59)),
            end: (end.0.min(23), end.1.min(59)),
            on_change: Rc::new(on_change),
            hour12: false,
            minute_step: 5,
            children: Vec::new(),
        };
        range.rebuild();
        range
    }

    /// Switches both pickers to **12-hour** (AM/PM).
    pub fn hour12(mut self) -> Self {
        self.hour12 = true;
        self.rebuild();
        self
    }

    /// Sets the **minute step** of both pickers (1–60).
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = step.clamp(1, 60);
        self.rebuild();
        self
    }

    /// (Re)builds the two labelled columns from the current state.
    fn rebuild(&mut self) {
        let (hour12, step) = (self.hour12, self.minute_step);
        let oc = self.on_change.clone();
        // A TimePicker whose messages are tagged with the endpoint they target.
        let make = |ep: Endpoint, h: u32, m: u32| -> TimePicker<Msg> {
            let (oc_h, oc_m) = (oc.clone(), oc.clone());
            let mut tp = TimePicker::new(
                h,
                m,
                move |hh| oc_h(ep, TimeField::Hour, hh),
                move |mm| oc_m(ep, TimeField::Minute, mm),
            );
            if hour12 {
                tp = tp.hour12();
            }
            tp.minute_step(step)
        };
        let start_col = Flex::column()
            .gap(8.0)
            .child(Text::new("Start").size(14.0))
            .child(make(Endpoint::Start, self.start.0, self.start.1));
        let end_col = Flex::column()
            .gap(8.0)
            .child(Text::new("End").size(14.0))
            .child(make(Endpoint::End, self.end.0, self.end.1));
        self.children = vec![Box::new(start_col), Box::new(end_col)];
    }
}

impl<Msg: Clone> Widget<Msg> for TimeRange<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 24.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::{Point, Primitive};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Hour(u32),
        Minute(u32),
    }

    fn preview_text(ui: &crate::Ui<Msg>) -> Option<String> {
        ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Text { text, position, .. } if position.y < 40.0 && text.contains(':') => {
                Some(text.clone())
            }
            _ => None,
        })
    }

    #[test]
    fn builds_header_hours_and_minutes() {
        let tp = TimePicker::new(9, 30, Msg::Hour, Msg::Minute);
        assert_eq!(Widget::<Msg>::children(&tp).len(), 3);
        // Section heures 24 h = [label, grille(24)].
        let hours_grid = &Widget::<Msg>::children(&tp)[1].children()[1];
        assert_eq!(hours_grid.children().len(), 24);
        // Minutes pas de 5 → 12 cases.
        let minutes_grid = &Widget::<Msg>::children(&tp)[2].children()[1];
        assert_eq!(minutes_grid.children().len(), 12);
    }

    #[test]
    fn minute_step_changes_the_minute_count() {
        let tp = TimePicker::new(0, 0, Msg::Hour, Msg::Minute).minute_step(15);
        let minutes_grid = &Widget::<Msg>::children(&tp)[2].children()[1];
        assert_eq!(minutes_grid.children().len(), 4, "60/15 = 4 minutes");
    }

    #[test]
    fn hour12_shows_am_pm_and_twelve_hours() {
        // 15 h 05 → 12 h : 3 PM.
        let tp = TimePicker::new(15, 5, Msg::Hour, Msg::Minute).hour12();
        // Section heures = [label, AM/PM, grille(12)].
        let hours_section = &Widget::<Msg>::children(&tp)[1];
        assert_eq!(hours_section.children().len(), 3);
        let hours_grid = &hours_section.children()[2];
        assert_eq!(hours_grid.children().len(), 12);

        let ui = build_ui(
            &tp,
            Size::new(240.0, 360.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(preview_text(&ui).as_deref(), Some("3:05 PM"));
    }

    #[test]
    fn twentyfour_hour_preview() {
        let tp = TimePicker::new(9, 30, Msg::Hour, Msg::Minute);
        let ui = build_ui(
            &tp,
            Size::new(240.0, 320.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(preview_text(&ui).as_deref(), Some("09:30"));
    }

    #[test]
    fn clicking_a_cell_emits_a_message() {
        let tp = TimePicker::new(0, 0, Msg::Hour, Msg::Minute);
        let ui = build_ui(
            &tp,
            Size::new(240.0, 320.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        let msg = (0..320)
            .step_by(4)
            .find_map(|y| click(CELL * 0.5, y as f32))
            .expect("a clickable cell exists");
        assert!(matches!(msg, Msg::Hour(_) | Msg::Minute(_)));
    }

    #[derive(Clone, Debug, PartialEq)]
    enum RangeMsg {
        Set(Endpoint, TimeField, u32),
    }

    #[test]
    fn range_builds_start_and_end_pickers() {
        let tr = TimeRange::new((9, 0), (17, 30), RangeMsg::Set).minute_step(15);
        // Two labelled columns [label, TimePicker].
        let cols = Widget::<RangeMsg>::children(&tr);
        assert_eq!(cols.len(), 2);
        let start_tp = &cols[0].children()[1];
        assert_eq!(start_tp.children().len(), 3, "preview + hours + minutes");
        // Minutes in steps of 15 → 4 cells.
        let start_minutes = &start_tp.children()[2].children()[1];
        assert_eq!(start_minutes.children().len(), 4);
        // Clicking 09 h in the End endpoint emits Set(End, Hour, 9).
        let end_tp = &cols[1].children()[1];
        let end_hours = &end_tp.children()[1].children()[1];
        assert_eq!(
            end_hours.children()[9].on_click(),
            Some(RangeMsg::Set(Endpoint::End, TimeField::Hour, 9)),
        );
    }

    #[test]
    fn hour12_applies_to_both_pickers() {
        let tr = TimeRange::new((15, 0), (20, 0), RangeMsg::Set).hour12();
        let cols = Widget::<RangeMsg>::children(&tr);
        // A 12-hour hours section = [label, AM/PM, grid(12)] → 3 children, for both endpoints.
        for col in cols {
            let tp = &col.children()[1];
            assert_eq!(tp.children()[1].children().len(), 3);
        }
    }
}
