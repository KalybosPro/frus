//! [`Image`]: displays a bitmap image ([`frus_core::ImageHandle`]) in a fixed-size
//! box, fitted according to a [`BoxFit`] — aspect preserved, letterboxed or cropped.
//! The image is uploaded **once** then cached by the renderer, keyed on the
//! `ImageData`'s identity.

use frus_core::{BoxFit, Color, ImageHandle, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// An image displayed in a `width×height` box, fitted by `fit`.
pub struct Image {
    image: ImageHandle,
    width: f32,
    height: f32,
    fit: BoxFit,
    tint: Option<Color>,
}

impl Image {
    /// A `width×height` image, fitted with [`BoxFit::Contain`] by default.
    pub fn new(image: ImageHandle, width: f32, height: f32) -> Self {
        Self {
            image,
            width,
            height,
            fit: BoxFit::Contain,
            tint: None,
        }
    }

    /// Changes the fit mode.
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    /// A multiplied tint, white leaving it unchanged — for bitmap icons, say.
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }
}

impl<Msg> Widget<Msg> for Image {
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

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        let (dst, uv) = self.fit.apply(self.image.size(), bounds);
        let tint = self.tint.unwrap_or(Color::WHITE).fade(status.opacity);
        scene.draw_image(&self.image, dst, uv, tint);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{ImageData, Primitive};

    fn handle(w: u32, h: u32) -> ImageHandle {
        ImageData::from_rgba(w, h, vec![255u8; (w * h * 4) as usize]).into_handle()
    }

    fn paint(image: Image, bounds: Rect) -> Primitive {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &image,
            bounds,
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives()[0].clone()
    }

    #[test]
    fn contain_letterboxes_a_square_in_a_wide_box() {
        // A square 10×10 source in a 100×40 box gives 40×40, centred in x.
        let prim = paint(
            Image::new(handle(10, 10), 100.0, 40.0),
            Rect::new(0.0, 0.0, 100.0, 40.0),
        );
        match prim {
            Primitive::Image { rect, uv, .. } => {
                assert_eq!(rect, Rect::new(30.0, 0.0, 40.0, 40.0));
                assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
            }
            _ => panic!("expected an image"),
        }
    }

    #[test]
    fn tint_override_is_applied() {
        let prim = paint(
            Image::new(handle(4, 4), 20.0, 20.0).tint(Color::rgb(1.0, 0.0, 0.0)),
            Rect::new(0.0, 0.0, 20.0, 20.0),
        );
        match prim {
            Primitive::Image { tint, .. } => {
                assert_eq!(tint.r, 1.0);
                assert_eq!(tint.g, 0.0);
            }
            _ => panic!("expected an image"),
        }
    }

    #[test]
    fn size_drives_the_layout_box() {
        let image = Image::new(handle(4, 4), 64.0, 48.0);
        let style = Widget::<()>::style(&image);
        assert_eq!(style.width, Dimension::Length(64.0));
        assert_eq!(style.height, Dimension::Length(48.0));
    }
}
