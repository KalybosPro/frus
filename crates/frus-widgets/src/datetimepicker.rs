//! [`DateTimePicker`] : un **flux date + heure** en un seul widget (façon
//! `showDateTimePicker`) — le calendrier [`crate::DatePicker`] au-dessus du sélecteur
//! d'heure [`crate::TimePicker`], coiffés d'un **récapitulatif** de la sélection.
//!
//! Purement composite et contrôlé : il ne fait que combiner les deux sous-sélecteurs et
//! relayer leurs messages ; l'état (date, heure) vit dans l'application.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::datepicker::DatePicker;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::timepicker::TimePicker;
use crate::widget::Widget;

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

/// Un sélecteur date + heure.
pub struct DateTimePicker<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> DateTimePicker<Msg> {
    /// Crée un flux date + heure. La partie **date** affiche `year`/`month` (1–12) avec le
    /// `day` éventuellement sélectionné (`on_day(jour)` au clic, `on_nav(±1)` pour changer
    /// de mois) ; la partie **heure** affiche `hour`/`minute` (`on_hour`/`on_minute`). Un
    /// récapitulatif « Mois jour, année  HH:MM » coiffe le tout quand un jour est choisi.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: i32,
        month: u32,
        day: Option<u32>,
        hour: u32,
        minute: u32,
        on_day: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
        on_hour: impl Fn(u32) -> Msg + 'static,
        on_minute: impl Fn(u32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        let (hour, minute) = (hour.min(23), minute.min(59));

        let date = DatePicker::new(year, month, day, on_day, on_nav);
        let time = TimePicker::new(hour, minute, on_hour, on_minute);

        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        if let Some(day) = day {
            let summary =
                format!("{} {day}, {year}  {hour:02}:{minute:02}", MONTHS[(month - 1) as usize]);
            children.push(Box::new(Text::new(summary).size(16.0)));
        }
        children.push(Box::new(date));
        children.push(Box::new(time));

        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for DateTimePicker<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            gap: 16.0,
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
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Day(u32),
        Nav(i32),
        Hour(u32),
        Minute(u32),
    }

    #[test]
    fn summary_appears_only_when_a_day_is_picked() {
        // Sans jour : [date, heure]. Avec jour : [récap, date, heure].
        let without =
            DateTimePicker::new(2026, 7, None, 9, 30, Msg::Day, Msg::Nav, Msg::Hour, Msg::Minute);
        assert_eq!(Widget::<Msg>::children(&without).len(), 2);

        let with = DateTimePicker::new(
            2026,
            7,
            Some(11),
            9,
            30,
            Msg::Day,
            Msg::Nav,
            Msg::Hour,
            Msg::Minute,
        );
        assert_eq!(Widget::<Msg>::children(&with).len(), 3);
    }

    #[test]
    fn renders_the_combined_summary() {
        let dt = DateTimePicker::new(
            2026,
            7,
            Some(11),
            9,
            30,
            Msg::Day,
            Msg::Nav,
            Msg::Hour,
            Msg::Minute,
        );
        let ui = build_ui(&dt, Size::new(280.0, 520.0), &Runtime::default(), &Theme::default());
        let has_summary = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "July 11, 2026  09:30"));
        assert!(has_summary, "le récapitulatif date + heure est affiché");
    }
}
