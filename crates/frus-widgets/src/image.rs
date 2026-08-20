//! [`Image`]: displays a bitmap image ([`frus_core::ImageHandle`]), fitted according to
//! a [`BoxFit`] — aspect preserved, letterboxed or cropped. The image is uploaded
//! **once** then cached by the renderer, keyed on the `ImageData`'s identity.
//!
//! **How big is it?** Whatever you say, and failing that whatever the bitmap is. The
//! reference's rule, and the only one that lets an image be shown by someone who does not
//! already know its pixel dimensions:
//!
//! | given | box |
//! |---|---|
//! | width and height | that box |
//! | one of the two | the other from the image's own ratio |
//! | neither | the image's own size |

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Color, ImageHandle, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// An image, fitted by `fit` into whatever box it ends up with.
pub struct Image {
    image: ImageHandle,
    width: Option<f32>,
    height: Option<f32>,
    fit: BoxFit,
    tint: Option<Color>,
    opacity: f32,
    alignment: AlignmentGeometry,
}

impl Image {
    /// An image at **its own size**, fitted with [`BoxFit::Contain`] by default.
    pub fn new(image: ImageHandle) -> Self {
        Self {
            image,
            width: None,
            height: None,
            fit: BoxFit::Contain,
            tint: None,
            opacity: 1.0,
            alignment: AlignmentGeometry::Physical(Alignment::CENTER),
        }
    }

    /// Sets the width, in logical pixels. With no height, the height follows from the
    /// image's own ratio.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the height, in logical pixels. With no width, the width follows from the
    /// image's own ratio.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Both at once.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.width(width).height(height)
    }

    /// Changes the fit mode.
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    /// Where the image sits in a box it does not fill, and which part of it survives a
    /// box it overflows. Centre by default, as in the reference.
    ///
    /// It is the same anchor either way round: aligning to the top means the top of the
    /// box for a letterboxed image and the top of the *image* for a cropped one, which
    /// is the answer a photograph of a person wants in both cases.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// A multiplied tint, white leaving it unchanged — for bitmap icons, say.
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Draws the image at a fraction of its opacity, `1.0` being fully opaque.
    ///
    /// Unlike a group opacity this needs no layer: an image is one primitive, so the
    /// fade goes into the tint it is already multiplied by.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// The box this asks for, given the bitmap's own size.
    fn box_style(&self) -> Style {
        let natural = self.image.size();
        let ratio = if natural.height > 0.0 {
            natural.width / natural.height
        } else {
            1.0
        };
        match (self.width, self.height) {
            (Some(w), Some(h)) => Style {
                width: Dimension::Length(w),
                height: Dimension::Length(h),
                ..Default::default()
            },
            // One side and a ratio: the layout engine derives the other, which is the
            // reference's rule and the reason an image can be given a width alone.
            (Some(w), None) => Style {
                width: Dimension::Length(w),
                aspect_ratio: Some(ratio),
                ..Default::default()
            },
            (None, Some(h)) => Style {
                height: Dimension::Length(h),
                aspect_ratio: Some(ratio),
                ..Default::default()
            },
            (None, None) => Style {
                width: Dimension::Length(natural.width),
                height: Dimension::Length(natural.height),
                ..Default::default()
            },
        }
    }
}

impl<Msg> Widget<Msg> for Image {
    fn style(&self) -> Style {
        self.box_style()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        // Physical, not directional-aware here: `paint` is not told the reading
        // direction, and an image is a picture rather than a run of text — a portrait
        // does not want its crop mirrored because the interface is in Arabic.
        let align = self.alignment.resolve(frus_core::TextDirection::Ltr);
        let (dst, uv) = self.fit.apply_aligned(self.image.size(), bounds, align);
        let tint = self
            .tint
            .unwrap_or(Color::WHITE)
            .fade(status.opacity * self.opacity);
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

    /// The box an image asks for, down a column 400 wide.
    ///
    /// A `Column` and not a `Flex`: the reference's column centres its children, where
    /// flexbox stretches them, and a **stretched** cross axis is a width the image was
    /// handed — which beats a ratio, here as there. That is the parent's say, not a
    /// property of the image.
    fn asked_box(image: Image) -> Rect {
        let root = crate::Column::<()>::new().child(image);
        let runtime = crate::Runtime::default();
        let theme = Theme::default();
        let mut layout = frus_layout::Layout::new();
        let node = crate::ui::build_layout(
            &root,
            crate::interaction::WidgetId::ROOT,
            &runtime,
            &theme,
            &mut layout,
        );
        layout.compute_filled(node, 400.0, 400.0);
        // The column is first; the image is its only child.
        layout.absolute_rects(node)[1].0
    }

    /// No size given: the image's own, which is the whole point — showing a bitmap
    /// should not require already knowing how many pixels across it is.
    #[test]
    fn an_image_with_no_size_is_its_own_size() {
        let r = asked_box(Image::new(handle(64, 32)));
        assert_eq!((r.width, r.height), (64.0, 32.0));
    }

    /// One side given: the other follows the image's ratio, as in the reference — as
    /// long as the parent is not handing that side a size of its own.
    #[test]
    fn one_side_derives_the_other_from_the_ratio() {
        let by_width = asked_box(Image::new(handle(64, 32)).width(128.0));
        assert_eq!((by_width.width, by_width.height), (128.0, 64.0));

        let by_height = asked_box(Image::new(handle(64, 32)).height(8.0));
        assert_eq!((by_height.width, by_height.height), (16.0, 8.0));
    }

    /// Both given: that box, ratio or no ratio.
    #[test]
    fn both_sides_given_win() {
        let r = asked_box(Image::new(handle(64, 32)).size(10.0, 90.0));
        assert_eq!((r.width, r.height), (10.0, 90.0));
    }

    /// The alignment decides where a letterboxed image sits, and which part of a
    /// cropped one survives. Both from one anchor, which is the reference's answer too.
    #[test]
    fn the_alignment_places_the_letterbox_and_moves_the_crop() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        // A square in a wide box, contained: 40×40, and the anchor says where across.
        let left = paint(
            Image::new(handle(10, 10))
                .size(100.0, 40.0)
                .alignment(Alignment::CENTER_LEFT),
            bounds,
        );
        let right = paint(
            Image::new(handle(10, 10))
                .size(100.0, 40.0)
                .alignment(Alignment::CENTER_RIGHT),
            bounds,
        );
        let x_of = |p: &Primitive| match p {
            Primitive::Image { rect, .. } => rect.x,
            other => panic!("an image, not {other:?}"),
        };
        assert_eq!(x_of(&left), 0.0);
        assert_eq!(x_of(&right), 60.0, "hard against the right edge");

        // A tall image cropped to a wide box keeps the part the anchor names, which
        // travels the other way: aligning to the top keeps the top of the image.
        let uv_y = |p: &Primitive| match p {
            Primitive::Image { uv, .. } => uv.y,
            other => panic!("an image, not {other:?}"),
        };
        let top = paint(
            Image::new(handle(10, 100))
                .size(100.0, 40.0)
                .fit(BoxFit::Cover)
                .alignment(Alignment::TOP_CENTER),
            bounds,
        );
        let middle = paint(
            Image::new(handle(10, 100))
                .size(100.0, 40.0)
                .fit(BoxFit::Cover),
            bounds,
        );
        assert_eq!(uv_y(&top), 0.0, "the top of the image is kept");
        assert!(uv_y(&middle) > 0.0, "centred, the top is cropped away");
    }

    /// Opacity goes into the tint rather than into a layer: an image is one primitive.
    #[test]
    fn opacity_fades_the_tint() {
        let faded = paint(
            Image::new(handle(4, 4)).size(20.0, 20.0).opacity(0.5),
            Rect::new(0.0, 0.0, 20.0, 20.0),
        );
        match faded {
            Primitive::Image { tint, .. } => assert_eq!(tint.a, 0.5),
            other => panic!("an image, not {other:?}"),
        }
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
            Image::new(handle(10, 10)).size(100.0, 40.0),
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
            Image::new(handle(4, 4))
                .size(20.0, 20.0)
                .tint(Color::rgb(1.0, 0.0, 0.0)),
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
        let image = Image::new(handle(4, 4)).size(64.0, 48.0);
        let style = Widget::<()>::style(&image);
        assert_eq!(style.width, Dimension::Length(64.0));
        assert_eq!(style.height, Dimension::Length(48.0));
    }
}
