//! [`CustomPaint`] : une **toile libre** — le pendant du `CustomPainter` de
//! Flutter. Le widget réserve une boîte de taille fixe et confie sa peinture à
//! une closure fournie par l'application, qui dessine ce qu'elle veut (chemins
//! vectoriels, rectangles…) via la [`Scene`].
//!
//! La closure reçoit sa **boîte résolue** et le **thème** courant, pour se
//! thémer au moment de peindre (comme tous les widgets). Elle n'émet aucun
//! message : c'est un widget purement visuel.

use std::marker::PhantomData;

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Type de la fonction de peinture : `(scène, boîte, thème)`.
type PaintFn = dyn Fn(&mut Scene, Rect, &Theme);

/// Une toile personnalisée de taille fixe, peinte par une closure.
pub struct CustomPaint<Msg> {
    width: f32,
    height: f32,
    painter: Box<PaintFn>,
    // `fn() -> Msg` garde le paramètre sans imposer `Msg: Send`/covariance.
    _msg: PhantomData<fn() -> Msg>,
}

impl<Msg> CustomPaint<Msg> {
    /// Une toile `width×height` peinte par `painter`.
    pub fn new(width: f32, height: f32, painter: impl Fn(&mut Scene, Rect, &Theme) + 'static) -> Self {
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
            Primitive::Path { path, fill: Some(_), .. } => {
                // Le premier sommet suit la boîte passée (x=10, y=20).
                assert_eq!(
                    path.verbs().first(),
                    Some(&frus_core::PathVerb::MoveTo(Point::new(10.0, 20.0)))
                );
            }
            _ => panic!("attendu un chemin rempli"),
        }
    }
}
