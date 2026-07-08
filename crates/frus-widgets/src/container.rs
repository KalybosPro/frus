//! [`Container`] : une boîte avec taille, marge intérieure, couleur de fond
//! optionnelle, un enfant optionnel et un gestionnaire de clic optionnel.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::widget::Widget;

/// Une boîte rectangulaire décorée.
pub struct Container<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    padding: f32,
    color: Option<Color>,
    on_click: Option<Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Container<Msg> {
    /// Crée un conteneur vide (taille automatique, sans couleur ni clic).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            padding: 0.0,
            color: None,
            on_click: None,
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

    /// Couleur de fond.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Message émis lorsque le conteneur est cliqué.
    pub fn on_click(mut self, message: Msg) -> Self {
        self.on_click = Some(message);
        self
    }

    /// Définit l'enfant du conteneur.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for Container<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Container<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            padding: self.padding,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, scene: &mut Scene) {
        if let Some(color) = self.color {
            scene.fill_rect(bounds, color);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }
}
