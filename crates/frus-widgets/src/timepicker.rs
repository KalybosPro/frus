//! [`TimePicker`] : un sélecteur d'heure **contrôlé** (heures 0–23, minutes par pas de
//! 5), pendant de [`crate::DatePicker`]. Deux grilles de cases cliquables et un aperçu
//! `HH:MM` ; l'heure choisie vient de l'état applicatif, le widget émet au clic.

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
/// Pas des minutes proposées (0, 5, 10 … 55).
const MINUTE_STEP: u32 = 5;

/// Une case-nombre cliquable (heure ou minute), surlignée si sélectionnée.
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
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> TimePicker<Msg> {
    /// Crée un sélecteur pour `hour` (0–23) / `minute` (0–59). `on_hour(h)` au clic sur une
    /// heure, `on_minute(m)` sur une minute (multiples de 5). L'aperçu `HH:MM` reflète
    /// exactement `hour`/`minute` (même si la minute n'est pas un multiple de 5).
    pub fn new(
        hour: u32,
        minute: u32,
        on_hour: impl Fn(u32) -> Msg + 'static,
        on_minute: impl Fn(u32) -> Msg + 'static,
    ) -> Self {
        let hour = hour.min(23);
        let minute = minute.min(59);

        // Aperçu HH:MM.
        let header = Text::new(format!("{hour:02}:{minute:02}")).size(28.0);

        // Grille des heures (0–23, 6 colonnes).
        let mut hours = Grid::new(6).gap(4.0);
        for h in 0..24u32 {
            hours = hours.cell(TimeCell {
                label: format!("{h:02}"),
                selected: h == hour,
                message: Some(on_hour(h)),
            });
        }

        // Grille des minutes (pas de 5, 6 colonnes → 12 cases). La sélection ne
        // s'allume que si la minute courante tombe sur un pas.
        let mut minutes = Grid::new(6).gap(4.0);
        let mut m = 0;
        while m < 60 {
            minutes = minutes.cell(TimeCell {
                label: format!("{m:02}"),
                selected: m == minute,
                message: Some(on_minute(m)),
            });
            m += MINUTE_STEP;
        }

        let hours_section = Flex::column()
            .gap(6.0)
            .child(Text::new("Hour").size(13.0))
            .child(hours);
        let minutes_section = Flex::column()
            .gap(6.0)
            .child(Text::new("Minute").size(13.0))
            .child(minutes);

        Self {
            children: vec![Box::new(header), Box::new(hours_section), Box::new(minutes_section)],
        }
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

    #[test]
    fn builds_header_hours_and_minutes() {
        let tp = TimePicker::new(9, 30, Msg::Hour, Msg::Minute);
        // [aperçu, section heures, section minutes].
        assert_eq!(Widget::<Msg>::children(&tp).len(), 3);
        // Section heures = [label, grille] ; la grille a 24 cases.
        let hours_grid = &Widget::<Msg>::children(&tp)[1].children()[1];
        assert_eq!(hours_grid.children().len(), 24);
        // Section minutes = [label, grille] ; 60/5 = 12 cases.
        let minutes_grid = &Widget::<Msg>::children(&tp)[2].children()[1];
        assert_eq!(minutes_grid.children().len(), 12);
    }

    #[test]
    fn preview_and_selection_are_rendered() {
        let tp = TimePicker::new(9, 30, Msg::Hour, Msg::Minute);
        let ui = build_ui(&tp, Size::new(240.0, 320.0), &Runtime::default(), &Theme::default());
        // L'aperçu HH:MM est peint.
        let has_preview = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "09:30"));
        assert!(has_preview, "l'aperçu 09:30 est affiché");
        // Case sélectionnée surlignée en primary.
        let theme = Theme::default();
        let has_sel = ui.scene().primitives().iter().any(|p| matches!(
            p,
            Primitive::Rect { color, .. } if color.fade(1.0) == theme.primary.fade(1.0)
        ));
        assert!(has_sel, "l'heure/minute sélectionnée est surlignée");
    }

    #[test]
    fn clicking_a_cell_emits_the_hour_or_minute() {
        let tp = TimePicker::new(0, 0, Msg::Hour, Msg::Minute);
        let ui = build_ui(&tp, Size::new(240.0, 320.0), &Runtime::default(), &Theme::default());
        // La première case d'heures ("00") est en haut-gauche de sa grille : un clic y
        // émet Hour(0). On la localise par son identité via `hit`.
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        // La grille des heures suit l'aperçu ; la 1re case est vers le haut à gauche.
        // On balaie une bande pour trouver une cible d'heure.
        let msg = (0..320)
            .step_by(4)
            .find_map(|y| click(CELL * 0.5, y as f32))
            .expect("une case cliquable existe");
        assert!(matches!(msg, Msg::Hour(_) | Msg::Minute(_)));
    }
}
