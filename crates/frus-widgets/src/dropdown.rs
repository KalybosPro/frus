//! [`Dropdown`] : une liste déroulante **contrôlée** (l'état ouvert vient de
//! l'application). L'ouverture déploie les options **en place** (sous l'en-tête).

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 240.0;
const ROW_H: f32 = 40.0;
const PAD_X: f32 = 12.0;
const SIZE: f32 = 18.0;

/// Une ligne (en-tête ou option) de la liste déroulante.
struct Row<Msg> {
    label: String,
    is_header: bool,
    on_click: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(WIDTH),
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let bg = theme
            .surface
            .lerp(theme.on_surface, 0.06 * status.hover_progress);
        let border = if self.is_header { theme.border } else { theme.border.fade(0.5) };
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, border.fade(o));
        scene.text(
            Point::new(bounds.x + PAD_X, bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5),
            self.label.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
        if self.is_header {
            scene.text(
                Point::new(bounds.x + WIDTH - PAD_X - 12.0, bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5),
                "▾".to_string(),
                SIZE,
                theme.muted.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }
}

/// Une liste déroulante à sélection unique.
pub struct Dropdown<Msg> {
    rows: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Dropdown<Msg> {
    /// Crée une liste : libellé courant + message de bascule (ouvrir/fermer).
    pub fn new(selected_label: impl Into<String>, on_toggle: Msg) -> Self {
        let header = Row {
            label: selected_label.into(),
            is_header: true,
            on_click: Some(on_toggle),
        };
        Self {
            rows: vec![Box::new(header)],
        }
    }

    /// Ajoute les options ; si `open`, elles sont déployées. `on_select` mappe
    /// l'index choisi vers un message.
    pub fn options(
        mut self,
        open: bool,
        labels: &[&str],
        on_select: impl Fn(usize) -> Msg,
    ) -> Self {
        if open {
            for (index, label) in labels.iter().enumerate() {
                self.rows.push(Box::new(Row {
                    label: (*label).to_string(),
                    is_header: false,
                    on_click: Some(on_select(index)),
                }));
            }
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Dropdown<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            gap: 4.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.rows
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}
