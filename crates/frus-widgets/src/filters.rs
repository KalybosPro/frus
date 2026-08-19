//! [`ColorFiltered`], [`ImageFiltered`] and [`ShaderMask`]: the three widgets that
//! apply a **pixel effect to their own subtree**.
//!
//! All three are pass-throughs in layout — the box is the child's — and all three
//! work the same way at paint time: the subtree is drained into a composited layer
//! carrying the effect, which the renderer applies when it composites. That is the
//! same machinery an opacity group and a shape clip already use, and the reason
//! these three cost one layer between them rather than one each: two of them wrapped
//! one inside the other are folded into a single layer, since a blur and a tint of a
//! blur are two independent slots, not two pictures.
//!
//! They share an `enabled` flag, and it means *no layer at all* rather than a layer
//! with a neutral effect. A blur of zero is still a blur — two full-surface passes
//! and a texture — so a caller animating one to nothing wants a way to say so, and
//! the flag is it.

use frus_core::{
    BlendMode, Color, ColorFilter, FractionalMask, ImageFilter, LayerFilter, Rect, Scene,
    ShaderMask as MaskEffect,
};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Applies a [`ColorFilter`] to every pixel of its child: greyscale, a tint, a
/// contrast curve, a silhouette.
///
/// Each pixel is transformed on its own, so nothing moves and nothing spreads — the
/// subtree keeps exactly the shape it had. Use [`ImageFiltered`] when the effect
/// needs a pixel's neighbours, and `BackdropFilter` when it should apply to what is
/// painted *underneath* rather than to the child.
///
/// ```ignore
/// ColorFiltered::new(ColorFilter::grayscale()).child(Image::asset("photo.png"))
/// ColorFiltered::new(ColorFilter::Mode(theme.primary, BlendMode::SrcIn))
///     .child(Icon::new(icons::STAR))   // the icon as a flat silhouette
/// ```
pub struct ColorFiltered<Msg> {
    filter: ColorFilter,
    enabled: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ColorFiltered<Msg> {
    /// Applies `filter` to the child.
    pub fn new(filter: ColorFilter) -> Self {
        Self {
            filter,
            enabled: true,
            children: Vec::new(),
        }
    }

    /// Drains the colour out of the child.
    pub fn grayscale() -> Self {
        Self::new(ColorFilter::grayscale())
    }

    /// Blends `color` into every pixel with `blend`.
    pub fn mode(color: Color, blend: BlendMode) -> Self {
        Self::new(ColorFilter::Mode(color, blend))
    }

    /// Sets the filtered child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// Turns the filter off: the child is painted as if this widget were not there,
    /// with no layer and no cost.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ColorFiltered<Msg> {
    fn style(&self) -> Style {
        // A pass-through: the box is the child's, and the effect is paint-time only.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn layer_filter(&self, _box_rect: Rect) -> Option<LayerFilter> {
        self.enabled.then_some(LayerFilter {
            color: Some(self.filter),
            ..LayerFilter::NONE
        })
    }
}

/// Applies an [`ImageFilter`] to its child: a blur, a dilate, an erode.
///
/// The result depends on a pixel's neighbours, so the child is treated as an image —
/// which is why the effect **spreads beyond the child's box**, and why a caller who
/// wants it contained wraps this in a clip.
///
/// This is the cheap way to blur one thing. `BackdropFilter` blurs everything painted
/// beneath it, which is a different and much more expensive question; when the answer
/// wanted is "blur this widget", it is this one.
///
/// ```ignore
/// ImageFiltered::blur(8.0).child(Image::asset("photo.png"))
/// ImageFiltered::new(ImageFilter::Dilate { radius_x: 2.0, radius_y: 2.0 }).child(logo)
/// ```
pub struct ImageFiltered<Msg> {
    filter: ImageFilter,
    enabled: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ImageFiltered<Msg> {
    /// Applies `filter` to the child.
    pub fn new(filter: ImageFilter) -> Self {
        Self {
            filter,
            enabled: true,
            children: Vec::new(),
        }
    }

    /// A Gaussian blur of `sigma` logical pixels, the same in both directions.
    pub fn blur(sigma: f32) -> Self {
        Self::new(ImageFilter::blur(sigma))
    }

    /// Sets the filtered child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// Turns the filter off: the child is painted as if this widget were not there.
    ///
    /// Prefer this over a filter that happens to be a no-op. A blur of zero still
    /// costs a texture and two full-surface passes; this costs nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ImageFiltered<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn layer_filter(&self, _box_rect: Rect) -> Option<LayerFilter> {
        self.enabled.then_some(LayerFilter {
            image: Some(self.filter),
            ..LayerFilter::NONE
        })
    }
}

/// Blends a **two-stop fade** over its child: a list that fades out at its bottom
/// edge, a headline filled with a gradient, a photograph dissolving into its page.
///
/// The fade is written in **fractions of the child's box** — `(0, 0)` its top-left
/// corner, `(1, 1)` its bottom-right — and follows it through every resize with
/// nothing to recompute. The default blend, [`BlendMode::Modulate`], multiplies, so
/// the fade's *alpha* becomes the child's transparency: that is what makes a mask a
/// mask, and it is why the colours in the examples below are white with only their
/// alpha varying. Any other blend mode turns it into a tint, a highlight, or a
/// silhouette instead.
///
/// ```ignore
/// ShaderMask::fade_out_bottom().child(list)
/// ShaderMask::linear((0.0, 0.0), (1.0, 0.0), theme.primary, theme.tertiary)
///     .blend(BlendMode::SrcIn)
///     .child(Text::new("gradient headline"))
/// ```
pub struct ShaderMask<Msg> {
    mask: FractionalMask,
    blend: BlendMode,
    enabled: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ShaderMask<Msg> {
    /// Blends `mask` over the child, multiplying it in.
    pub fn new(mask: FractionalMask) -> Self {
        Self {
            mask,
            blend: BlendMode::Modulate,
            enabled: true,
            children: Vec::new(),
        }
    }

    /// A straight fade between two points of the box, from `from_color` to
    /// `to_color`.
    pub fn linear(from: (f32, f32), to: (f32, f32), from_color: Color, to_color: Color) -> Self {
        Self::new(FractionalMask::Linear {
            from,
            to,
            from_color,
            to_color,
        })
    }

    /// A fade outwards from `center`, reaching `to_color` at `radius` — a fraction of
    /// the box's smaller side.
    pub fn radial(center: (f32, f32), radius: f32, from_color: Color, to_color: Color) -> Self {
        Self::new(FractionalMask::Radial {
            center,
            radius,
            from_color,
            to_color,
        })
    }

    /// The child, opaque at its top and gone at its bottom.
    pub fn fade_out_bottom() -> Self {
        Self::new(FractionalMask::fade_out_bottom())
    }

    /// How the fade meets the child. The default multiplies it in.
    pub fn blend(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }

    /// Sets the masked child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// Turns the mask off: the child is painted as if this widget were not there.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ShaderMask<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn layer_filter(&self, box_rect: Rect) -> Option<LayerFilter> {
        self.enabled.then(|| LayerFilter {
            mask: Some(MaskEffect {
                // The fractions become pixels here, where the box is finally known.
                shader: LayerFilter::resolve_mask(self.mask, box_rect),
                blend: self.blend,
            }),
            ..LayerFilter::NONE
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex, Theme};
    use frus_core::{MaskShader, Primitive, Size};

    fn scene_of(root: &impl Widget<()>) -> Vec<Primitive> {
        let rt = crate::runtime::Runtime::default();
        let theme = Theme::dark();
        let ui = crate::ui::build_ui(root, Size::new(200.0, 200.0), &rt, &theme);
        ui.scene().primitives().to_vec()
    }

    fn only_layer(primitives: &[Primitive]) -> LayerFilter {
        let layers: Vec<&Primitive> = primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Layer { .. }))
            .collect();
        assert_eq!(layers.len(), 1, "exactly one layer: {primitives:?}");
        match layers[0] {
            Primitive::Layer { filter, .. } => *filter,
            _ => unreachable!(),
        }
    }

    fn box_of(color: Color) -> Container<()> {
        Container::new().width(100.0).height(50.0).color(color)
    }

    #[test]
    fn a_colour_filter_reaches_the_layer() {
        let root = ColorFiltered::grayscale().child(box_of(Color::rgb(1.0, 0.0, 0.0)));
        let filter = only_layer(&scene_of(&root));
        assert!(matches!(filter.color, Some(ColorFilter::Matrix(_))));
        assert!(filter.image.is_none() && filter.mask.is_none());
    }

    /// The disabled flag means **no layer**, not a layer with nothing in it: a blur
    /// of zero still costs a texture and two passes, and the point of the flag is to
    /// cost nothing.
    #[test]
    fn disabled_paints_no_layer_at_all() {
        let root = ImageFiltered::blur(8.0)
            .enabled(false)
            .child(box_of(Color::rgb(1.0, 0.0, 0.0)));
        let primitives = scene_of(&root);
        assert!(
            !primitives
                .iter()
                .any(|p| matches!(p, Primitive::Layer { .. })),
            "no layer: {primitives:?}"
        );
    }

    /// Two filters one inside the other are **one** layer, not two — because
    /// compositing does not re-composite a layer nested in another, and because a
    /// blur and a tint are independent slots rather than two pictures.
    #[test]
    fn two_filters_share_one_layer() {
        let root = ColorFiltered::grayscale()
            .child(ImageFiltered::blur(4.0).child(box_of(Color::rgb(1.0, 0.0, 0.0))));
        let filter = only_layer(&scene_of(&root));
        assert!(filter.color.is_some(), "the outer greyscale survived");
        assert!(filter.image.is_some(), "the inner blur survived");
    }

    /// Two filters asking for the **same** slot cannot share one, so they nest. The
    /// outer one is a filter of the inner one's result, and folding them would
    /// silently drop one.
    #[test]
    fn two_filters_of_the_same_kind_do_not_fold() {
        let root = ColorFiltered::new(ColorFilter::invert())
            .child(ColorFiltered::grayscale().child(box_of(Color::rgb(1.0, 0.0, 0.0))));
        let primitives = scene_of(&root);
        let outer = primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Layer { .. }))
            .count();
        assert_eq!(outer, 1, "the outer layer");
        match &primitives
            .iter()
            .find(|p| matches!(p, Primitive::Layer { .. }))
        {
            Some(Primitive::Layer { primitives, .. }) => assert!(
                primitives
                    .iter()
                    .any(|p| matches!(p, Primitive::Layer { .. })),
                "the inner filter kept a layer of its own"
            ),
            _ => panic!("a layer"),
        }
    }

    /// The mask is written in fractions and resolved against the box the widget
    /// actually got — which is the whole reason the resolution happens in the walk
    /// and not at the call site.
    #[test]
    fn a_mask_is_resolved_against_the_box_on_screen() {
        // The masked box is the second row of a column of two 100x50 boxes, so it
        // starts at y = 50 and ends at y = 100.
        let root = Flex::<()>::column()
            .child(box_of(Color::rgb(0.0, 1.0, 0.0)))
            .child(ShaderMask::fade_out_bottom().child(box_of(Color::rgb(1.0, 0.0, 0.0))));
        let filter = only_layer(&scene_of(&root));
        let mask = filter.mask.expect("a mask").shader;
        match mask {
            MaskShader::Linear { from, to, .. } => {
                assert!(
                    (from.y - 50.0).abs() < 0.5,
                    "starts at the box top: {from:?}"
                );
                assert!((to.y - 100.0).abs() < 0.5, "ends at its bottom: {to:?}");
            }
            other => panic!("a linear fade: {other:?}"),
        }
    }

    /// The default blend multiplies, which is what turns a fade into a mask rather
    /// than into a tint painted over the child.
    #[test]
    fn a_mask_multiplies_unless_told_otherwise() {
        let plain = ShaderMask::fade_out_bottom().child(box_of(Color::WHITE));
        assert_eq!(
            only_layer(&scene_of(&plain)).mask.expect("a mask").blend,
            BlendMode::Modulate
        );
        let tinted = ShaderMask::fade_out_bottom()
            .blend(BlendMode::SrcIn)
            .child(box_of(Color::WHITE));
        assert_eq!(
            only_layer(&scene_of(&tinted)).mask.expect("a mask").blend,
            BlendMode::SrcIn
        );
    }

    /// A filter widget changes nothing about layout: its box is its child's, and the
    /// sibling after it starts exactly where it would have without the filter.
    #[test]
    fn a_filter_does_not_move_anything() {
        let bare = Flex::<()>::column()
            .child(box_of(Color::rgb(1.0, 0.0, 0.0)))
            .child(box_of(Color::rgb(0.0, 0.0, 1.0)));
        let filtered = Flex::<()>::column()
            .child(ImageFiltered::blur(6.0).child(box_of(Color::rgb(1.0, 0.0, 0.0))))
            .child(box_of(Color::rgb(0.0, 0.0, 1.0)));
        let blue_y = |primitives: &[Primitive]| {
            primitives
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { rect, color, .. } if color.b > 0.5 && color.r < 0.5 => {
                        Some(rect.y)
                    }
                    _ => None,
                })
                .expect("the blue sibling")
        };
        assert_eq!(blue_y(&scene_of(&bare)), blue_y(&scene_of(&filtered)));
    }
}
