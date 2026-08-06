//! [`DateTimeRange`] : une **plage date + heure** en un widget — le calendrier double de
//! [`crate::DatePicker::range_dual`] au-dessus de la plage horaire [`crate::TimeRange`], coiffés
//! d'un **récapitulatif** « début → fin ». Pour réserver un créneau complet : « du 28 juillet
//! 09:00 au 3 août 17:30 ».
//!
//! Purement composite et **contrôlé** : il combine les deux sous-sélecteurs et relaie leurs
//! messages ; l'état (dates, heures) vit dans l'application, qui décide quelle borne reçoit un
//! jour cliqué (comme la plage de dates).

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::datepicker::DatePicker;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::timepicker::{Endpoint, TimeField, TimeRange};
use crate::widget::Widget;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Un sélecteur de plage date + heure.
pub struct DateTimeRange<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> DateTimeRange<Msg> {
    /// Crée le sélecteur. La partie **date** montre `year`/`month` (1–12) et le mois suivant,
    /// surlignant la plage `[start_date, end_date]` (`on_date((année, mois, jour))` au clic,
    /// `on_nav(±1)` pour décaler la paire) ; la partie **heure** montre `start_time`/`end_time`
    /// (`on_time(borne, champ, valeur)`). Un récapitulatif « début → fin » coiffe le tout quand
    /// les **deux** dates sont posées.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: i32,
        month: u32,
        start_date: Option<(i32, u32, u32)>,
        end_date: Option<(i32, u32, u32)>,
        start_time: (u32, u32),
        end_time: (u32, u32),
        on_date: impl Fn((i32, u32, u32)) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
        on_time: impl Fn(Endpoint, TimeField, u32) -> Msg + 'static,
    ) -> Self {
        let calendar = DatePicker::range_dual(year, month, start_date, end_date, on_date, on_nav);
        let times = TimeRange::new(start_time, end_time, on_time);

        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        if let (Some(s), Some(e)) = (start_date, end_date) {
            let stamp = |d: (i32, u32, u32), t: (u32, u32)| {
                format!(
                    "{} {}, {}  {:02}:{:02}",
                    MONTHS[(d.1 - 1) as usize],
                    d.2,
                    d.0,
                    t.0,
                    t.1
                )
            };
            let summary = format!("{}  →  {}", stamp(s, start_time), stamp(e, end_time));
            children.push(Box::new(Text::new(summary).size(16.0)));
        }
        children.push(Box::new(calendar));
        children.push(Box::new(times));

        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for DateTimeRange<Msg> {
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
        Date(i32, u32, u32),
        Nav(i32),
        Time(Endpoint, TimeField, u32),
    }

    fn build(start: Option<(i32, u32, u32)>, end: Option<(i32, u32, u32)>) -> DateTimeRange<Msg> {
        DateTimeRange::new(
            2026,
            7,
            start,
            end,
            (9, 0),
            (17, 30),
            |(y, m, d)| Msg::Date(y, m, d),
            Msg::Nav,
            Msg::Time,
        )
    }

    #[test]
    fn summary_appears_only_with_both_dates() {
        // Aucune / une seule borne → [calendrier, heures]. Les deux → [récap, calendrier, heures].
        assert_eq!(Widget::<Msg>::children(&build(None, None)).len(), 2);
        assert_eq!(
            Widget::<Msg>::children(&build(Some((2026, 7, 28)), None)).len(),
            2
        );
        assert_eq!(
            Widget::<Msg>::children(&build(Some((2026, 7, 28)), Some((2026, 8, 3)))).len(),
            3,
        );
    }

    #[test]
    fn renders_the_combined_summary() {
        let dtr = build(Some((2026, 7, 28)), Some((2026, 8, 3)));
        let ui = build_ui(
            &dtr,
            Size::new(560.0, 700.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has = ui.scene().primitives().iter().any(|p| {
            matches!(p, Primitive::Text { text, .. }
                if text == "July 28, 2026  09:00  →  August 3, 2026  17:30")
        });
        assert!(has, "récapitulatif début → fin affiché");
    }
}
