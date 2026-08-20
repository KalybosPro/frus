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
    /// What a screen reader is told this picture is; see [`Image::semantic_label`].
    semantic_label: Option<String>,
    /// Whether to leave it out of the tree a screen reader walks.
    exclude_from_semantics: bool,
    /// Whether the picture is **mirrored** in a right-to-left reading direction.
    match_text_direction: bool,
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
            semantic_label: None,
            exclude_from_semantics: false,
            match_text_direction: false,
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

    /// What a screen reader says instead of the picture.
    ///
    /// A picture with no label is a picture nobody reading by ear can see, and until
    /// this existed **every** image in every application was one: the vocabulary had
    /// [`frus_core::Role::Image`] in it and nothing ever emitted it.
    ///
    /// Say what the picture *is*, not that it is a picture — a reader is already told
    /// the role. If it carries no meaning of its own, do not label it: reach for
    /// [`exclude_from_semantics`](Image::exclude_from_semantics) instead.
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }

    /// Leaves the image out of the tree a screen reader walks.
    ///
    /// The right answer for **decoration** — a divider, a texture, a shape behind a
    /// heading. Announcing those interrupts a reader with something that was never
    /// meant to be read, and an empty label would still announce the role. This is the
    /// reference's `excludeFromSemantics`, and like it, it wins over any label given.
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    /// Mirrors the image horizontally when the reading direction is right-to-left.
    ///
    /// **Off** by default, which is the reference's default and the right one: an image
    /// is a picture rather than a run of text, and a photograph of a person does not
    /// want to be flipped because the interface is in Arabic. It is a per-image
    /// decision, so it is a per-image switch.
    ///
    /// Turn it on for a picture that **points**: an arrow meaning *forward*, a
    /// speech bubble with a tail, a hand indicating the next step. Those follow the
    /// direction the reader's eye travels, and in RTL that is the other way round.
    pub fn match_text_direction(mut self, match_direction: bool) -> Self {
        self.match_text_direction = match_direction;
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

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // The **alignment** stays physical whatever the direction. An image is a
        // picture rather than a run of text: a portrait aligned to the top of its
        // crop wants the top of its crop in every language. Mirroring is the separate,
        // opt-in question below, which is how the reference splits it too.
        let align = self.alignment.resolve(frus_core::TextDirection::Ltr);
        let (dst, mut uv) = self.fit.apply_aligned(self.image.size(), bounds, align);
        // A mirror costs nothing but a sign. The shader reads
        // `uv.xy + unit_pos * uv.zw` with `unit_pos` running 0..1, so a **negative**
        // width walks the same span backwards — which is the reference's "scaling
        // factor of -1 in the horizontal direction", without a transform, a layer, or
        // a second copy of the pixels.
        if self.match_text_direction && theme.direction == frus_core::TextDirection::Rtl {
            uv = Rect::new(uv.x + uv.width, uv.y, -uv.width, uv.height);
        }
        let tint = self
            .tint
            .unwrap_or(Color::WHITE)
            .fade(status.opacity * self.opacity);
        scene.draw_image(&self.image, dst, uv, tint);
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        if self.exclude_from_semantics {
            return None;
        }
        let mut semantics = frus_core::Semantics::new(frus_core::Role::Image);
        if let Some(label) = self.semantic_label.as_deref() {
            semantics = semantics.label(label);
        }
        Some(semantics)
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
        paint_in(image, bounds, &Theme::default())
    }

    /// The same, under a theme of the caller's choosing — which is how the reading
    /// direction reaches `paint`.
    fn paint_in(image: Image, bounds: Rect, theme: &Theme) -> Primitive {
        let mut scene = Scene::new();
        Widget::<()>::paint(&image, bounds, Status::default(), theme, &mut scene);
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

    /// The hole this milestone came for. `Role::Image` was in the vocabulary and mapped
    /// to the platform's, and **nothing in the framework ever emitted it** — so every
    /// picture in every application was silent to a screen reader.
    #[test]
    fn a_labelled_image_is_announced() {
        let semantics = Widget::<()>::semantics(&Image::new(handle(4, 4)).semantic_label("Ada"))
            .expect("an image is announced");
        assert_eq!(semantics.role, frus_core::Role::Image);
        assert_eq!(semantics.label.as_deref(), Some("Ada"));
    }

    /// Unlabelled, it still says *there is a picture here*. A reader who meets it knows
    /// something is there and can move past it; leaving it out entirely would be the
    /// application's decision, not the widget's default.
    #[test]
    fn an_unlabelled_image_still_announces_the_role() {
        let semantics =
            Widget::<()>::semantics(&Image::new(handle(4, 4))).expect("still announced");
        assert_eq!(semantics.role, frus_core::Role::Image);
        assert_eq!(semantics.label, None);
    }

    /// Decoration is excluded outright, and the exclusion wins over a label — the
    /// reference's rule, and the only one that is not ambiguous when both are given.
    #[test]
    fn decoration_is_left_out_of_the_tree_a_reader_walks() {
        let decoration = Image::new(handle(4, 4))
            .semantic_label("a texture")
            .exclude_from_semantics(true);
        assert!(Widget::<()>::semantics(&decoration).is_none());
    }

    /// The sub-region an image samples, as painted.
    fn sampled(image: Image, theme: &Theme) -> Rect {
        match paint_in(image, Rect::new(0.0, 0.0, 40.0, 40.0), theme) {
            Primitive::Image { uv, .. } => uv,
            _ => panic!("expected an image"),
        }
    }

    /// A picture that **points** is mirrored in a right-to-left reading direction, which
    /// the reference describes as a scaling factor of -1 horizontally. Here it is a sign
    /// on the sampled width: the shader reads `uv.xy + unit_pos * uv.zw`, so a negative
    /// width walks the same span backwards — no transform, no layer, no second copy of
    /// the pixels.
    #[test]
    fn a_directional_image_is_mirrored_in_rtl() {
        let forward = sampled(Image::new(handle(8, 4)), &Theme::default());
        let mirrored = sampled(
            Image::new(handle(8, 4)).match_text_direction(true),
            &Theme::default().rtl(),
        );
        assert_eq!(
            mirrored.width, -forward.width,
            "the span is walked backwards"
        );
        assert_eq!(
            mirrored.x,
            forward.x + forward.width,
            "and it starts at the far edge"
        );
        // The vertical span is untouched: this is a mirror, not a rotation.
        assert_eq!((mirrored.y, mirrored.height), (forward.y, forward.height));
    }

    /// It is **opt-in**, and off it stays off in both directions. A photograph of a
    /// person does not want to be flipped because the interface is in Arabic, which is
    /// why the reference makes this a per-image switch rather than a global rule.
    #[test]
    fn an_ordinary_image_is_not_mirrored_by_the_reading_direction() {
        let plain = Image::new(handle(8, 4));
        assert_eq!(
            sampled(plain, &Theme::default().rtl()),
            sampled(Image::new(handle(8, 4)), &Theme::default()),
        );
    }

    /// And the switch alone does nothing: it is the direction that mirrors, not the flag.
    #[test]
    fn a_directional_image_is_left_alone_in_ltr() {
        assert_eq!(
            sampled(
                Image::new(handle(8, 4)).match_text_direction(true),
                &Theme::default()
            ),
            sampled(Image::new(handle(8, 4)), &Theme::default()),
        );
    }

    /// And it reaches the tree, not just the hook.
    ///
    /// A trait method nobody calls is the shape of bug this project has already been
    /// bitten by — a hook that answers correctly while the walk never asks it, green
    /// unit tests over a feature that does nothing. So this drives `build_ui` and reads
    /// what the walk actually collected.
    #[test]
    fn the_walk_collects_the_image_and_skips_the_decoration() {
        let tree = crate::Flex::<()>::column()
            .child(Image::new(handle(8, 8)).semantic_label("Ada Lovelace"))
            .child(Image::new(handle(8, 8)).exclude_from_semantics(true));
        let ui = crate::build_ui(
            &tree,
            crate::Size::new(200.0, 200.0),
            &crate::Runtime::default(),
            &Theme::default(),
        );
        let images: Vec<_> = ui
            .semantics()
            .iter()
            .filter(|(_, _, s)| s.role == frus_core::Role::Image)
            .map(|(_, _, s)| s.label.clone())
            .collect();
        assert_eq!(
            images,
            vec![Some("Ada Lovelace".to_string())],
            "the labelled one is announced and the decoration is not"
        );
    }
}
