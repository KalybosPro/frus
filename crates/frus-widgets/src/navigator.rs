//! [`Navigator`]: shows a full-window **screen**, with a slide transition
//! between the outgoing and the incoming screen on a push or a pop.
//!
//! The `Navigator` is **controlled**: the application holds the route stack and
//! the transition's progress, and (re)builds the screens on every frame.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A screen container with a slide transition.
pub struct Navigator<Msg> {
    width: f32,
    height: f32,
    /// Transition progress (`1.0` = no transition in flight).
    progress: f32,
    /// `true` = push (entering from the right), `false` = pop (entering from the left).
    forward: bool,
    /// `[screen]` or `[outgoing, incoming]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Navigator<Msg> {
    /// Shows a full-window screen (no transition).
    pub fn new(screen: impl Widget<Msg> + 'static, width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            progress: 1.0,
            forward: true,
            children: vec![Box::new(screen)],
        }
    }

    /// Adds the **outgoing** screen and the progress of a transition in flight.
    pub fn from(
        mut self,
        previous: impl Widget<Msg> + 'static,
        progress: f32,
        forward: bool,
    ) -> Self {
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
        Container::<()>::new()
            .width(400.0)
            .height(300.0)
            .color(color)
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
        assert!(has(red), "the outgoing screen is rendered");
        assert!(has(blue), "the incoming screen is rendered");
    }

    #[test]
    fn pop_parallaxes_and_orders_back_screen() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        // A pop half-way through: `red` = outgoing screen (front), `blue` = revealed back.
        let nav = Navigator::new(screen(blue), 400.0, 300.0).from(screen(red), 0.5, false);
        let ui = build_ui(
            &nav,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        let x_of = |c: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == c => Some(rect.x),
                    _ => None,
                })
                .expect("screen present")
        };
        let front = x_of(red); // +0.5·400 = 200
        let back = x_of(blue); // parallaxe : -0.5·400·0.3 = -60
        assert!(
            front > back,
            "the front ({front}) is to the right of the back ({back})"
        );
        // Without parallax the back would sit at -200; it is compressed toward 0.
        assert!(back > -200.0 && back < 0.0, "back parallaxed: {back}");
    }
}
