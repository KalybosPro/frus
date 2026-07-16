//! [`Dropdown`] : une liste déroulante **contrôlée** dont les options flottent
//! au-dessus du reste (via le mécanisme d'overlay), sous l'en-tête.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 240.0;
const ROW_H: f32 = 40.0;
const PAD_X: f32 = 12.0;
const SIZE: f32 = 18.0;

/// Une ligne (en-tête ou option).
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
        let bg = theme.state_layer(theme.surface, theme.on_surface, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            self.label.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
        if self.is_header {
            scene.text(
                Point::new(bounds.x + WIDTH - PAD_X - 12.0, ty),
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

/// Une liste déroulante à sélection unique (menu flottant).
pub struct Dropdown<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
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
            children: vec![Box::new(header)],
        }
    }

    /// Ajoute les options ; si `open`, elles flottent sous l'en-tête. `on_select`
    /// mappe l'index choisi vers un message.
    pub fn options(mut self, open: bool, labels: &[&str], on_select: impl Fn(usize) -> Msg) -> Self {
        if open {
            let mut menu = Flex::column().gap(4.0);
            for (index, label) in labels.iter().enumerate() {
                menu = menu.child(Row {
                    label: (*label).to_string(),
                    is_header: false,
                    on_click: Some(on_select(index)),
                });
            }
            self.children.truncate(1);
            self.children.push(Box::new(menu));
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Dropdown<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
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

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|menu| (menu.as_ref(), Placement::Below))
    }
}
