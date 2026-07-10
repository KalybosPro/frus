//! [`Scroll`] : un conteneur défilable verticalement.
//!
//! Il occupe un viewport de taille fixe ; son contenu (unique) est mis en page
//! à hauteur libre par le pilote, puis découpé au viewport et translaté selon
//! l'offset de défilement (retenu au runtime, clé par identité).

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Axe(s) de défilement d'un [`Scroll`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
    Both,
}

impl Axis {
    pub(crate) fn free_x(self) -> bool {
        matches!(self, Axis::Horizontal | Axis::Both)
    }
    pub(crate) fn free_y(self) -> bool {
        matches!(self, Axis::Vertical | Axis::Both)
    }
}

/// Un conteneur défilable.
pub struct Scroll<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    axis: Axis,
    content: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Scroll<Msg> {
    /// Crée une zone défilable (viewport 200 px de haut par défaut).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Length(200.0),
            flex_grow: 0.0,
            axis: Axis::Vertical,
            content: Vec::new(),
        }
    }

    /// Choisit le ou les axes de défilement (vertical par défaut).
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Fixe la largeur du viewport, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur du viewport, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Définit le contenu défilable.
    pub fn child(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.content.clear();
        self.content.push(Box::new(content));
        self
    }
}

impl<Msg> Default for Scroll<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Scroll<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Le viewport lui-même est transparent : seul le contenu est dessiné.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        self.content.first().map(|child| child.as_ref())
    }

    fn scroll_axis(&self) -> Axis {
        self.axis
    }
}
