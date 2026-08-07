//! [`CustomPaint`]: a **free canvas**. The widget reserves a fixed-size box and hands
//! its painting over to a closure the application supplies, which draws whatever it
//! likes — vector paths, rectangles and so on — through the [`Scene`].
//!
//! The closure receives its **resolved box** and the current **theme**, so it can theme
//! itself as it paints, as every widget does. It emits no message: this is a purely
//! visual widget.

use std::marker::PhantomData;

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The paint function's type: `(scene, box, theme)`.
type PaintFn = dyn Fn(&mut Scene, Rect, &Theme);

/// A fixed-size custom canvas, painted by a closure.
pub struct CustomPaint<Msg> {
    width: f32,
    height: f32,
    painter: Box<PaintFn>,
    // `fn() -> Msg` keeps the parameter without forcing `Msg: Send` or covariance.
    _msg: PhantomData<fn() -> Msg>,
}

impl<Msg> CustomPaint<Msg> {
    /// Une toile `width×height` peinte par `painter`.
    pub fn new(
        width: f32,
        height: f32,
        painter: impl Fn(&mut Scene, Rect, &Theme) + 'static,
    ) -> Self {
        Self {
            width,
            height,
            painter: Box::new(painter),
            _msg: PhantomData,
        }
    }
}

impl<Msg> Widget<Msg> for CustomPaint<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, _status: Status, theme: &Theme, scene: &mut Scene) {
        (self.painter)(scene, bounds, theme);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Path, Point, Primitive};

    #[test]
    fn invokes_the_closure_with_its_box() {
        let widget: CustomPaint<()> = CustomPaint::new(50.0, 30.0, |scene, bounds, _theme| {
            let tri = Path::new()
                .move_to(Point::new(bounds.x, bounds.y))
                .line_to(Point::new(bounds.x + bounds.width, bounds.y))
                .line_to(Point::new(bounds.x, bounds.y + bounds.height))
                .close();
            scene.fill_path(&tri, Color::rgb(0.2, 0.4, 0.6));
        });

        let mut scene = Scene::new();
        Widget::<()>::paint(
            &widget,
            Rect::new(10.0, 20.0, 50.0, 30.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );

        assert_eq!(scene.len(), 1);
        match &scene.primitives()[0] {
            Primitive::Path {
                path,
                fill: Some(_),
                ..
            } => {
                // The first vertex follows the box that was passed (x=10, y=20).
                assert_eq!(
                    path.verbs().first(),
                    Some(&frus_core::PathVerb::MoveTo(Point::new(10.0, 20.0)))
                );
            }
            _ => panic!("attendu un chemin rempli"),
        }
    }
}
