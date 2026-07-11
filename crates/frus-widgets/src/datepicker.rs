//! [`DatePicker`] : un calendrier mensuel **contrôlé**, bâti sur [`crate::Grid`].
//! Calcul de date **maison** (aucune dépendance temporelle).

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::grid::Grid;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

const CELL: f32 = 34.0;
const SIZE: f32 = 15.0;

/// Vrai si `year` est bissextile.
fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Nombre de jours du mois `month` (1..=12) de `year`.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Jour de la semaine (`0` = dimanche … `6` = samedi) du 1er du mois (Sakamoto).
fn first_weekday(year: i32, month: u32) -> usize {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let m = month as usize;
    (((y + y / 4 - y / 100 + y / 400 + T[m - 1] + 1) % 7 + 7) % 7) as usize
}

/// Une case-jour cliquable (vide si `day == 0`).
struct Day<Msg> {
    day: u32,
    selected: bool,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Day<Msg> {
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
        if self.day == 0 {
            return; // case de remplissage
        }
        let o = status.opacity;
        let (bg, fg) = if self.selected {
            (theme.primary, theme.on_primary)
        } else {
            let hovered = theme.surface.lerp(theme.on_surface, 0.08 * status.hover_progress);
            (hovered, theme.on_surface)
        };
        scene.draw_rect(bounds, bg.fade(o), CELL * 0.5, 0.0, Color::TRANSPARENT);
        let label = self.day.to_string();
        let w = frus_text::measure(&label, SIZE).width;
        scene.text(
            Point::new(
                bounds.x + (CELL - w) * 0.5,
                bounds.y + (CELL - frus_text::line_height(SIZE)) * 0.5,
            ),
            label,
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

/// Un calendrier mensuel.
pub struct DatePicker<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

const WEEKDAYS: [&str; 7] = ["D", "L", "M", "M", "J", "V", "S"];
const MONTHS: [&str; 12] = [
    "Janvier", "Février", "Mars", "Avril", "Mai", "Juin", "Juillet", "Août", "Septembre",
    "Octobre", "Novembre", "Décembre",
];

impl<Msg: Clone + 'static> DatePicker<Msg> {
    /// Crée un calendrier pour `year`/`month` (1..=12), avec le jour `selected`
    /// éventuel. `on_select(jour)` au clic ; `on_nav(±1)` pour changer de mois.
    pub fn new(
        year: i32,
        month: u32,
        selected: Option<u32>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);

        // En-tête : ‹ Mois Année ›.
        let header = Flex::row()
            .align(Align::Center)
            .gap(8.0)
            .child(Button::new("‹").variant(Variant::Secondary).size(15.0).on_press(on_nav(-1)))
            .child(Flex::row().flex(1.0))
            .child(Text::new(format!("{} {}", MONTHS[(month - 1) as usize], year)).size(16.0))
            .child(Flex::row().flex(1.0))
            .child(Button::new("›").variant(Variant::Secondary).size(15.0).on_press(on_nav(1)));

        // Ligne des jours de la semaine.
        let mut weekdays = Grid::new(7).gap(2.0);
        for wd in WEEKDAYS {
            weekdays = weekdays.cell(WeekdayCell {
                label: wd.to_string(),
            });
        }

        // Grille des jours (cases vides avant le 1er).
        let lead = first_weekday(year, month);
        let total = days_in_month(year, month);
        let mut grid = Grid::new(7).gap(2.0);
        for _ in 0..lead {
            grid = grid.cell(Day::<Msg> {
                day: 0,
                selected: false,
                message: None,
            });
        }
        for day in 1..=total {
            grid = grid.cell(Day {
                day,
                selected: selected == Some(day),
                message: Some(on_select(day)),
            });
        }

        Self {
            children: vec![Box::new(header), Box::new(weekdays), Box::new(grid)],
        }
    }
}

/// Un en-tête de jour de semaine (non cliquable).
struct WeekdayCell {
    label: String,
}

impl<Msg> Widget<Msg> for WeekdayCell {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(22.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let w = frus_text::measure(&self.label, 13.0).width;
        scene.text(
            Point::new(bounds.x + (CELL - w) * 0.5, bounds.y),
            self.label.clone(),
            13.0,
            theme.muted.fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

impl<Msg: Clone> Widget<Msg> for DatePicker<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(7.0 * (CELL + 2.0)),
            flex_direction: FlexDirection::Column,
            gap: 8.0,
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pick(u32),
        Nav(i32),
    }

    #[test]
    fn date_math_is_correct() {
        assert_eq!(days_in_month(2024, 2), 29); // bissextile
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        // 1er janvier 2024 = lundi (1).
        assert_eq!(first_weekday(2024, 1), 1);
        // 1er juillet 2026 = mercredi (3).
        assert_eq!(first_weekday(2026, 7), 3);
    }

    #[test]
    fn builds_header_weekdays_and_grid() {
        let dp = DatePicker::new(2026, 7, Some(11), Msg::Pick, Msg::Nav);
        // [en-tête, jours de semaine, grille].
        assert_eq!(Widget::<Msg>::children(&dp).len(), 3);
        // La grille = cases de remplissage + 31 jours ; juillet 2026 commence mercredi
        // (3 cases vides) → 3 + 31 = 34 cellules.
        let grid = &Widget::<Msg>::children(&dp)[2];
        assert_eq!(grid.children().len(), 34);
    }
}
