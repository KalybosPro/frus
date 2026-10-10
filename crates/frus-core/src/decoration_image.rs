//! **A picture as a box's background** (milestone 645): the reference's `DecorationImage`
//! and the `paintImage` that draws it (`decoration_image.dart`).
//!
//! A [`BoxDecoration`](crate::BoxDecoration) paints it between its background and its
//! border, clipped to its corners or its circle. Everything about where the picture
//! goes is here: how it is fitted, which part of it shows, where the spare room goes,
//! whether it repeats, which part of it stretches, and whether it is mirrored.

use crate::{
    AlignmentGeometry, BoxFit, Color, ColorFilter, ImageHandle, LayerFilter, Primitive, Rect,
    Scene, Size, TextDirection,
};

/// How a picture fills the room its fit leaves: once, or repeated across, down, or both
/// — the reference's `ImageRepeat`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageRepeat {
    /// Across and down, until the box is covered.
    Repeat,
    /// Across only.
    RepeatX,
    /// Down only.
    RepeatY,
    /// Once; the rest of the box stays empty.
    #[default]
    NoRepeat,
}

/// The two sizes a fit settles on — the reference's `FittedSizes`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FittedSizes {
    /// How much of the picture shows, in the picture's own units.
    pub source: Size,
    /// How big it is drawn.
    pub destination: Size,
}

/// Fits `input` into `output` by `fit` — the reference's `applyBoxFit`.
///
/// Unlike [`BoxFit::apply`], nothing is ever drawn bigger than `output`: a fit that would
/// overflow shows **less of the picture** instead. [`BoxFit::None`] shows as much of it
/// as fits at its own size, and [`BoxFit::FitWidth`] on a picture taller than its box
/// shows a band of it. An empty size on either side fits nothing.
pub fn apply_box_fit(fit: BoxFit, input: Size, output: Size) -> FittedSizes {
    let zero = Size::new(0.0, 0.0);
    if input.width <= 0.0 || input.height <= 0.0 || output.width <= 0.0 || output.height <= 0.0 {
        return FittedSizes {
            source: zero,
            destination: zero,
        };
    }
    // Whether the box is wider, for its height, than the picture.
    let wider = output.width / output.height > input.width / input.height;
    // The whole picture, as large as fits.
    let contain = || {
        if wider {
            Size::new(input.width * output.height / input.height, output.height)
        } else {
            Size::new(output.width, input.height * output.width / input.width)
        }
    };
    // The part of the picture the box's shape cuts out of it.
    let cover = || {
        if wider {
            Size::new(input.width, input.width * output.height / output.width)
        } else {
            Size::new(input.height * output.width / output.height, input.height)
        }
    };
    let (source, destination) = match fit {
        BoxFit::Fill => (input, output),
        BoxFit::Contain => (input, contain()),
        BoxFit::Cover => (cover(), output),
        BoxFit::FitWidth if wider => (cover(), output),
        BoxFit::FitWidth => (input, contain()),
        BoxFit::FitHeight if wider => (input, contain()),
        BoxFit::FitHeight => (cover(), output),
        BoxFit::None => {
            let shown = Size::new(
                input.width.min(output.width),
                input.height.min(output.height),
            );
            (shown, shown)
        }
        BoxFit::ScaleDown => {
            let aspect = input.width / input.height;
            let mut size = input;
            if size.height > output.height {
                size = Size::new(output.height * aspect, output.height);
            }
            if size.width > output.width {
                size = Size::new(output.width, output.width / aspect);
            }
            (input, size)
        }
    };
    FittedSizes {
        source,
        destination,
    }
}

/// The most tiles one repeated picture draws. A one-pixel pattern repeated over a large
/// box would otherwise be millions of draws; past this, the rest of the box stays empty.
pub const MAX_IMAGE_TILES: usize = 10_000;

/// One part of a picture drawn somewhere: where, and which part, in `0..1` of the
/// picture (a negative width walks it backwards, which is how a picture is mirrored).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagePiece {
    /// Where it lands.
    pub dst: Rect,
    /// The part of the picture sampled.
    pub uv: Rect,
    /// Whether sampling stays strictly inside `uv`: the parts of a sliced picture,
    /// stretched apart from each other.
    pub strict: bool,
}

/// A picture painted as a box's background — the reference's `DecorationImage`.
///
/// The defaults are the reference's: centred, not repeated, no slice, at its own scale
/// and opacity, fitted by [`BoxFit::ScaleDown`] (or [`BoxFit::Fill`] with a centre slice)
/// when no fit is named.
///
/// The picture is pixels already decoded. One that is still on its way — a network
/// picture, say — is `None` for now, and paints nothing until it arrives, as the
/// reference's does.
#[derive(Clone, Debug, PartialEq)]
pub struct DecorationImage {
    /// The pixels; `None` paints nothing.
    pub image: Option<ImageHandle>,
    /// A colour filter over the picture, applied before its opacity is.
    pub color_filter: Option<ColorFilter>,
    /// How the picture is fitted into the box; `None` is the reference's default.
    pub fit: Option<BoxFit>,
    /// Where the picture sits in the spare room, and which part of it a crop keeps.
    pub alignment: AlignmentGeometry,
    /// The **centre slice**, in the picture's units (its pixels over `scale`): the part
    /// that stretches, while the four corners stay their size and the four edges
    /// stretch along their length. Fits that crop ([`BoxFit::None`],
    /// [`BoxFit::Cover`]) have nothing to stretch and are not meant for it.
    pub center_slice: Option<Rect>,
    /// Whether the picture repeats to fill the box.
    pub repeat: ImageRepeat,
    /// Whether the picture is mirrored when the box reads right to left.
    pub match_text_direction: bool,
    /// The picture's pixels per logical pixel: `2.0` draws it at half its pixel size.
    pub scale: f32,
    /// How opaque the picture is, `0..=1`.
    pub opacity: f32,
    /// Whether the picture's colours are inverted, after the colour filter.
    pub invert_colors: bool,
    /// Two pictures mid-transition, painted one over the other.
    crossfade: Option<Box<Crossfade>>,
}

/// Two pictures `t` of the way from one to the other.
#[derive(Clone, Debug, PartialEq)]
struct Crossfade {
    from: Option<DecorationImage>,
    to: Option<DecorationImage>,
    t: f32,
}

impl DecorationImage {
    /// The picture `image`, centred, at its own size or smaller.
    pub fn new(image: impl Into<Option<ImageHandle>>) -> Self {
        Self {
            image: image.into(),
            color_filter: None,
            fit: None,
            alignment: crate::Alignment::CENTER.into(),
            center_slice: None,
            repeat: ImageRepeat::NoRepeat,
            match_text_direction: false,
            scale: 1.0,
            opacity: 1.0,
            invert_colors: false,
            crossfade: None,
        }
    }

    /// Sets the colour filter.
    pub fn color_filter(mut self, filter: ColorFilter) -> Self {
        self.color_filter = Some(filter);
        self
    }

    /// Sets the fit.
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = Some(fit);
        self
    }

    /// Sets where the picture sits.
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// Sets the centre slice, the part that stretches.
    pub fn center_slice(mut self, slice: Rect) -> Self {
        self.center_slice = Some(slice);
        self
    }

    /// Sets how the picture repeats.
    pub fn repeat(mut self, repeat: ImageRepeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Mirrors the picture when the box reads right to left.
    pub fn match_text_direction(mut self, matches: bool) -> Self {
        self.match_text_direction = matches;
        self
    }

    /// Sets the picture's pixels per logical pixel.
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Sets the opacity.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Inverts the picture's colours.
    pub fn invert_colors(mut self, invert: bool) -> Self {
        self.invert_colors = invert;
        self
    }

    /// The picture `t` of the way from `a` to `b` — the reference's
    /// `DecorationImage.lerp`. Two different pictures are not blended into a third: both
    /// are painted, the second over the first, fading in. A picture on one side only
    /// fades in or out. At `0` and `1` the answer is the end itself.
    pub fn lerp(
        a: Option<&DecorationImage>,
        b: Option<&DecorationImage>,
        t: f32,
    ) -> Option<DecorationImage> {
        if t.is_nan() || t <= 0.0 || a == b {
            return a.cloned();
        }
        if t >= 1.0 {
            return b.cloned();
        }
        // What the transition reports as its own settings: the arriving picture's, or
        // the leaving one's when nothing arrives — as the reference's blend does.
        let shown = b.or(a)?.clone();
        Some(DecorationImage {
            crossfade: Some(Box::new(Crossfade {
                from: a.cloned(),
                to: b.cloned(),
                t,
            })),
            ..shown
        })
    }

    /// Where each part of the picture lands when it is painted into `rect`, for a box
    /// reading in `direction`, given the picture's pixel `size` — the reference's
    /// `paintImage`, short of the drawing. Also says whether the parts repeat, which is
    /// when they are clipped to `rect`.
    ///
    /// Empty when there is nothing to draw: an empty box, an empty picture, or a fit
    /// that leaves no room.
    pub fn pieces(
        &self,
        size: Size,
        rect: Rect,
        direction: TextDirection,
    ) -> (Vec<ImagePiece>, bool) {
        let none = (Vec::new(), false);
        if rect.width <= 0.0 || rect.height <= 0.0 || size.width <= 0.0 || size.height <= 0.0 {
            return none;
        }
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        let mut output = Size::new(rect.width, rect.height);
        let mut input = size;
        // The slice's border: what lies outside the centre, drawn at its own size.
        let border = self.center_slice.map(|slice| {
            Size::new(
                size.width / scale - slice.width,
                size.height / scale - slice.height,
            )
        });
        if let Some(b) = border {
            output = Size::new(output.width - b.width, output.height - b.height);
            input = Size::new(
                input.width - b.width * scale,
                input.height - b.height * scale,
            );
        }
        let fit = self.fit.unwrap_or(if self.center_slice.is_none() {
            BoxFit::ScaleDown
        } else {
            BoxFit::Fill
        });
        let fitted = apply_box_fit(
            fit,
            Size::new(input.width / scale, input.height / scale),
            output,
        );
        let source = Size::new(fitted.source.width * scale, fitted.source.height * scale);
        let mut destination = fitted.destination;
        if let Some(b) = border {
            output = Size::new(output.width + b.width, output.height + b.height);
            destination = Size::new(destination.width + b.width, destination.height + b.height);
        }
        if destination.width <= 0.0 || destination.height <= 0.0 {
            return none;
        }
        // Filling the box exactly, there is nothing to repeat.
        let repeat = if destination == output {
            ImageRepeat::NoRepeat
        } else {
            self.repeat
        };
        let flip = self.match_text_direction && direction.is_rtl();
        let alignment = self.alignment.resolve(direction);
        // The spare room, and where the alignment puts the picture in it. A mirrored
        // picture is placed by the mirrored alignment, and the whole lot mirrored back
        // below — so the picture lands where the alignment says, reversed.
        let half_w = (output.width - destination.width) / 2.0;
        let half_h = (output.height - destination.height) / 2.0;
        let ax = if flip { -alignment.x } else { alignment.x };
        let placed = Rect::new(
            rect.x + half_w + ax * half_w,
            rect.y + half_h + alignment.y * half_h,
            destination.width,
            destination.height,
        );
        let tiles = if repeat == ImageRepeat::NoRepeat {
            vec![placed]
        } else {
            tile_rects(rect, placed, repeat)
        };
        let mut pieces = Vec::new();
        match self.center_slice {
            None => {
                // The part of the picture a crop keeps, placed by the alignment.
                let x = (size.width - source.width) / 2.0 * (1.0 + alignment.x);
                let y = (size.height - source.height) / 2.0 * (1.0 + alignment.y);
                let uv = Rect::new(
                    x / size.width,
                    y / size.height,
                    source.width / size.width,
                    source.height / size.height,
                );
                pieces.extend(tiles.into_iter().map(|dst| ImagePiece {
                    dst,
                    uv,
                    strict: false,
                }));
            }
            Some(slice) => {
                for tile in tiles {
                    nine(&mut pieces, size, slice, scale, tile);
                }
            }
        }
        if flip {
            // Mirrored about the box's centre: each part lands on the other side, read
            // backwards.
            let across = 2.0 * rect.x + rect.width;
            for piece in &mut pieces {
                let d = piece.dst;
                piece.dst = Rect::new(across - d.x - d.width, d.y, d.width, d.height);
                let uv = piece.uv;
                piece.uv = Rect::new(uv.x + uv.width, uv.y, -uv.width, uv.height);
            }
        }
        (pieces, repeat != ImageRepeat::NoRepeat)
    }

    /// Paints the picture into `rect`, for a box reading in `direction`, at `blend` of
    /// its opacity — the reference's `DecorationImagePainter.paint`, short of the clip,
    /// which the decoration adds.
    pub fn paint(&self, scene: &mut Scene, rect: Rect, direction: TextDirection, blend: f32) {
        if let Some(crossfade) = &self.crossfade {
            crossfade.paint(scene, rect, direction, blend);
            return;
        }
        let Some(image) = &self.image else {
            return;
        };
        let alpha = (self.opacity * blend).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let (pieces, repeats) = self.pieces(image.size(), rect, direction);
        if pieces.is_empty() {
            return;
        }
        let start = scene.primitives().len();
        let outer = scene.current_clip();
        if repeats {
            scene.set_clip(outer.intersect(rect));
        }
        let tint = Color::WHITE.fade(alpha);
        for piece in pieces {
            if piece.strict {
                scene.draw_image_strict(image, piece.dst, piece.uv, tint);
            } else {
                scene.draw_image(image, piece.dst, piece.uv, tint);
            }
        }
        scene.set_clip(outer);
        // The colour filter first, then the inversion over its result, as the
        // reference's paint applies them.
        if let Some(filter) = self.color_filter {
            group(scene, start, 1.0, Some(filter));
        }
        if self.invert_colors {
            group(scene, start, 1.0, Some(ColorFilter::invert()));
        }
    }
}

impl Crossfade {
    /// Both pictures, the arriving one over the leaving one; one alone, faded.
    fn paint(&self, scene: &mut Scene, rect: Rect, direction: TextDirection, blend: f32) {
        match (&self.from, &self.to) {
            (Some(from), Some(to)) => {
                // The leaving picture whole, the arriving one over it at `t`: where the
                // arriving one is opaque, that is the reference's sum of the two at
                // `1 - t` and `t`. As a group, so the blend fades the pair as one.
                let start = scene.primitives().len();
                from.paint(scene, rect, direction, 1.0);
                to.paint(scene, rect, direction, self.t);
                if blend < 1.0 {
                    group(scene, start, blend, None);
                }
            }
            (Some(from), None) => from.paint(scene, rect, direction, blend * (1.0 - self.t)),
            (None, Some(to)) => to.paint(scene, rect, direction, blend * self.t),
            (None, None) => {}
        }
    }
}

/// Wraps what was painted since `start` into one layer, at `opacity`, through `filter`.
fn group(scene: &mut Scene, start: usize, opacity: f32, filter: Option<ColorFilter>) {
    if scene.primitives().len() == start {
        return;
    }
    let primitives = scene.split_off(start);
    let layer = Primitive::Layer {
        primitives,
        opacity,
        clip: scene.current_clip(),
        clip_shape: crate::ClipShape::Rect,
        transform: None,
        filter: LayerFilter {
            color: filter,
            ..LayerFilter::NONE
        },
        owner: scene.current_owner(),
    };
    scene.push_primitive(layer);
}

/// The copies of `placed` that cover `rect`, along the axes `repeat` names — the
/// reference's `_generateImageTileRects`, stopped at [`MAX_IMAGE_TILES`].
fn tile_rects(rect: Rect, placed: Rect, repeat: ImageRepeat) -> Vec<Rect> {
    let (stride_x, stride_y) = (placed.width, placed.height);
    let across = matches!(repeat, ImageRepeat::Repeat | ImageRepeat::RepeatX);
    let down = matches!(repeat, ImageRepeat::Repeat | ImageRepeat::RepeatY);
    let (mut start_x, mut stop_x, mut start_y, mut stop_y) = (0i64, 0i64, 0i64, 0i64);
    if across {
        start_x = ((rect.x - placed.x) / stride_x).floor() as i64;
        stop_x = ((rect.x + rect.width - placed.x - placed.width) / stride_x).ceil() as i64;
    }
    if down {
        start_y = ((rect.y - placed.y) / stride_y).floor() as i64;
        stop_y = ((rect.y + rect.height - placed.y - placed.height) / stride_y).ceil() as i64;
    }
    let mut tiles = Vec::new();
    'columns: for i in start_x..=stop_x {
        for j in start_y..=stop_y {
            if tiles.len() == MAX_IMAGE_TILES {
                break 'columns;
            }
            tiles.push(Rect::new(
                placed.x + i as f32 * stride_x,
                placed.y + j as f32 * stride_y,
                placed.width,
                placed.height,
            ));
        }
    }
    tiles
}

/// The nine parts of a picture of pixel `size` stretched into `dst` around its centre
/// `slice` (in the picture's units, its pixels over `scale`): the corners at their own
/// size, the edges stretched along their length, the centre both ways. `dst` is never
/// smaller than the corners: [`DecorationImage::pieces`] grows it past a box too small
/// for them, as the reference does.
fn nine(pieces: &mut Vec<ImagePiece>, size: Size, slice: Rect, scale: f32, dst: Rect) {
    // The picture's columns and rows, in pixels, the slice clamped to it.
    let sx = [
        0.0,
        (slice.x * scale).clamp(0.0, size.width),
        ((slice.x + slice.width) * scale).clamp(0.0, size.width),
        size.width,
    ];
    let sy = [
        0.0,
        (slice.y * scale).clamp(0.0, size.height),
        ((slice.y + slice.height) * scale).clamp(0.0, size.height),
        size.height,
    ];
    // The fixed parts, in logical pixels.
    let (left, right) = (sx[1] / scale, (sx[3] - sx[2]) / scale);
    let (top, bottom) = (sy[1] / scale, (sy[3] - sy[2]) / scale);
    let dx = [
        dst.x,
        dst.x + left,
        dst.x + dst.width - right,
        dst.x + dst.width,
    ];
    let dy = [
        dst.y,
        dst.y + top,
        dst.y + dst.height - bottom,
        dst.y + dst.height,
    ];
    for j in 0..3 {
        for i in 0..3 {
            let (w, h) = (dx[i + 1] - dx[i], dy[j + 1] - dy[j]);
            let (sw, sh) = (sx[i + 1] - sx[i], sy[j + 1] - sy[j]);
            if w <= 0.0 || h <= 0.0 || sw <= 0.0 || sh <= 0.0 {
                continue;
            }
            pieces.push(ImagePiece {
                dst: Rect::new(dx[i], dy[j], w, h),
                uv: Rect::new(
                    sx[i] / size.width,
                    sy[j] / size.height,
                    sw / size.width,
                    sh / size.height,
                ),
                strict: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Alignment, AlignmentDirectional, ImageData};

    fn handle(width: u32, height: u32) -> ImageHandle {
        ImageData::from_rgba(width, height, vec![255; (width * height * 4) as usize]).into_handle()
    }

    fn size(width: f32, height: f32) -> Size {
        Size::new(width, height)
    }

    fn fitted(fit: BoxFit, input: Size, output: Size) -> (Size, Size) {
        let f = apply_box_fit(fit, input, output);
        (f.source, f.destination)
    }

    fn pieces(image: &DecorationImage, px: Size, rect: Rect) -> Vec<ImagePiece> {
        image.pieces(px, rect, TextDirection::Ltr).0
    }

    /// The reference's table, for a picture twice as wide as it is tall in a square.
    #[test]
    fn a_fit_settles_the_reference_sizes() {
        let (input, square) = (size(200.0, 100.0), size(100.0, 100.0));
        assert_eq!(fitted(BoxFit::Fill, input, square), (input, square));
        assert_eq!(
            fitted(BoxFit::Contain, input, square),
            (input, size(100.0, 50.0))
        );
        assert_eq!(
            fitted(BoxFit::Cover, input, square),
            (size(100.0, 100.0), square)
        );
        assert_eq!(
            fitted(BoxFit::FitWidth, input, square),
            (input, size(100.0, 50.0))
        );
        assert_eq!(
            fitted(BoxFit::FitHeight, input, square),
            (size(100.0, 100.0), square)
        );
        assert_eq!(fitted(BoxFit::None, input, square), (square, square));
        assert_eq!(
            fitted(BoxFit::ScaleDown, input, square),
            (input, size(100.0, 50.0))
        );
        // In a box wider than the picture, the width fit crops and the height fit
        // letterboxes.
        let (picture, band) = (size(100.0, 100.0), size(200.0, 50.0));
        assert_eq!(
            fitted(BoxFit::FitWidth, picture, band),
            (size(100.0, 25.0), band)
        );
        assert_eq!(
            fitted(BoxFit::FitHeight, picture, band),
            (picture, size(50.0, 50.0))
        );
        assert_eq!(
            fitted(BoxFit::Contain, picture, band),
            (picture, size(50.0, 50.0))
        );
        assert_eq!(
            fitted(BoxFit::Cover, picture, band),
            (size(100.0, 25.0), band)
        );
        // Scaled down to the box's height, then its width.
        assert_eq!(
            fitted(BoxFit::ScaleDown, size(100.0, 400.0), size(50.0, 100.0)),
            (size(100.0, 400.0), size(25.0, 100.0))
        );
        assert_eq!(
            fitted(BoxFit::ScaleDown, size(400.0, 100.0), size(100.0, 100.0)),
            (size(400.0, 100.0), size(100.0, 25.0))
        );
        // Never up.
        assert_eq!(
            fitted(BoxFit::ScaleDown, size(10.0, 10.0), square),
            (size(10.0, 10.0), size(10.0, 10.0))
        );
    }

    #[test]
    fn an_empty_size_fits_nothing() {
        let zero = size(0.0, 0.0);
        let ten = size(10.0, 10.0);
        assert_eq!(fitted(BoxFit::Fill, zero, ten), (zero, zero));
        assert_eq!(fitted(BoxFit::Fill, size(10.0, 0.0), ten), (zero, zero));
        assert_eq!(fitted(BoxFit::Fill, ten, size(10.0, 0.0)), (zero, zero));
        assert_eq!(fitted(BoxFit::Fill, ten, size(0.0, 10.0)), (zero, zero));
    }

    /// The default: at its own size or smaller, centred, the whole picture.
    #[test]
    fn a_small_picture_sits_in_the_middle() {
        let image = DecorationImage::new(handle(10, 10));
        let got = pieces(&image, size(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(
            got,
            vec![ImagePiece {
                dst: Rect::new(45.0, 20.0, 10.0, 10.0),
                uv: Rect::new(0.0, 0.0, 1.0, 1.0),
                strict: false,
            }]
        );
        let corner = image.alignment(Alignment::TOP_LEFT);
        let got = pieces(&corner, size(10.0, 10.0), Rect::new(5.0, 5.0, 100.0, 50.0));
        assert_eq!(got[0].dst, Rect::new(5.0, 5.0, 10.0, 10.0));
        let empty = pieces(&corner, size(10.0, 10.0), Rect::new(5.0, 5.0, 0.0, 50.0));
        assert!(empty.is_empty(), "an empty box draws nothing");
    }

    /// A crop keeps the part the alignment names.
    #[test]
    fn a_cover_keeps_the_aligned_part() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let wide = size(20.0, 10.0);
        let left = DecorationImage::new(handle(20, 10))
            .fit(BoxFit::Cover)
            .alignment(Alignment::CENTER_LEFT);
        assert_eq!(
            pieces(&left, wide, rect)[0].uv,
            Rect::new(0.0, 0.0, 0.5, 1.0)
        );
        let right = left.clone().alignment(Alignment::CENTER_RIGHT);
        assert_eq!(
            pieces(&right, wide, rect)[0].uv,
            Rect::new(0.5, 0.0, 0.5, 1.0)
        );
        let centre = left.alignment(Alignment::CENTER);
        assert_eq!(
            pieces(&centre, wide, rect)[0].uv,
            Rect::new(0.25, 0.0, 0.5, 1.0)
        );
        let tall = DecorationImage::new(handle(10, 20))
            .fit(BoxFit::Cover)
            .alignment(Alignment::BOTTOM_CENTER);
        assert_eq!(
            pieces(&tall, size(10.0, 20.0), rect)[0].uv,
            Rect::new(0.0, 0.5, 1.0, 0.5)
        );
    }

    /// Repeated from where the picture sits, outwards, until the box is covered, and
    /// clipped to it.
    #[test]
    fn a_repeated_picture_covers_its_axis() {
        let image = DecorationImage::new(handle(10, 10)).repeat(ImageRepeat::RepeatX);
        let rect = Rect::new(0.0, 0.0, 25.0, 30.0);
        let (got, clipped) = image.pieces(size(10.0, 10.0), rect, TextDirection::Ltr);
        let xs: Vec<f32> = got.iter().map(|p| p.dst.x).collect();
        assert_eq!(xs, vec![-2.5, 7.5, 17.5]);
        assert!(got.iter().all(|p| p.dst.y == 10.0), "across only");
        assert!(clipped, "a repeat is clipped to the box");
        let both = image.repeat(ImageRepeat::Repeat);
        assert_eq!(
            pieces(&both, size(10.0, 10.0), rect).len(),
            9,
            "three by three"
        );
        let down = both.repeat(ImageRepeat::RepeatY);
        let got = pieces(&down, size(10.0, 10.0), rect);
        let ys: Vec<f32> = got.iter().map(|p| p.dst.y).collect();
        assert_eq!(ys, vec![0.0, 10.0, 20.0]);
        assert!(got.iter().all(|p| p.dst.x == 7.5), "down only");
        let once = down.repeat(ImageRepeat::NoRepeat);
        let (got, clipped) = once.pieces(size(10.0, 10.0), rect, TextDirection::Ltr);
        assert_eq!(got.len(), 1);
        assert!(!clipped);
    }

    /// A picture that already fills the box has nothing to repeat.
    #[test]
    fn a_picture_filling_its_box_is_drawn_once() {
        let image = DecorationImage::new(handle(10, 10))
            .fit(BoxFit::Fill)
            .repeat(ImageRepeat::Repeat);
        let rect = Rect::new(0.0, 0.0, 40.0, 40.0);
        let (got, clipped) = image.pieces(size(10.0, 10.0), rect, TextDirection::Ltr);
        assert_eq!(got.len(), 1);
        assert!(!clipped);
    }

    #[test]
    fn a_tiny_pattern_stops_at_the_tile_limit() {
        let image = DecorationImage::new(handle(1, 1)).repeat(ImageRepeat::Repeat);
        let got = pieces(&image, size(1.0, 1.0), Rect::new(0.0, 0.0, 200.0, 200.0));
        assert_eq!(got.len(), MAX_IMAGE_TILES);
    }

    fn sliced() -> DecorationImage {
        DecorationImage::new(handle(30, 30)).center_slice(Rect::new(10.0, 10.0, 10.0, 10.0))
    }

    /// The corners keep their size, the edges stretch along their length, the centre
    /// both ways.
    #[test]
    fn a_centre_slice_stretches_the_middle_only() {
        let got = pieces(
            &sliced(),
            size(30.0, 30.0),
            Rect::new(0.0, 0.0, 100.0, 60.0),
        );
        assert_eq!(got.len(), 9);
        let third = 1.0 / 3.0;
        assert_eq!(
            got[0].dst,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            "a corner, its own size"
        );
        assert_eq!(got[0].uv, Rect::new(0.0, 0.0, third, third));
        assert_eq!(
            got[1].dst,
            Rect::new(10.0, 0.0, 80.0, 10.0),
            "an edge, along its length"
        );
        assert_eq!(
            got[4].dst,
            Rect::new(10.0, 10.0, 80.0, 40.0),
            "the centre, stretched"
        );
        assert_eq!(got[4].uv, Rect::new(third, third, third, third));
        assert!(
            got.iter().all(|p| p.strict),
            "each part sampled within itself"
        );
        assert_eq!(got[8].dst, Rect::new(90.0, 50.0, 10.0, 10.0));
        assert_eq!(got[8].uv, Rect::new(2.0 * third, 2.0 * third, third, third));
    }

    /// Corners bigger than the box keep their size, centred on it, and the middle has no
    /// room: the reference's, which draws past the box. A decoration clips it to its
    /// shape.
    #[test]
    fn a_slice_in_a_small_box_keeps_its_corners() {
        let got = pieces(&sliced(), size(30.0, 30.0), Rect::new(0.0, 0.0, 10.0, 10.0));
        let dsts: Vec<Rect> = got.iter().map(|p| p.dst).collect();
        assert_eq!(
            dsts,
            vec![
                Rect::new(-5.0, -5.0, 10.0, 10.0),
                Rect::new(5.0, -5.0, 10.0, 10.0),
                Rect::new(-5.0, 5.0, 10.0, 10.0),
                Rect::new(5.0, 5.0, 10.0, 10.0),
            ]
        );
    }

    /// A sliced picture repeats as a whole, each copy sliced.
    #[test]
    fn a_repeated_slice_is_sliced_per_copy() {
        let image = sliced().fit(BoxFit::Contain).repeat(ImageRepeat::RepeatX);
        let got = pieces(&image, size(30.0, 30.0), Rect::new(0.0, 0.0, 90.0, 30.0));
        assert_eq!(got.len(), 27, "three copies of nine");
    }

    /// At a scale of two, a picture's pixels are half a logical pixel each.
    #[test]
    fn a_scaled_picture_is_drawn_smaller() {
        let image = DecorationImage::new(handle(20, 20))
            .scale(2.0)
            .alignment(Alignment::TOP_LEFT);
        let got = pieces(&image, size(20.0, 20.0), Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(got[0].dst, Rect::new(0.0, 0.0, 10.0, 10.0));
        let sliced = DecorationImage::new(handle(30, 30))
            .scale(3.0)
            .center_slice(Rect::new(2.0, 2.0, 6.0, 6.0));
        let got = pieces(&sliced, size(30.0, 30.0), Rect::new(0.0, 0.0, 40.0, 40.0));
        assert_eq!(
            got[0].dst,
            Rect::new(0.0, 0.0, 2.0, 2.0),
            "a corner at its scaled size"
        );
        assert_eq!(got[0].uv, Rect::new(0.0, 0.0, 0.2, 0.2));
        assert_eq!(got[8].dst, Rect::new(38.0, 38.0, 2.0, 2.0));
    }

    /// Mirrored in a right-to-left box: the picture lands where the alignment puts it,
    /// read backwards.
    #[test]
    fn a_directional_picture_is_mirrored_in_rtl() {
        let image = DecorationImage::new(handle(10, 10))
            .alignment(Alignment::TOP_LEFT)
            .match_text_direction(true);
        let rect = Rect::new(20.0, 0.0, 100.0, 50.0);
        let ltr = image.pieces(size(10.0, 10.0), rect, TextDirection::Ltr).0;
        assert_eq!(ltr[0].uv, Rect::new(0.0, 0.0, 1.0, 1.0));
        let rtl = image.pieces(size(10.0, 10.0), rect, TextDirection::Rtl).0;
        assert_eq!(rtl[0].dst, Rect::new(20.0, 0.0, 10.0, 10.0));
        assert_eq!(rtl[0].uv, Rect::new(1.0, 0.0, -1.0, 1.0));
        // A start alignment is the right in a right-to-left box.
        let start = DecorationImage::new(handle(10, 10)).alignment(AlignmentDirectional::TOP_START);
        let rtl = start.pieces(size(10.0, 10.0), rect, TextDirection::Rtl).0;
        assert_eq!(rtl[0].dst, Rect::new(110.0, 0.0, 10.0, 10.0));
        assert_eq!(
            rtl[0].uv,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            "not mirrored unless asked"
        );
    }

    /// A mirrored slice is the whole slice mirrored: the left border lands on the right.
    #[test]
    fn a_mirrored_slice_swaps_its_sides() {
        let image = DecorationImage::new(handle(30, 10))
            .center_slice(Rect::new(5.0, 0.0, 5.0, 10.0))
            .match_text_direction(true);
        let rect = Rect::new(0.0, 0.0, 100.0, 10.0);
        let got = image.pieces(size(30.0, 10.0), rect, TextDirection::Rtl).0;
        // The picture's left border, five wide, is drawn at the right edge, reversed.
        assert_eq!(got[0].dst, Rect::new(95.0, 0.0, 5.0, 10.0));
        assert_eq!(got[0].uv, Rect::new(5.0 / 30.0, 0.0, -5.0 / 30.0, 1.0));
        // Its right border, twenty wide, at the left.
        assert_eq!(got[2].dst, Rect::new(0.0, 0.0, 20.0, 10.0));
    }

    fn painted(image: &DecorationImage, blend: f32) -> Scene {
        let mut scene = Scene::new();
        image.paint(
            &mut scene,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            TextDirection::Ltr,
            blend,
        );
        scene
    }

    fn tints(primitives: &[Primitive]) -> Vec<f32> {
        primitives
            .iter()
            .flat_map(|p| match p {
                Primitive::Image { tint, .. } => vec![tint.a],
                Primitive::Layer { primitives, .. } => tints(primitives),
                _ => vec![],
            })
            .collect()
    }

    #[test]
    fn a_picture_paints_at_its_opacity() {
        let image = DecorationImage::new(handle(10, 10)).opacity(0.5);
        assert_eq!(tints(painted(&image, 0.5).primitives()), vec![0.25]);
        assert!(painted(&DecorationImage::new(None), 1.0)
            .primitives()
            .is_empty());
        assert!(painted(&image.opacity(0.0), 1.0).primitives().is_empty());
    }

    /// The colour filter, then the inversion of its result.
    #[test]
    fn the_filter_comes_before_the_inversion() {
        let filtered = DecorationImage::new(handle(10, 10))
            .color_filter(ColorFilter::grayscale())
            .invert_colors(true);
        let scene = painted(&filtered, 1.0);
        let Primitive::Layer {
            primitives, filter, ..
        } = &scene.primitives()[0]
        else {
            panic!("a layer");
        };
        assert_eq!(filter.color, Some(ColorFilter::invert()), "inverted last");
        let Primitive::Layer { filter, .. } = &primitives[0] else {
            panic!("the filter's layer, inside");
        };
        assert_eq!(filter.color, Some(ColorFilter::grayscale()));
        let plain = painted(&DecorationImage::new(handle(10, 10)), 1.0);
        assert!(
            matches!(plain.primitives()[0], Primitive::Image { .. }),
            "no layer for nothing"
        );
        let inverted = painted(
            &DecorationImage::new(handle(10, 10)).invert_colors(true),
            1.0,
        );
        assert!(
            matches!(&inverted.primitives()[0], Primitive::Layer { filter, .. } if filter.color == Some(ColorFilter::invert()))
        );
    }

    #[test]
    fn a_repeated_picture_is_clipped_to_its_box() {
        let mut scene = Scene::new();
        scene.set_clip(Rect::new(0.0, 0.0, 30.0, 300.0));
        DecorationImage::new(handle(10, 10))
            .repeat(ImageRepeat::Repeat)
            .paint(
                &mut scene,
                Rect::new(5.0, 5.0, 40.0, 40.0),
                TextDirection::Ltr,
                1.0,
            );
        assert_eq!(scene.primitives().len(), 25, "five by five");
        for p in scene.primitives() {
            let Primitive::Image { clip, .. } = p else {
                panic!("tiles");
            };
            assert_eq!(*clip, Rect::new(5.0, 5.0, 25.0, 40.0));
        }
        assert_eq!(
            scene.current_clip(),
            Rect::new(0.0, 0.0, 30.0, 300.0),
            "restored"
        );
    }

    /// Two pictures: the leaving one whole, the arriving one over it at `t`. One alone
    /// fades.
    #[test]
    fn two_pictures_cross_over() {
        let a = DecorationImage::new(handle(10, 10));
        let b = DecorationImage::new(handle(12, 12)).opacity(0.8);
        assert_eq!(
            DecorationImage::lerp(Some(&a), Some(&b), 0.0),
            Some(a.clone())
        );
        assert_eq!(
            DecorationImage::lerp(Some(&a), Some(&b), 1.0),
            Some(b.clone())
        );
        assert_eq!(
            DecorationImage::lerp(Some(&a), Some(&a), 0.5),
            Some(a.clone())
        );
        let mid = DecorationImage::lerp(Some(&a), Some(&b), 0.25).unwrap();
        assert_eq!(mid.opacity, 0.8, "the arriving picture's settings");
        assert_eq!(tints(painted(&mid, 1.0).primitives()), vec![1.0, 0.2]);
        // Faded as a pair, in one group.
        let scene = painted(&mid, 0.5);
        assert!(
            matches!(scene.primitives(), [Primitive::Layer { opacity, .. }] if *opacity == 0.5)
        );
        let leaving = DecorationImage::lerp(Some(&a), None, 0.25).unwrap();
        assert_eq!(tints(painted(&leaving, 1.0).primitives()), vec![0.75]);
        let arriving = DecorationImage::lerp(None, Some(&a), 0.25).unwrap();
        assert_eq!(tints(painted(&arriving, 1.0).primitives()), vec![0.25]);
        assert_eq!(DecorationImage::lerp(None, None, 0.5), None);
    }
}
