//! [`Flex`] : un conteneur qui dispose ses enfants en rangée ou en colonne.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::widget::Widget;

/// Un conteneur flex (rangée ou colonne). Ne peint aucune décoration propre.
pub struct Flex<Msg> {
    direction: FlexDirection,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    padding: f32,
    gap: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Flex<Msg> {
    /// Conteneur disposant ses enfants horizontalement.
    pub fn row() -> Self {
        Self::with_direction(FlexDirection::Row)
    }

    /// Conteneur disposant ses enfants verticalement.
    pub fn column() -> Self {
        Self::with_direction(FlexDirection::Column)
    }

    fn with_direction(direction: FlexDirection) -> Self {
        Self {
            direction,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            padding: 0.0,
            gap: 0.0,
            children: Vec::new(),
        }
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

    /// Marge intérieure uniforme, en pixels logiques.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Espacement entre enfants, en pixels logiques.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Ajoute un enfant.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Flex<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            flex_direction: self.direction,
            padding: self.padding,
            gap: self.gap,
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _scene: &mut Scene) {
        // Un conteneur flex est transparent : pas de décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}
