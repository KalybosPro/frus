//! [`Table`] : un tableau de données texte, bâti sur [`crate::Grid`]. En-tête
//! stylé + lignes de cellules ; colonnes égales.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::grid::Grid;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 34.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 15.0;

/// Une cellule (en-tête ou donnée), thémée au rendu.
struct Cell {
    label: String,
    header: bool,
}

impl<Msg> Widget<Msg> for Cell {
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
        if self.header {
            let bg = theme.surface.lerp(theme.on_surface, 0.06);
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }
        let color = if self.header { theme.muted } else { theme.on_surface };
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            self.label.clone(),
            SIZE,
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Un tableau à `columns` colonnes égales.
pub struct Table<Msg> {
    grid: Grid<Msg>,
}

impl<Msg> Table<Msg> {
    /// Crée un tableau de `columns` colonnes.
    pub fn new(columns: usize) -> Self {
        Self {
            grid: Grid::new(columns.max(1)).gap(2.0),
        }
    }

    /// Définit la ligne d'en-tête (une étiquette par colonne).
    pub fn header(mut self, labels: &[&str]) -> Self {
        for label in labels {
            self.grid = self.grid.cell(Cell {
                label: (*label).to_string(),
                header: true,
            });
        }
        self
    }

    /// Ajoute une ligne de données (une valeur par colonne).
    pub fn row(mut self, cells: &[&str]) -> Self {
        for cell in cells {
            self.grid = self.grid.cell(Cell {
                label: (*cell).to_string(),
                header: false,
            });
        }
        self
    }

    /// Fixe la largeur du tableau, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.grid = self.grid.width(width);
        self
    }
}

impl<Msg> Widget<Msg> for Table<Msg> {
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

    #[test]
    fn header_and_rows_produce_cells() {
        let table = Table::<()>::new(2)
            .header(&["Nom", "Note"])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // 2 colonnes × (1 en-tête + 2 lignes) = 6 cellules.
        assert_eq!(Widget::<()>::children(&table).len(), 6);

        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        // Les textes des cellules sont peints.
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Nom") && has("Ada") && has("Bob"));
    }
}
