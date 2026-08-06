//! [`RadioGroup`] : un groupe de boutons radio (une seule option sélectionnée).

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const DOT: f32 = 20.0;
const GAP: f32 = 10.0;

/// Une option de radio (usage interne au groupe).
struct RadioOption<Msg> {
    label: String,
    selected: bool,
    size: f32,
    on_click: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for RadioOption<Msg> {
    fn style(&self) -> Style {
        let line = frus_text::line_height(self.size).max(DOT);
        let label_w = frus_text::measure(&self.label, self.size).width;
        Style {
            width: Dimension::Length((DOT + GAP + label_w).ceil()),
            height: Dimension::Length(line.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let cy = bounds.y + (bounds.height - DOT) * 0.5;
        let outer = Rect::new(bounds.x, cy, DOT, DOT);
        let ring = if self.selected {
            theme.primary
        } else {
            theme.border
        };
        scene.draw_rect(outer, theme.surface.fade(o), DOT * 0.5, 2.0, ring.fade(o));
        if self.selected {
            let inner = DOT * 0.5;
            let pad = (DOT - inner) * 0.5;
            scene.draw_rect(
                Rect::new(outer.x + pad, outer.y + pad, inner, inner),
                theme.primary.fade(o),
                inner * 0.5,
                0.0,
                Color::TRANSPARENT,
            );
        }
        scene.text(
            Point::new(bounds.x + DOT + GAP, bounds.y),
            self.label.clone(),
            self.size,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }
}

/// Un groupe de boutons radio à sélection unique.
pub struct RadioGroup<Msg> {
    selected: usize,
    size: f32,
    gap: f32,
    on_select: Box<dyn Fn(usize) -> Msg>,
    options: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> RadioGroup<Msg> {
    /// Crée un groupe : index sélectionné + closure `index -> message`.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            size: 18.0,
            gap: 8.0,
            on_select: Box::new(on_select),
            options: Vec::new(),
        }
    }

    /// Ajoute une option (dans l'ordre).
    pub fn option(mut self, label: impl Into<String>) -> Self {
        let index = self.options.len();
        let message = (self.on_select)(index);
        self.options.push(Box::new(RadioOption {
            label: label.into(),
            selected: index == self.selected,
            size: self.size,
            on_click: Some(message),
        }));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for RadioGroup<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            gap: self.gap,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.options
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}
