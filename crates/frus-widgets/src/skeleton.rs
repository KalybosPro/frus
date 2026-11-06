// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! [`Skeleton`]: a loading placeholder whose intensity **pulses** over time, a
//! shimmer. It reuses the continuous clock (`Status::time` plus `continuous`).

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Vitesse de pulsation (radians/seconde).
const SPEED: f32 = 2.5;

/// An animated loading block.
pub struct Skeleton {
    width: Dimension,
    height: f32,
    radius: f32,
}

impl Skeleton {
    /// Creates a placeholder: full width, and 16 tall by default.
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: 16.0,
            radius: 6.0,
        }
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets the corner radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Skeleton {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn continuous(&self) -> bool {
        true
    }

    /// **The width offered, when it was given none** (milestone 759).
    ///
    /// A placeholder stands in for a line of content, and a line takes the width of its
    /// column. With no width of its own and no children to measure, a placeholder left to
    /// its content came out zero pixels wide wherever its parent did not stretch it (a box
    /// that only caps the width, a column aligned to the start), and drew nothing at all.
    /// An empty box of a given height takes the width its parent allows; so does this.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        if matches!(self.width, Dimension::Auto) {
            crate::widget::FillAxes::WIDTH
        } else {
            crate::widget::FillAxes::NONE
        }
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A `0..1` pulse driven by time.
        let pulse = 0.5 + 0.5 * (status.time * SPEED).sin();
        let alpha = (0.18 + 0.22 * pulse) * o;
        scene.draw_rect(
            bounds,
            theme.muted.fade(alpha),
            self.radius,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn alpha_at(time: f32) -> f32 {
        let sk = Skeleton::new().width(120.0);
        let status = Status {
            time,
            ..Default::default()
        };
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &sk,
            Rect::new(0.0, 0.0, 120.0, 16.0),
            status,
            &Theme::default(),
            &mut scene,
        );
        match scene.primitives()[0] {
            Primitive::Rect { color, .. } => color.a,
            _ => panic!("expected a rect"),
        }
    }

    #[test]
    fn is_continuous() {
        assert!(Widget::<()>::continuous(&Skeleton::new()));
    }

    #[test]
    fn intensity_varies_with_time() {
        // Two distinct instants of the pulse give different opacities.
        assert_ne!(alpha_at(0.0), alpha_at(0.6));
    }

    /// The bars a scene draws, wherever they are nested.
    fn bars(ps: &[Primitive], out: &mut Vec<Rect>) {
        for p in ps {
            match p {
                Primitive::Rect { rect, .. } => out.push(*rect),
                Primitive::Layer { primitives, .. } => bars(primitives, out),
                _ => {}
            }
        }
    }

    /// **A placeholder with no width takes the width it is offered** (milestone 759). In a
    /// box that caps the width but does not size its child, in a start-aligned column — the
    /// shape of a loading line under other content — it came out zero pixels wide and drew
    /// nothing.
    #[test]
    fn a_placeholder_takes_the_width_it_is_offered() {
        let boxed = crate::ConstrainedBox::new(Skeleton::new()).max_width(300.0);
        let screen = crate::Flex::<()>::column()
            .align(frus_layout::Align::Start)
            .child(boxed);
        let ui = crate::build_ui(
            &screen,
            crate::Size::new(400.0, 300.0),
            &crate::Runtime::default(),
            &Theme::default(),
        );
        let mut found = Vec::new();
        bars(ui.scene().primitives(), &mut found);
        assert!(
            found
                .iter()
                .any(|r| r.height == 16.0 && (r.width - 300.0).abs() < 0.5),
            "a 300 x 16 bar: {found:?}"
        );
    }

    /// One given a width keeps it.
    #[test]
    fn a_placeholder_given_a_width_keeps_it() {
        assert_eq!(
            Widget::<()>::fill_axes(&Skeleton::new().width(80.0), &Theme::default()),
            crate::widget::FillAxes::NONE
        );
    }
}
