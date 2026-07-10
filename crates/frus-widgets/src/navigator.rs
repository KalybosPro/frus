//! [`Navigator`] : affiche un **écran** plein-fenêtre, avec une transition
//! glissée entre l'écran sortant et l'écran entrant lors d'un push/pop.
//!
//! Le `Navigator` est **contrôlé** : l'application tient la pile de routes et
//! l'avancement de la transition, et (re)construit les écrans à chaque frame.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un conteneur d'écran avec transition glissée.
pub struct Navigator<Msg> {
    width: f32,
    height: f32,
    /// Avancement de la transition (`1.0` = pas de transition en cours).
    progress: f32,
    /// `true` = push (entrée par la droite), `false` = pop (entrée par la gauche).
    forward: bool,
    /// `[écran]` ou `[sortant, entrant]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Navigator<Msg> {
    /// Affiche un écran plein-fenêtre (pas de transition).
    pub fn new(screen: impl Widget<Msg> + 'static, width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            progress: 1.0,
            forward: true,
            children: vec![Box::new(screen)],
        }
    }

    /// Ajoute l'écran **sortant** et l'avancement d'une transition en cours.
    pub fn from(mut self, previous: impl Widget<Msg> + 'static, progress: f32, forward: bool) -> Self {
        self.children.insert(0, Box::new(previous));
        self.progress = progress.clamp(0.0, 1.0);
        self.forward = forward;
        self
    }
}

impl<Msg> Widget<Msg> for Navigator<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
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

    fn navigator(&self) -> Option<(f32, bool)> {
        Some((self.progress, self.forward))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    fn screen(color: Color) -> Container<()> {
        Container::<()>::new().width(400.0).height(300.0).color(color)
    }

    #[test]
    fn transition_renders_both_screens() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let nav = Navigator::new(screen(blue), 400.0, 300.0).from(screen(red), 0.5, true);
        let ui = build_ui(
            &nav,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let has = |c: Color| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == c))
        };
        assert!(has(red), "l'écran sortant est rendu");
        assert!(has(blue), "l'écran entrant est rendu");
    }
}
