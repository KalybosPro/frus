//! [`Rating`] : une note en **étoiles cliquables**, **contrôlée**.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const STAR: f32 = 24.0;

/// Une étoile (pleine ou vide), cliquable.
struct Star<Msg> {
    filled: bool,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Star<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(STAR),
            height: Dimension::Length(STAR),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let base = if self.filled {
            theme.primary
        } else {
            theme.muted.fade(0.45)
        };
        // Léger éclaircissement au survol.
        let color = base.lerp(theme.on_surface, 0.2 * status.hover_progress);
        scene.text(
            Point::new(bounds.x, bounds.y - 2.0),
            "★".to_string(),
            STAR,
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Une note en étoiles (`value` sur `max`).
pub struct Rating<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Rating<Msg> {
    /// Crée une note : `value` étoiles pleines sur `max` ; cliquer la i-ᵉ étoile
    /// émet `on_rate(i + 1)`.
    pub fn new(value: u32, max: u32, on_rate: impl Fn(u32) -> Msg + 'static) -> Self {
        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        for i in 0..max {
            children.push(Box::new(Star {
                filled: i < value,
                message: on_rate(i + 1),
            }));
        }
        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for Rating<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 4.0,
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Rate(u32),
    }

    #[test]
    fn has_max_stars_and_click_emits_rank() {
        let rating = Rating::new(2, 5, Msg::Rate);
        let stars = Widget::<Msg>::children(&rating);
        assert_eq!(stars.len(), 5);
        // Cliquer la 4ᵉ étoile (index 3) → note 4.
        assert_eq!(stars[3].on_click(), Some(Msg::Rate(4)));
    }

    #[test]
    fn filled_stars_match_value() {
        // On distingue pleines/vides par la couleur peinte.
        let rating = Rating::new(3, 5, Msg::Rate);
        let count_color = |filled_expected: bool| {
            let theme = Theme::default();
            let target = if filled_expected {
                theme.primary
            } else {
                theme.muted.fade(0.45)
            };
            (0..5)
                .filter(|&i| {
                    let mut scene = Scene::new();
                    Widget::<Msg>::children(&rating)[i].paint(
                        Rect::new(0.0, 0.0, STAR, STAR),
                        Status::default(),
                        &theme,
                        &mut scene,
                    );
                    matches!(scene.primitives()[0], frus_core::Primitive::Text { color, .. } if color == target)
                })
                .count()
        };
        assert_eq!(count_color(true), 3); // 3 pleines
        assert_eq!(count_color(false), 2); // 2 vides
    }
}
