//! [`Grid`] : un conteneur en **grille** à colonnes égales. Les cellules se
//! placent automatiquement, ligne par ligne ; la hauteur suit le contenu.
//!
//! Contrairement aux composites, `Grid` est un **conteneur normal** : la
//! disposition est faite par le moteur de layout (CSS Grid de taffy), donc
//! `cell()` n'est qu'un ajout d'enfant.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Une grille à `columns` colonnes égales.
pub struct Grid<Msg> {
    columns: usize,
    gap: f32,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Grid<Msg> {
    /// Crée une grille de `columns` colonnes (au moins 1).
    pub fn new(columns: usize) -> Self {
        Self {
            columns: columns.max(1),
            gap: 0.0,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            children: Vec::new(),
        }
    }

    /// Espacement entre cellules (lignes et colonnes), en pixels logiques.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Ajoute une cellule (placée automatiquement dans la grille).
    pub fn cell(mut self, cell: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(cell));
        self
    }
}

impl<Msg> Widget<Msg> for Grid<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            gap: self.gap,
            grid_columns: Some(self.columns),
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
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    fn tile(c: Color) -> Container<()> {
        Container::<()>::new().height(20.0).color(c)
    }

    #[test]
    fn cells_flow_into_rows_and_columns() {
        let a = Color::rgb(1.0, 0.0, 0.0);
        let b = Color::rgb(0.0, 1.0, 0.0);
        let c = Color::rgb(0.0, 0.0, 1.0);
        let d = Color::rgb(1.0, 1.0, 0.0);
        // Grille 2 colonnes : [a b] / [c d].
        let grid = Grid::<()>::new(2)
            .gap(10.0)
            .width(220.0)
            .cell(tile(a))
            .cell(tile(b))
            .cell(tile(c))
            .cell(tile(d));
        let ui = build_ui(&grid, Size::new(220.0, 200.0), &Runtime::default(), &Theme::default());

        let rect_of = |col: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == col => Some(*rect),
                    _ => None,
                })
                .expect("cellule présente")
        };
        let (ra, rb, rc, rd) = (rect_of(a), rect_of(b), rect_of(c), rect_of(d));

        // a et b : même ligne (même y), colonnes différentes (b à droite de a).
        assert_eq!(ra.y, rb.y);
        assert!(rb.x > ra.x);
        // c est sous a (ligne suivante), même colonne (même x).
        assert!(rc.y > ra.y);
        assert_eq!(rc.x, ra.x);
        // d aligné avec b en x, avec c en y.
        assert_eq!(rd.x, rb.x);
        assert_eq!(rd.y, rc.y);
        // Colonnes égales : largeur de a == largeur de b.
        assert_eq!(ra.width, rb.width);
    }
}
