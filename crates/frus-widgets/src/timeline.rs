//! [`Timeline`] : une chronologie verticale — des événements reliés par une
//! ligne, chacun marqué d'un point.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 56.0;
const LINE_X: f32 = 8.0;
const DOT: f32 = 10.0;
const TEXT_X: f32 = 28.0;

/// Un événement de la chronologie (point + ligne + textes).
struct Event {
    title: String,
    detail: String,
}

impl<Msg> Widget<Msg> for Event {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let cx = bounds.x + LINE_X;
        // Ligne continue (traverse toute la ligne ; les rangées se rejoignent).
        scene.fill_rect(
            Rect::new(cx - 1.0, bounds.y, 2.0, bounds.height),
            theme.border.fade(o),
        );
        // Point.
        let cy = bounds.y + ROW_H * 0.5;
        scene.draw_rect(
            Rect::new(cx - DOT * 0.5, cy - DOT * 0.5, DOT, DOT),
            theme.primary.fade(o),
            DOT * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // Titre + détail.
        scene.text(
            Point::new(bounds.x + TEXT_X, bounds.y + 8.0),
            self.title.clone(),
            16.0,
            theme.on_surface.fade(o),
        );
        scene.text(
            Point::new(bounds.x + TEXT_X, bounds.y + 30.0),
            self.detail.clone(),
            13.0,
            theme.muted.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Une chronologie verticale.
pub struct Timeline<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Timeline<Msg> {
    /// Crée une chronologie vide.
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    /// Ajoute un événement (titre + détail), du plus ancien au plus récent.
    pub fn event(mut self, title: impl Into<String>, detail: impl Into<String>) -> Self {
        self.children.push(Box::new(Event {
            title: title.into(),
            detail: detail.into(),
        }));
        self
    }
}

impl<Msg> Default for Timeline<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Timeline<Msg> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[test]
    fn events_paint_dots_and_text() {
        let timeline = Timeline::<()>::new()
            .event("Jalon 1", "fenêtre")
            .event("Jalon 2", "layout");
        assert_eq!(Widget::<()>::children(&timeline).len(), 2);

        let ui = build_ui(&timeline, Size::new(300.0, 200.0), &Runtime::default(), &Theme::default());
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Jalon 1") && has("Jalon 2"));
    }
}
