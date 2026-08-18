//! [`Placeholder`]: a crossed box standing in for a widget not written yet.
//!
//! A design tool rather than a control: it takes whatever room it is given, draws a
//! rectangle with its two diagonals, and makes the shape of a layout readable before any
//! of it is built. The reference's defaults — blue grey, a 2 px stroke, and 400×400 when
//! nothing constrains it — are the defaults here, and all three are overridable.

use frus_core::{BorderRadius, Color, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The reference's colour: blue grey 700.
pub const PLACEHOLDER_COLOR: Color = Color {
    r: 0x45 as f32 / 255.0,
    g: 0x5A as f32 / 255.0,
    b: 0x64 as f32 / 255.0,
    a: 1.0,
};
/// The reference's stroke width.
pub const PLACEHOLDER_STROKE: f32 = 2.0;
/// The size it takes when nothing constrains it.
pub const PLACEHOLDER_FALLBACK: f32 = 400.0;

/// A crossed box standing in for a widget not written yet.
pub struct Placeholder {
    color: Option<Color>,
    stroke: f32,
    width: f32,
    height: f32,
}

impl Default for Placeholder {
    fn default() -> Self {
        Self::new()
    }
}

impl Placeholder {
    /// A placeholder at the reference's defaults.
    pub fn new() -> Self {
        Self {
            color: None,
            stroke: PLACEHOLDER_STROKE,
            width: PLACEHOLDER_FALLBACK,
            height: PLACEHOLDER_FALLBACK,
        }
    }

    /// The colour of the box and its diagonals.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The stroke width.
    pub fn stroke(mut self, stroke: f32) -> Self {
        self.stroke = stroke;
        self
    }

    /// The size to take when nothing constrains it.
    pub fn fallback(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl<Msg> Widget<Msg> for Placeholder {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            // It is a stand-in: it should take the room on offer rather than insist on
            // 400 px and push the layout it is meant to be illustrating out of shape.
            flex_grow: 1.0,
            min_width: Dimension::Length(0.0),
            min_height: Dimension::Length(0.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self.color.unwrap_or(PLACEHOLDER_COLOR).fade(status.opacity);
        let _ = theme;
        scene.draw_rect(
            bounds,
            Color::TRANSPARENT,
            BorderRadius::ZERO,
            self.stroke,
            color,
        );
        // The two diagonals, as strokes rather than a filled shape, so a thin placeholder
        // still reads as a cross rather than as a solid wedge.
        let half = self.stroke / 2.0;
        let (right, bottom) = (bounds.x + bounds.width, bounds.y + bounds.height);
        for (x0, y0, x1, y1) in [
            (bounds.x, bounds.y, right, bottom),
            (bounds.x, bottom, right, bounds.y),
        ] {
            let (dx, dy) = (x1 - x0, y1 - y0);
            let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);
            let (nx, ny) = (-dy / len * half, dx / len * half);
            let path = Path::new()
                .move_to(Point::new(x0 + nx, y0 + ny))
                .line_to(Point::new(x1 + nx, y1 + ny))
                .line_to(Point::new(x1 - nx, y1 - ny))
                .line_to(Point::new(x0 - nx, y0 - ny))
                .close();
            scene.fill_path(&path, color);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Primitive, Size};

    fn primitives(w: &dyn Widget<()>) -> Vec<Primitive> {
        let mut scene = Scene::new();
        w.paint(
            Rect::new(0.0, 0.0, 100.0, 50.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let _ = Size::new(0.0, 0.0);
        scene.primitives().to_vec()
    }

    /// A box and two diagonals — the reference's picture.
    #[test]
    fn it_draws_a_box_and_its_two_diagonals() {
        let p = Placeholder::new();
        let painted = primitives(&p);
        let rects = painted
            .iter()
            .filter(|x| matches!(x, Primitive::Rect { .. }))
            .count();
        let paths = painted
            .iter()
            .filter(|x| matches!(x, Primitive::Path { .. }))
            .count();
        assert_eq!(rects, 1, "the outline");
        assert_eq!(paths, 2, "the two diagonals");
    }

    /// The colour is the caller's when they gave one, and the reference's otherwise.
    #[test]
    fn the_colour_is_overridable() {
        let default_color = match &primitives(&Placeholder::new())[0] {
            Primitive::Rect { border_color, .. } => *border_color,
            other => panic!("{other:?}"),
        };
        assert_eq!(default_color, PLACEHOLDER_COLOR);

        let asked = Color::rgb(1.0, 0.0, 0.0);
        let mine = match &primitives(&Placeholder::new().color(asked))[0] {
            Primitive::Rect { border_color, .. } => *border_color,
            other => panic!("{other:?}"),
        };
        assert_eq!(mine, asked);
    }

    /// It takes the room on offer rather than insisting on its 400 px fallback.
    #[test]
    fn it_gives_way_to_the_layout_around_it() {
        let style = Widget::<()>::style(&Placeholder::new());
        assert_eq!(style.flex_grow, 1.0);
        assert_eq!(style.min_width, Dimension::Length(0.0));
        assert_eq!(style.width, Dimension::Length(PLACEHOLDER_FALLBACK));
    }
}
