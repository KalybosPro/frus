//! [`ColorPicker`]: a palette of clickable colour swatches, built on
//! [`crate::Grid`]. The selected swatch carries a ring.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::grid::Grid;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const SWATCH: f32 = 30.0;

/// Une pastille de couleur cliquable.
struct Swatch<Msg> {
    color: Color,
    selected: bool,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Swatch<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(SWATCH),
            height: Dimension::Length(SWATCH),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        scene.draw_rect(bounds, self.color.fade(o), 8.0, 0.0, Color::TRANSPARENT);
        if self.selected {
            // The selection ring, overflowing slightly.
            let ring = Rect::new(
                bounds.x - 2.0,
                bounds.y - 2.0,
                bounds.width + 4.0,
                bounds.height + 4.0,
            );
            scene.draw_rect(ring, Color::TRANSPARENT, 10.0, 2.0, theme.focus.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// A controlled colour picker, laid out as a grid of swatches.
pub struct ColorPicker<Msg> {
    selected: Option<Color>,
    on_pick: Box<dyn Fn(Color) -> Msg>,
    grid: Grid<Msg>,
}

impl<Msg: Clone + 'static> ColorPicker<Msg> {
    /// Creates a picker: the selected colour if there is one, `on_pick(colour)` for
    /// clicks, and a column count.
    pub fn new(
        selected: Option<Color>,
        columns: usize,
        on_pick: impl Fn(Color) -> Msg + 'static,
    ) -> Self {
        Self {
            selected,
            on_pick: Box::new(on_pick),
            grid: Grid::new(columns.max(1)).gap(8.0),
        }
    }

    /// Ajoute une pastille de couleur.
    pub fn swatch(mut self, color: Color) -> Self {
        let selected = self.selected == Some(color);
        let message = (self.on_pick)(color);
        self.grid = self.grid.cell(Swatch {
            color,
            selected,
            message,
        });
        self
    }
}

impl<Msg> Widget<Msg> for ColorPicker<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style(&self.grid)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        Widget::<Msg>::children(&self.grid)
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
        Pick(Color),
    }

    #[test]
    fn swatches_pick_and_selected_has_ring() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let picker = ColorPicker::new(Some(red), 4, Msg::Pick)
            .swatch(red)
            .swatch(blue);
        assert_eq!(Widget::<Msg>::children(&picker).len(), 2);

        let ui = build_ui(
            &picker,
            Size::new(200.0, 60.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The selection, red here, adds a ring, drawn as a focus border.
        let focus = Theme::default().focus;
        let has_ring = ui.scene().primitives().iter().any(|p| {
            matches!(p, Primitive::Rect { border_color, border_width, .. } if *border_width > 0.0 && *border_color == focus)
        });
        assert!(has_ring, "the selected swatch has a ring");
    }
}
