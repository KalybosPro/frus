//! [`TimePicker`] : un sélecteur d'heure **contrôlé**, pendant de [`crate::DatePicker`].
//! Deux grilles de cases cliquables (heures, minutes) et un aperçu ; en 24 h par défaut,
//! ou 12 h avec bascule AM/PM ([`hour12`](TimePicker::hour12)). Le pas des minutes est
//! réglable ([`minute_step`](TimePicker::minute_step)). L'heure vient de l'état
//! applicatif ; le widget émet au clic, toujours en **heure 24 h**.

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

/// Une case-nombre cliquable (heure, minute ou AM/PM), surlignée si sélectionnée.
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
            (theme.state_layer(theme.surface, theme.on_surface, &status), theme.on_surface)
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

/// Un sélecteur d'heure.
pub struct TimePicker<Msg> {
    hour: u32,
    minute: u32,
    on_hour: Box<dyn Fn(u32) -> Msg>,
    on_minute: Box<dyn Fn(u32) -> Msg>,
    hour12: bool,
    minute_step: u32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

/// Chiffre d'horloge 12 h (1–12) d'une heure 24 h.
fn digit12(hour24: u32) -> u32 {
    if hour24 % 12 == 0 {
        12
    } else {
        hour24 % 12
    }
}

impl<Msg: Clone + 'static> TimePicker<Msg> {
    /// Crée un sélecteur pour `hour` (0–23) / `minute` (0–59). `on_hour(h)` est émis au
    /// clic sur une heure (toujours en **24 h**), `on_minute(m)` sur une minute. Par
    /// défaut : 24 h, minutes par pas de 5.
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

    /// Bascule en **12 h** : grille de 1 à 12 + bascule AM/PM ; l'aperçu s'affiche en 12 h.
    /// Les messages émis restent en heure 24 h.
    pub fn hour12(mut self) -> Self {
        self.hour12 = true;
        self.rebuild();
        self
    }

    /// Règle le **pas des minutes** proposées (1–60, borné). Par défaut 5.
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = step.clamp(1, 60);
        self.rebuild();
        self
    }

    /// Assemble l'aperçu et les grilles à partir de l'état courant.
    fn rebuild(&mut self) {
        let (hour, minute) = (self.hour, self.minute);
        let pm = hour >= 12;

        // Aperçu HH:MM (+ AM/PM en 12 h).
        let preview = if self.hour12 {
            let suffix = if pm { "PM" } else { "AM" };
            Text::new(format!("{}:{minute:02} {suffix}", digit12(hour))).size(28.0)
        } else {
            Text::new(format!("{hour:02}:{minute:02}")).size(28.0)
        };

        // Section des heures.
        let hours_section = if self.hour12 {
            // Bascule AM/PM : cliquer une moitié y bascule l'heure courante.
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
            // Grille 1–12 ; chaque case vise l'heure 24 h de la moitié courante.
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
            Flex::column().gap(6.0).child(Text::new("Hour").size(13.0)).child(grid)
        };

        // Section des minutes (pas réglable). La sélection ne s'allume que si la minute
        // courante tombe sur un pas.
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
        let minutes_section =
            Flex::column().gap(6.0).child(Text::new("Minute").size(13.0)).child(minutes);

        self.children =
            vec![Box::new(preview), Box::new(hours_section), Box::new(minutes_section)];
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

        let ui = build_ui(&tp, Size::new(240.0, 360.0), &Runtime::default(), &Theme::default());
        assert_eq!(preview_text(&ui).as_deref(), Some("3:05 PM"));
    }

    #[test]
    fn twentyfour_hour_preview() {
        let tp = TimePicker::new(9, 30, Msg::Hour, Msg::Minute);
        let ui = build_ui(&tp, Size::new(240.0, 320.0), &Runtime::default(), &Theme::default());
        assert_eq!(preview_text(&ui).as_deref(), Some("09:30"));
    }

    #[test]
    fn clicking_a_cell_emits_a_message() {
        let tp = TimePicker::new(0, 0, Msg::Hour, Msg::Minute);
        let ui = build_ui(&tp, Size::new(240.0, 320.0), &Runtime::default(), &Theme::default());
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        let msg = (0..320)
            .step_by(4)
            .find_map(|y| click(CELL * 0.5, y as f32))
            .expect("une case cliquable existe");
        assert!(matches!(msg, Msg::Hour(_) | Msg::Minute(_)));
    }
}
