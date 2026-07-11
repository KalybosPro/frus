//! [`Switch`] : un interrupteur à bascule (pilule), **contrôlé**.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const W: f32 = 44.0;
const H: f32 = 24.0;
const MARGIN: f32 = 3.0;

/// Un interrupteur on/off.
pub struct Switch<Msg> {
    on: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Msg>>,
}

impl<Msg> Switch<Msg> {
    /// Crée un interrupteur dont l'état est fourni.
    pub fn new(on: bool) -> Self {
        Self {
            on,
            on_toggle: None,
        }
    }

    /// Closure produisant un message depuis le nouvel état.
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }
}

impl<Msg> Widget<Msg> for Switch<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(W),
            height: Dimension::Length(H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // `t` : position animée de la bascule (0 = off, 1 = on).
        let t = status.value;
        let track = theme.border.lerp(theme.primary, t);
        scene.draw_rect(bounds, track.fade(o), H * 0.5, 0.0, Color::TRANSPARENT);

        let d = H - MARGIN * 2.0;
        let off_x = bounds.x + MARGIN;
        let on_x = bounds.x + W - MARGIN - d;
        let thumb_x = off_x + (on_x - off_x) * t;
        scene.draw_rect(
            Rect::new(thumb_x, bounds.y + MARGIN, d, d),
            Color::WHITE.fade(o),
            d * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_toggle.as_ref().map(|make| make(!self.on))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn anim_target(&self) -> Option<f32> {
        Some(if self.on { 1.0 } else { 0.0 })
    }
}
