// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! `frus-svg` — **SVG pictures drawn as frus's own vector paths**.
//!
//! An SVG file is parsed once ([`Svg::parse`]) into shapes in the picture's own
//! coordinates, and painted into a [`Scene`] at any size ([`Svg::paint`]). Nothing is
//! rasterised: every shape is a [`Path`] the renderer tessellates at the size it is shown,
//! so a picture stays sharp on any screen and at any zoom.
//!
//! What is drawn: paths, with their fill rule; strokes, with their width, caps, joins,
//! miter limit and dashes (expanded into outlines, which is also what lets a stroke take a
//! gradient); flat colours, linear and radial gradients with any number of stops, their
//! spread method and their own transform; group opacity; clip paths; masks, by luminance
//! or by alpha; filters built from Gaussian blurs, drop shadows, offsets, floods, merges,
//! `over`, `in` and `out` composites, normal blends and colour matrices (see below);
//! embedded PNG and JPEG bitmaps, decoded once, when the picture is parsed, and embedded
//! SVG pictures, drawn as vectors like the rest. What is not yet: patterns, the other filter
//! primitives, GIF and WebP bitmaps, and text — each is skipped, and named in
//! [`Svg::unsupported`], so an application can say why a picture looks incomplete.
//!
//! Masks and filters are layers of the scene: a mask is a layer meeting the masked group
//! with `DstIn`, a blur a blurred layer, and so on. Filters mix colours in linear light by
//! default and frus's colour matrix works on sRGB values, so a colour matrix is drawn when
//! the picture asks for sRGB, or when it only keeps channels, sets them to constants and
//! scales alpha — what design tools export for shadows; `atop`, `xor` and `lighter`
//! composites likewise in sRGB only. A filter with a primitive that is not drawn is left
//! off entirely — the group is drawn unfiltered — and named `"filter"`.
//!
//! A bitmap is read only from the picture itself, as a `data:` address: a picture never
//! makes the application open a file it names.
//!
//! Parsing is [`usvg`]'s, which resolves CSS, `<use>`, units, `objectBoundingBox`
//! gradients and the rest into a simple tree.

#![deny(missing_docs)]

use std::fmt;
use std::sync::Arc;

use frus_core::{
    Affine, AlignmentGeometry, BlendMode, BoxFit, ClipShape, Color, ColorFilter, FillRule,
    GradientRamp, ImageFilter, ImageHandle, LayerFilter, Path, Point, Primitive, RampShape, Rect,
    Scene, Size, TextDirection, TileMode,
};
use usvg::tiny_skia_path;

mod effect;

/// A parsing failure: the bytes are not an SVG this parser can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgError(String);

impl SvgError {
    /// The underlying message.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SVG parsing failed: {}", self.0)
    }
}

impl std::error::Error for SvgError {}

/// What a shape is filled with.
#[derive(Clone, Debug, PartialEq)]
enum Paint {
    Solid(Color),
    /// In the picture's coordinates.
    Ramp(GradientRamp),
}

/// One thing the picture draws, in order.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    /// A filled outline — a fill, or a stroke expanded into one.
    Shape {
        path: Path,
        paint: Paint,
        opacity: f32,
    },
    /// Items drawn together, as one layer: faded as one, clipped to an outline, filtered,
    /// or meeting what is under them by a blend mode — which is what a mask does.
    Group {
        opacity: f32,
        clip: Option<Path>,
        effect: Effect,
        items: Vec<Item>,
    },
    /// A bitmap, its pixels taken to the picture's coordinates by `transform`.
    Image {
        image: ImageHandle,
        transform: Affine,
    },
}

/// What a group's layer does to its pixels, beyond fading and clipping them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Effect {
    /// A Gaussian blur's standard deviations, in the picture's units along its axes.
    blur: Option<(f32, f32)>,
    /// A function of each pixel, on straight sRGB-encoded values.
    color: Option<ColorFilter>,
    /// How the layer meets what is drawn under it in its parent: `DstIn` keeps the parent's
    /// pixels where this layer is opaque, which is what a mask is.
    blend: Option<BlendMode>,
    /// Pixels that only shape another group — a mask — and that a tint must not recolour,
    /// or the mask would change with the theme.
    untinted: bool,
}

impl Effect {
    /// The effect carried into another picture by `map`: a blur's reach follows the scale.
    fn mapped(self, map: Affine) -> Effect {
        let (sx, sy) = axis_scales(map);
        Effect {
            blur: self.blur.map(|(x, y)| (x * sx, y * sy)),
            ..self
        }
    }
}

/// How much `map` stretches lengths along each of its axes.
fn axis_scales(map: Affine) -> (f32, f32) {
    let [a, b, c, d, _, _] = map.m;
    (a.hypot(b), c.hypot(d))
}

impl Item {
    /// A layer holding `items`, unfaded: what a blend inside it needs, to meet only them.
    fn layer(effect: Effect, clip: Option<Path>, items: Vec<Item>) -> Item {
        Item::Group {
            opacity: 1.0,
            clip,
            effect,
            items,
        }
    }

    /// The item moved into another picture by `map` — an SVG drawn inside an SVG.
    fn mapped(self, map: Affine) -> Item {
        match self {
            Item::Shape {
                path,
                paint,
                opacity,
            } => Item::Shape {
                path: mapped(&path, map),
                paint: match paint {
                    Paint::Ramp(ramp) => Paint::Ramp(GradientRamp {
                        transform: ramp.transform.then(map),
                        ..ramp
                    }),
                    solid => solid,
                },
                opacity,
            },
            Item::Group {
                opacity,
                clip,
                effect,
                items,
            } => Item::Group {
                opacity,
                clip: clip.map(|clip| mapped(&clip, map)),
                effect: effect.mapped(map),
                items: items.into_iter().map(|item| item.mapped(map)).collect(),
            },
            Item::Image { image, transform } => Item::Image {
                image,
                transform: transform.then(map),
            },
        }
    }
}

/// **A parsed SVG picture**, ready to paint at any size.
///
/// ```
/// let svg = frus_svg::Svg::parse(br##"
///     <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
///       <circle cx="12" cy="12" r="10" fill="#1e88e5"/>
///     </svg>"##).unwrap();
/// assert_eq!(svg.size(), frus_core::Size::new(24.0, 24.0));
/// assert!(svg.unsupported().is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Svg {
    size: Size,
    items: Vec<Item>,
    unsupported: Vec<&'static str>,
}

/// A parsed picture behind a shared handle, as the widget layer caches it.
pub type SvgHandle = Arc<Svg>;

impl Svg {
    /// Parses SVG source — bytes of an `.svg` file, UTF-8.
    pub fn parse(bytes: &[u8]) -> Result<Svg, SvgError> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
                // An address that is not `data:` would be read from the disk.
                resolve_string: Box::new(|_, _| None),
            },
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_data(bytes, &options).map_err(|e| SvgError(e.to_string()))?;
        let mut unsupported = Vec::new();
        let items = group_items(tree.root(), &mut unsupported);
        unsupported.sort_unstable();
        unsupported.dedup();
        Ok(Svg {
            size: Size::new(tree.size().width(), tree.size().height()),
            items,
            unsupported,
        })
    }

    /// The picture's own size, in its own units — its `width` and `height`, or its
    /// `viewBox` when it gives none.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The SVG features this picture uses that are not drawn yet, by name (`"filter"` — a
    /// filter with a primitive not drawn, its group then drawn unfiltered —, `"pattern"`, `"image"` — a GIF, a WebP or a broken bitmap —, `"text"`,
    /// `"blend mode"`): empty when the whole picture is drawn.
    pub fn unsupported(&self) -> &[&'static str] {
        &self.unsupported
    }

    /// How many shapes the picture draws — fills and strokes counted apart.
    pub fn shape_count(&self) -> usize {
        fn count(items: &[Item]) -> usize {
            items
                .iter()
                .map(|item| match item {
                    Item::Shape { .. } => 1,
                    Item::Group { items, .. } => count(items),
                    Item::Image { .. } => 0,
                })
                .sum()
        }
        count(&self.items)
    }

    /// **Paints the picture into `bounds`**, fitted by `fit` and placed by `alignment`
    /// as an image is, at `opacity`. With `tint`, every shape takes that colour, keeping
    /// its own transparency — a single-colour icon recoloured to the theme; a bitmap in
    /// the picture keeps its own colours.
    pub fn paint(
        &self,
        scene: &mut Scene,
        bounds: Rect,
        fit: BoxFit,
        alignment: AlignmentGeometry,
        tint: Option<Color>,
        opacity: f32,
    ) {
        if self.size.width <= 0.0 || self.size.height <= 0.0 || opacity <= 0.0 {
            return;
        }
        let (dst, uv) = fit.apply_aligned(self.size, bounds, alignment.resolve(TextDirection::Ltr));
        if dst.width <= 0.0 || dst.height <= 0.0 || uv.width <= 0.0 || uv.height <= 0.0 {
            return;
        }
        // The visible part of the picture (`uv`, a fraction of it) onto `dst`.
        let (w, h) = (self.size.width, self.size.height);
        let place = Affine::translation(-uv.x * w, -uv.y * h)
            .then(Affine::scale(
                dst.width / (uv.width * w),
                dst.height / (uv.height * h),
            ))
            .then(Affine::translation(dst.x, dst.y));
        // A cropped picture (`BoxFit::Cover`, `None`) must not spill out of its box.
        let outer = scene.current_clip();
        scene.set_clip(intersect(outer, dst));
        let start = scene.primitives().len();
        paint_items(scene, &self.items, place, tint);
        if opacity < 1.0 {
            wrap(scene, start, opacity, ClipShape::Rect, LayerFilter::NONE);
        }
        scene.set_clip(outer);
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// The primitives painted since `start`, moved into one layer.
///
/// A layer that blends with what is under it is kept even when it holds nothing: a mask
/// with nothing in it hides everything.
fn wrap(scene: &mut Scene, start: usize, opacity: f32, clip_shape: ClipShape, filter: LayerFilter) {
    let primitives = scene.split_off(start);
    if primitives.is_empty() && filter.blend.is_none() {
        return;
    }
    scene.push_primitive(Primitive::Layer {
        primitives,
        opacity,
        clip: scene.current_clip(),
        clip_shape,
        transform: None,
        filter,
        owner: scene.current_owner(),
    });
}

fn paint_items(scene: &mut Scene, items: &[Item], place: Affine, tint: Option<Color>) {
    for item in items {
        match item {
            Item::Shape {
                path,
                paint,
                opacity,
            } => {
                let path = mapped(path, place);
                match (paint, tint) {
                    (Paint::Solid(color), Some(tint)) => {
                        scene.fill_path(&path, tint.fade(color.a * opacity))
                    }
                    (Paint::Ramp(ramp), Some(tint)) => {
                        let alpha = ramp.stops.iter().map(|(_, c)| c.a).fold(0.0, f32::max);
                        scene.fill_path(&path, tint.fade(alpha * opacity))
                    }
                    (Paint::Solid(color), None) => scene.fill_path(&path, color.fade(*opacity)),
                    (Paint::Ramp(ramp), None) => {
                        let ramp = GradientRamp {
                            transform: ramp.transform.then(place),
                            ..ramp.clone()
                        };
                        scene.fill_path_ramp(&path, ramp, *opacity)
                    }
                }
            }
            Item::Group {
                opacity,
                clip,
                effect,
                items,
            } => {
                let start = scene.primitives().len();
                let tint = if effect.untinted { None } else { tint };
                paint_items(scene, items, place, tint);
                let clip_shape = match clip {
                    Some(clip) => ClipShape::Path(mapped(clip, place)),
                    None => ClipShape::Rect,
                };
                // `place` only moves and scales: a blur's reach scales with it, per axis.
                let (sx, sy) = axis_scales(place);
                let filter = LayerFilter {
                    image: effect.blur.map(|(x, y)| ImageFilter::Blur {
                        sigma_x: x * sx,
                        sigma_y: y * sy,
                    }),
                    color: effect.color,
                    blend: effect.blend,
                    ..LayerFilter::NONE
                };
                wrap(scene, start, *opacity, clip_shape, filter);
            }
            Item::Image { image, transform } => {
                scene.draw_image_mapped(image, transform.then(place), Color::WHITE)
            }
        }
    }
}

/// `path` with every point taken through `map` — an affine map of a Bézier's control
/// points is the same Bézier mapped.
fn mapped(path: &Path, map: Affine) -> Path {
    let mut out = Path::new().with_fill_rule(path.fill_rule());
    for verb in path.verbs() {
        out = match *verb {
            frus_core::PathVerb::MoveTo(p) => out.move_to(map.apply(p)),
            frus_core::PathVerb::LineTo(p) => out.line_to(map.apply(p)),
            frus_core::PathVerb::QuadTo { ctrl, to } => out.quad_to(map.apply(ctrl), map.apply(to)),
            frus_core::PathVerb::CubicTo { c1, c2, to } => {
                out.cubic_to(map.apply(c1), map.apply(c2), map.apply(to))
            }
            frus_core::PathVerb::Close => out.close(),
        };
    }
    out
}

/// usvg's transform as frus's: `x' = sx·x + kx·y + tx`, `y' = ky·x + sy·y + ty`.
fn affine(t: usvg::Transform) -> Affine {
    Affine {
        m: [t.sx, t.ky, t.kx, t.sy, t.tx, t.ty],
    }
}

/// A tiny-skia path in the picture's coordinates, through `to_root`.
fn converted(data: &tiny_skia_path::Path, to_root: Affine, rule: FillRule) -> Path {
    let p = |p: tiny_skia_path::Point| to_root.apply(Point::new(p.x, p.y));
    let mut out = Path::new().with_fill_rule(rule);
    for segment in data.segments() {
        out = match segment {
            tiny_skia_path::PathSegment::MoveTo(a) => out.move_to(p(a)),
            tiny_skia_path::PathSegment::LineTo(a) => out.line_to(p(a)),
            tiny_skia_path::PathSegment::QuadTo(c, a) => out.quad_to(p(c), p(a)),
            tiny_skia_path::PathSegment::CubicTo(c1, c2, a) => out.cubic_to(p(c1), p(c2), p(a)),
            tiny_skia_path::PathSegment::Close => out.close(),
        };
    }
    out
}

fn color(c: usvg::Color, alpha: f32) -> Color {
    Color::rgba8(
        c.red,
        c.green,
        c.blue,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// A paint in the picture's coordinates, for a shape whose own space goes to the
/// picture's through `to_root`. `None` for what is not drawn.
fn paint(
    paint: &usvg::Paint,
    opacity: f32,
    to_root: Affine,
    unsupported: &mut Vec<&'static str>,
) -> Option<(Paint, f32)> {
    let ramp = |base: &usvg::BaseGradient, shape: RampShape| {
        let tile = match base.spread_method() {
            usvg::SpreadMethod::Pad => TileMode::Clamp,
            usvg::SpreadMethod::Reflect => TileMode::Mirror,
            usvg::SpreadMethod::Repeat => TileMode::Repeated,
        };
        let stops = base
            .stops()
            .iter()
            .map(|s| (s.offset().get(), color(s.color(), s.opacity().get())))
            .collect();
        Paint::Ramp(GradientRamp {
            shape,
            stops,
            tile,
            transform: affine(base.transform()).then(to_root),
        })
    };
    match paint {
        usvg::Paint::Color(c) => Some((Paint::Solid(color(*c, 1.0)), opacity)),
        usvg::Paint::LinearGradient(g) => Some((
            ramp(
                g,
                RampShape::Linear {
                    from: Point::new(g.x1(), g.y1()),
                    to: Point::new(g.x2(), g.y2()),
                },
            ),
            opacity,
        )),
        usvg::Paint::RadialGradient(g) => Some((
            ramp(
                g,
                RampShape::Radial {
                    center: Point::new(g.cx(), g.cy()),
                    radius: g.r().get(),
                    focal: Point::new(g.fx(), g.fy()),
                    focal_radius: g.fr().get(),
                },
            ),
            opacity,
        )),
        usvg::Paint::Pattern(_) => {
            unsupported.push("pattern");
            None
        }
    }
}

fn group_items(group: &usvg::Group, unsupported: &mut Vec<&'static str>) -> Vec<Item> {
    let mut items = Vec::new();
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => {
                if child.blend_mode() != usvg::BlendMode::Normal {
                    unsupported.push("blend mode");
                }
                let to_root = affine(child.abs_transform());
                let mut inner = group_items(child, unsupported);
                // In the reference renderer's order: the filters, then the clip and the
                // mask, then the opacity.
                if !child.filters().is_empty() {
                    match effect::filtered(child.filters(), inner.clone(), to_root) {
                        Some(output) => inner = output,
                        // Drawn unfiltered, and said so.
                        None => unsupported.push("filter"),
                    }
                }
                let opacity = child.opacity().get();
                let clip = child
                    .clip_path()
                    .map(|clip| clip_outline(clip, to_root, unsupported));
                let mut masked = false;
                if let Some(mask) = child.mask() {
                    if !effect::mask_layers(mask, to_root, &mut inner, unsupported) {
                        // A mask with nothing in it hides the whole group.
                        continue;
                    }
                    masked = true;
                }
                if opacity >= 1.0 && clip.is_none() && !masked {
                    items.extend(inner);
                } else {
                    items.push(Item::Group {
                        opacity,
                        clip,
                        effect: Effect::default(),
                        items: inner,
                    });
                }
            }
            usvg::Node::Path(path) => path_items(path, &mut items, unsupported),
            usvg::Node::Image(image) => image_items(image, &mut items, unsupported),
            usvg::Node::Text(_) => unsupported.push("text"),
        }
    }
    items
}

/// An embedded picture: a PNG or a JPEG decoded now, once, or an SVG drawn as vectors.
fn image_items(image: &usvg::Image, items: &mut Vec<Item>, unsupported: &mut Vec<&'static str>) {
    if !image.is_visible() {
        return;
    }
    // The picture fills `(0, 0)` to its size, in the space `abs_transform` takes to ours.
    let to_root = affine(image.abs_transform());
    let bytes = match image.kind() {
        usvg::ImageKind::PNG(bytes) | usvg::ImageKind::JPEG(bytes) => bytes,
        usvg::ImageKind::SVG(tree) => {
            let inner = group_items(tree.root(), unsupported);
            items.extend(inner.into_iter().map(|item| item.mapped(to_root)));
            return;
        }
        usvg::ImageKind::GIF(_) | usvg::ImageKind::WEBP(_) => {
            unsupported.push("image");
            return;
        }
    };
    let Ok(data) = frus_image::decode(bytes) else {
        unsupported.push("image");
        return;
    };
    let (pixels, size) = (data.size(), image.size());
    if pixels.width == 0.0 || pixels.height == 0.0 {
        return;
    }
    items.push(Item::Image {
        transform: Affine::scale(size.width() / pixels.width, size.height() / pixels.height)
            .then(to_root),
        image: data.into_handle(),
    });
}

/// A path's fill and stroke, in its paint order.
fn path_items(path: &usvg::Path, items: &mut Vec<Item>, unsupported: &mut Vec<&'static str>) {
    if !path.is_visible() {
        return;
    }
    let to_root = affine(path.abs_transform());
    // A line, horizontal or vertical, has no inside to fill: only its stroke draws.
    let bounds = path.data().bounds();
    let has_area = bounds.width() > 0.0 && bounds.height() > 0.0;
    let fill = path.fill().filter(|_| has_area).and_then(|fill| {
        let rule = match fill.rule() {
            usvg::FillRule::NonZero => FillRule::NonZero,
            usvg::FillRule::EvenOdd => FillRule::EvenOdd,
        };
        let (paint, opacity) = paint(fill.paint(), fill.opacity().get(), to_root, unsupported)?;
        Some(Item::Shape {
            path: converted(path.data(), to_root, rule),
            paint,
            opacity,
        })
    });
    let stroke = path.stroke().and_then(|stroke| {
        let outline = stroke_outline(path.data(), stroke)?;
        let (paint, opacity) = paint(stroke.paint(), stroke.opacity().get(), to_root, unsupported)?;
        Some(Item::Shape {
            // An outline crosses itself where the line turns; non-zero fills it whole.
            path: converted(&outline, to_root, FillRule::NonZero),
            paint,
            opacity,
        })
    });
    let (first, second) = match path.paint_order() {
        usvg::PaintOrder::FillAndStroke => (fill, stroke),
        usvg::PaintOrder::StrokeAndFill => (stroke, fill),
    };
    items.extend(first);
    items.extend(second);
}

/// How finely a stroke's curves are followed, as if drawn this many times larger than
/// the picture's units: an icon in a 24-unit box is shown at a hundred pixels and more.
const STROKE_RESOLUTION: f32 = 8.0;

/// A stroke as the outline it covers: dashed first if it is, then widened with its caps,
/// joins and miter limit.
fn stroke_outline(
    data: &tiny_skia_path::Path,
    stroke: &usvg::Stroke,
) -> Option<tiny_skia_path::Path> {
    let mut style = stroke.to_tiny_skia();
    let dashed;
    let source = match style.dash.take() {
        Some(dash) => {
            dashed = data.dash(&dash, STROKE_RESOLUTION)?;
            &dashed
        }
        None => data,
    };
    source.stroke(&style, STROKE_RESOLUTION)
}

/// A clip path as one outline in the picture's coordinates, `to_root` being the clipped
/// group's transform.
///
/// The outline is the union of the clip's shapes, filled non-zero. A shape with its own
/// `clip-rule="evenodd"` loses its holes, and a clip path that is itself clipped is drawn
/// unclipped: both rare, both noted.
fn clip_outline(
    clip: &usvg::ClipPath,
    to_root: Affine,
    unsupported: &mut Vec<&'static str>,
) -> Path {
    if clip.clip_path().is_some() {
        unsupported.push("nested clip path");
    }
    let to_root = affine(clip.transform()).then(to_root);
    let mut outline = Path::new().with_fill_rule(FillRule::NonZero);
    collect_clip(clip.root(), to_root, &mut outline, unsupported);
    outline
}

/// The shapes under `group`, a clip path's root or a group in it, appended to `outline`;
/// `to_root` takes the clip path's own space to the picture's.
///
/// The parser gives each shape in a clip path, as everywhere, an `abs_transform` that
/// already holds every group transform between it and the clip path's root (whose own is
/// the identity). So that transform alone places a shape in the clip's space: the groups
/// are walked, never applied again.
fn collect_clip(
    group: &usvg::Group,
    to_root: Affine,
    outline: &mut Path,
    unsupported: &mut Vec<&'static str>,
) {
    for node in group.children() {
        match node {
            usvg::Node::Path(path) if path.is_visible() => {
                let to_root = affine(path.abs_transform()).then(to_root);
                let shape = converted(path.data(), to_root, FillRule::NonZero);
                *outline = std::mem::take(outline).append(shape);
            }
            usvg::Node::Group(child) => collect_clip(child, to_root, outline, unsupported),
            usvg::Node::Text(_) => unsupported.push("text"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Svg {
        Svg::parse(source.as_bytes()).expect("parses")
    }

    fn painted(svg: &Svg, bounds: Rect, fit: BoxFit) -> Vec<Primitive> {
        let mut scene = Scene::new();
        scene.set_clip(Rect::new(-1000.0, -1000.0, 4000.0, 4000.0));
        svg.paint(
            &mut scene,
            bounds,
            fit,
            AlignmentGeometry::Physical(frus_core::Alignment::CENTER),
            None,
            1.0,
        );
        scene.primitives().to_vec()
    }

    fn first_point(p: &Primitive) -> Point {
        match p {
            Primitive::Path { path, .. } => match path.verbs()[0] {
                frus_core::PathVerb::MoveTo(p) => p,
                ref v => panic!("{v:?}"),
            },
            p => panic!("{p:?}"),
        }
    }

    #[test]
    fn a_rect_is_one_filled_path_at_the_size_shown() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <rect x="2" y="3" width="4" height="5" fill="#ff0000"/></svg>"##,
        );
        assert_eq!(svg.shape_count(), 1);
        let drawn = painted(&svg, Rect::new(100.0, 0.0, 100.0, 100.0), BoxFit::Contain);
        assert_eq!(drawn.len(), 1);
        // (2, 3) in a 10-unit picture shown 100 wide at x = 100.
        let p = first_point(&drawn[0]);
        assert!(
            (p.x - 120.0).abs() < 1e-3 && (p.y - 30.0).abs() < 1e-3,
            "{p:?}"
        );
        match &drawn[0] {
            Primitive::Path { fill, path, .. } => {
                assert_eq!(*fill, Some(Color::rgba8(255, 0, 0, 255)));
                assert_eq!(path.fill_rule(), FillRule::NonZero, "SVG's default");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn transforms_reach_the_points() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <g transform="translate(5 0) scale(2)"><rect x="1" y="1" width="1" height="1"/></g></svg>"##,
        );
        let p = first_point(&painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill)[0]);
        assert!(
            (p.x - 7.0).abs() < 1e-3 && (p.y - 2.0).abs() < 1e-3,
            "{p:?}"
        );
    }

    #[test]
    fn a_stroke_becomes_a_non_zero_outline_around_its_line() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
            <line x1="2" y1="10" x2="18" y2="10" stroke="#00ff00" stroke-width="4"
                  stroke-linecap="round"/></svg>"##,
        );
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 20.0, 20.0), BoxFit::Fill);
        assert_eq!(drawn.len(), 1, "a line has no fill, only its stroke");
        let Primitive::Path { path, fill, .. } = &drawn[0] else {
            unreachable!()
        };
        assert_eq!(path.fill_rule(), FillRule::NonZero);
        assert_eq!(*fill, Some(Color::rgba8(0, 255, 0, 255)));
        // The round caps reach 2 past each end, the width 2 either side.
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for v in path.verbs() {
            if let frus_core::PathVerb::LineTo(p)
            | frus_core::PathVerb::MoveTo(p)
            | frus_core::PathVerb::QuadTo { to: p, .. }
            | frus_core::PathVerb::CubicTo { to: p, .. } = v
            {
                x0 = x0.min(p.x);
                x1 = x1.max(p.x);
                y0 = y0.min(p.y);
                y1 = y1.max(p.y);
            }
        }
        assert!(
            x0 < 1.0 && x1 > 19.0 && y0 < 9.0 && y1 > 11.0,
            "{x0} {x1} {y0} {y1}"
        );
    }

    #[test]
    fn a_dashed_stroke_is_several_pieces() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
            <line x1="0" y1="10" x2="20" y2="10" stroke="#000" stroke-width="2"
                  stroke-dasharray="4 4"/></svg>"##,
        );
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 20.0, 20.0), BoxFit::Fill);
        let Primitive::Path { path, .. } = &drawn[0] else {
            unreachable!()
        };
        let pieces = path
            .verbs()
            .iter()
            .filter(|v| matches!(v, frus_core::PathVerb::MoveTo(_)))
            .count();
        assert!(pieces >= 3, "dashes: {pieces}");
    }

    #[test]
    fn a_gradient_follows_the_picture_onto_the_screen() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><linearGradient id="g" x1="0" y1="0" x2="10" y2="0" gradientUnits="userSpaceOnUse">
              <stop offset="0" stop-color="#000"/><stop offset="0.5" stop-color="#f00"/>
              <stop offset="1" stop-color="#fff"/></linearGradient></defs>
            <rect width="10" height="10" fill="url(#g)"/></svg>"##,
        );
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 100.0, 100.0), BoxFit::Fill);
        let Primitive::Path {
            gradient: Some(frus_core::PathGradient::Ramp(ramp)),
            ..
        } = &drawn[0]
        else {
            panic!("a ramp: {:?}", drawn[0])
        };
        assert_eq!(ramp.stops.len(), 3);
        // A quarter of the way across the box is a quarter of the way through.
        let t = ramp.t_at(Point::new(25.0, 50.0)).unwrap();
        assert!((t - 0.25).abs() < 1e-3, "{t}");
    }

    #[test]
    fn an_object_bounding_box_gradient_spans_its_shape() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <defs><radialGradient id="g"><stop offset="0" stop-color="#fff"/>
              <stop offset="1" stop-color="#00f"/></radialGradient></defs>
            <rect x="20" y="40" width="60" height="20" fill="url(#g)"/></svg>"##,
        );
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 100.0, 100.0), BoxFit::Fill);
        let Primitive::Path {
            gradient: Some(frus_core::PathGradient::Ramp(ramp)),
            ..
        } = &drawn[0]
        else {
            panic!()
        };
        // The centre of the box is the start; its right and bottom edges, the end.
        assert!(ramp.t_at(Point::new(50.0, 50.0)).unwrap() < 1e-3);
        assert!((ramp.t_at(Point::new(80.0, 50.0)).unwrap() - 1.0).abs() < 1e-3);
        assert!((ramp.t_at(Point::new(50.0, 60.0)).unwrap() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn group_opacity_and_clip_paths_become_layers() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><clipPath id="c"><circle cx="5" cy="5" r="4"/></clipPath></defs>
            <g opacity="0.5"><rect width="5" height="5"/><rect x="3" y="3" width="5" height="5"/></g>
            <g clip-path="url(#c)"><rect width="10" height="10" fill="#f00"/></g></svg>"##,
        );
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill);
        assert_eq!(drawn.len(), 2);
        assert!(
            matches!(&drawn[0], Primitive::Layer { opacity, primitives, .. }
            if (*opacity - 0.5).abs() < 1e-6 && primitives.len() == 2)
        );
        assert!(matches!(
            &drawn[1],
            Primitive::Layer {
                clip_shape: ClipShape::Path(_),
                ..
            }
        ));
    }

    #[test]
    fn a_clip_shapes_transform_is_applied_once() {
        // The parser wraps the transformed rect in a group whose transform is also in the
        // rect's own absolute one (milestone 740).
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
            <defs><clipPath id="c" transform="translate(1 0)">
              <rect width="2" height="2" transform="translate(5 2)"/></clipPath></defs>
            <g transform="translate(0 3)">
              <rect width="20" height="20" fill="#f00" clip-path="url(#c)"/></g></svg>"##,
        );
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 20.0, 20.0), BoxFit::Fill);
        let Primitive::Layer {
            clip_shape: ClipShape::Path(clip),
            ..
        } = &drawn[0]
        else {
            panic!("{drawn:?}")
        };
        let frus_core::PathVerb::MoveTo(p) = clip.verbs()[0] else {
            panic!("{clip:?}")
        };
        // The shape's, the clip's and the clipped element's transforms, each once.
        assert!(
            (p.x - 6.0).abs() < 1e-3 && (p.y - 5.0).abs() < 1e-3,
            "{p:?}"
        );
    }

    #[test]
    fn a_tint_recolours_every_shape_and_keeps_its_transparency() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <rect width="5" height="5" fill="#f00" fill-opacity="0.5"/></svg>"##,
        );
        let mut scene = Scene::new();
        let tint = Color::rgb(0.0, 0.0, 1.0);
        svg.paint(
            &mut scene,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            BoxFit::Fill,
            AlignmentGeometry::Physical(frus_core::Alignment::CENTER),
            Some(tint),
            1.0,
        );
        let Primitive::Path {
            fill: Some(fill), ..
        } = &scene.primitives()[0]
        else {
            panic!()
        };
        assert_eq!((fill.r, fill.g, fill.b), (0.0, 0.0, 1.0));
        assert!((fill.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn what_is_not_drawn_is_named() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><filter id="f"><feTurbulence baseFrequency="0.1"/></filter>
              <pattern id="p" width="2" height="2" patternUnits="userSpaceOnUse">
                <rect width="1" height="1"/></pattern></defs>
            <rect width="5" height="5" filter="url(#f)"/>
            <rect x="5" width="5" height="5" fill="url(#p)"/></svg>"##,
        );
        assert_eq!(svg.unsupported(), &["filter", "pattern"]);
        // The filtered square is drawn as it is, unfiltered; the pattern is not drawn.
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill);
        assert_eq!(drawn.len(), 1);
        assert!(matches!(&drawn[0], Primitive::Path { .. }), "{drawn:?}");
    }

    /// The layer's filter, or a panic naming what was found.
    fn layer(p: &Primitive) -> (&LayerFilter, &ClipShape, &[Primitive]) {
        match p {
            Primitive::Layer {
                filter,
                clip_shape,
                primitives,
                ..
            } => (filter, clip_shape, primitives),
            p => panic!("a layer: {p:?}"),
        }
    }

    #[test]
    fn an_alpha_mask_is_a_layer_meeting_the_group_with_dst_in() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><mask id="m" mask-type="alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="10" height="5">
              <rect width="10" height="10" fill="#000" fill-opacity="0.5"/></mask></defs>
            <rect width="10" height="10" fill="#f00" mask="url(#m)"/></svg>"##,
        );
        assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 100.0, 100.0), BoxFit::Fill);
        assert_eq!(drawn.len(), 1, "one isolated group: {drawn:?}");
        let (group, _, inside) = layer(&drawn[0]);
        assert!(group.is_none());
        // The red square, then the mask over it.
        assert_eq!(inside.len(), 2, "{inside:?}");
        assert!(matches!(&inside[0], Primitive::Path { fill: Some(c), .. } if c.r == 1.0));
        let (mask, shape, coverage) = layer(&inside[1]);
        assert_eq!(mask.blend, Some(BlendMode::DstIn));
        assert_eq!(mask.color, None);
        // Unclipped itself — what it does not cover, it hides —, its content clipped to
        // the mask's region.
        assert!(matches!(shape, ClipShape::Rect));
        assert_eq!(coverage.len(), 1);
        let (_, region, content) = layer(&coverage[0]);
        assert!(matches!(region, ClipShape::Path(_)));
        assert!(matches!(&content[0], Primitive::Path { fill: Some(c), .. }
            if (c.a - 0.5).abs() < 0.01));
    }

    #[test]
    fn a_luminance_mask_keeps_the_content_by_its_luminance() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><mask id="m"><rect width="10" height="10" fill="#fff"/></mask></defs>
            <g mask="url(#m)"><rect width="10" height="10" fill="#00f"/></g></svg>"##,
        );
        assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill);
        let (_, _, inside) = layer(&drawn[0]);
        let (mask, _, coverage) = layer(&inside[1]);
        assert_eq!(mask.blend, Some(BlendMode::DstIn));
        // The coverage, then its luminance kept of it.
        assert_eq!(coverage.len(), 2);
        let (luminance, _, _) = layer(&coverage[1]);
        assert_eq!(luminance.blend, Some(BlendMode::DstIn));
        let Some(ColorFilter::Matrix(m)) = luminance.color else {
            panic!("{luminance:?}")
        };
        // Alpha from the colour alone: white keeps everything, black nothing.
        assert!((m[15] + m[16] + m[17] - 1.0).abs() < 1e-4 && m[18] == 0.0);
    }

    #[test]
    fn an_empty_mask_hides_its_group_and_a_tint_spares_the_mask() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><mask id="empty"/><mask id="m"><rect width="10" height="10" fill="#fff"/></mask></defs>
            <rect width="10" height="10" fill="#f00" mask="url(#empty)"/>
            <rect width="10" height="10" fill="#f00" mask="url(#m)"/></svg>"##,
        );
        let mut scene = Scene::new();
        scene.set_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        svg.paint(
            &mut scene,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            BoxFit::Fill,
            AlignmentGeometry::Physical(frus_core::Alignment::CENTER),
            Some(Color::rgb(0.0, 1.0, 0.0)),
            1.0,
        );
        // Only the second square.
        assert_eq!(scene.primitives().len(), 1);
        let (_, _, inside) = layer(&scene.primitives()[0]);
        assert!(matches!(&inside[0], Primitive::Path { fill: Some(c), .. } if c.g == 1.0));
        // The mask's white stays white: a tint recolouring it would change what it keeps.
        let (_, _, coverage) = layer(&inside[1]);
        let (_, _, content) = layer(&coverage[0]);
        assert!(matches!(&content[0], Primitive::Path { fill: Some(c), .. }
            if c.r == 1.0 && c.g == 1.0 && c.b == 1.0));
    }

    #[test]
    fn a_gaussian_blur_is_a_blurred_layer_clipped_to_the_filter_region() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><filter id="f"><feGaussianBlur stdDeviation="0.5 1"/></filter></defs>
            <g transform="scale(2 1)"><rect x="1" y="2" width="2" height="5" filter="url(#f)"/></g></svg>"##,
        );
        assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
        // The picture shown ten times its size.
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 100.0, 100.0), BoxFit::Fill);
        let (region, shape, inside) = layer(&drawn[0]);
        assert!(region.is_none());
        let ClipShape::Path(region) = shape else {
            panic!("{shape:?}")
        };
        // The default region: the box grown by a tenth on each side — x 0.8..3.2 in the
        // group, 16..64 on the screen; y 1.5..7.5, 15..75.
        let first = match region.verbs()[0] {
            frus_core::PathVerb::MoveTo(p) => p,
            ref v => panic!("{v:?}"),
        };
        assert!(
            (first.x - 16.0).abs() < 1e-3 && (first.y - 15.0).abs() < 1e-3,
            "{first:?}"
        );
        let (blur, _, content) = layer(&inside[0]);
        // 0.5 × 2 (the group) × 10 (the screen) across, 1 × 10 down.
        assert_eq!(
            blur.image,
            Some(ImageFilter::Blur {
                sigma_x: 10.0,
                sigma_y: 10.0
            })
        );
        assert!(matches!(&content[0], Primitive::Path { .. }));
    }

    #[test]
    fn a_drop_shadow_is_the_silhouette_coloured_blurred_and_moved_under_the_source() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><filter id="f"><feDropShadow dx="1" dy="2" stdDeviation="0.5"
              flood-color="#00f" flood-opacity="0.5"/></filter></defs>
            <rect x="2" y="2" width="4" height="4" fill="#f00" filter="url(#f)"/></svg>"##,
        );
        assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill);
        let (_, _, inside) = layer(&drawn[0]);
        assert_eq!(inside.len(), 2, "{inside:?}");
        let (shadow, _, moved) = layer(&inside[0]);
        assert_eq!(shadow.image, Some(ImageFilter::blur(0.5)));
        let Some(ColorFilter::Mode(c, BlendMode::SrcIn)) = shadow.color else {
            panic!("{shadow:?}")
        };
        assert!(c.b == 1.0 && (c.a - 0.5).abs() < 0.01);
        let p = first_point(&moved[0]);
        assert!(
            (p.x - 3.0).abs() < 1e-3 && (p.y - 4.0).abs() < 1e-3,
            "{p:?}"
        );
        // The source over it, where it was.
        let p = first_point(&inside[1]);
        assert!(
            (p.x - 2.0).abs() < 1e-3 && (p.y - 2.0).abs() < 1e-3,
            "{p:?}"
        );
    }

    #[test]
    fn a_graph_of_offset_flood_composite_and_merge_is_drawn() {
        // A design tool's drop shadow, spelled out.
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><filter id="f" x="0" y="0" width="10" height="10" filterUnits="userSpaceOnUse">
              <feFlood flood-opacity="0" result="BackgroundImageFix"/>
              <feColorMatrix in="SourceAlpha" type="matrix"
                values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/>
              <feOffset dy="1"/>
              <feGaussianBlur stdDeviation="0.5"/>
              <feComposite in2="hardAlpha" operator="out"/>
              <feColorMatrix type="matrix" values="0 0 0 0 0.2 0 0 0 0 0.2 0 0 0 0 0.2 0 0 0 0.25 0"/>
              <feBlend mode="normal" in2="BackgroundImageFix" result="shadow"/>
              <feMerge><feMergeNode in="shadow"/><feMergeNode in="SourceGraphic"/></feMerge>
            </filter></defs>
            <rect x="2" y="2" width="4" height="4" fill="#f00" filter="url(#f)"/></svg>"##,
        );
        assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill);
        let (_, _, inside) = layer(&drawn[0]);
        // The transparent flood, the tinted shadow, the source.
        assert_eq!(inside.len(), 3, "{inside:?}");
        let (tint, _, composite) = layer(&inside[1]);
        let Some(ColorFilter::Matrix(m)) = tint.color else {
            panic!("{tint:?}")
        };
        // The constant 0.2, given in linear light, re-encoded as sRGB.
        assert!((m[4] - 0.4845).abs() < 1e-3, "{}", m[4]);
        let (_, _, composite) = layer(&composite[0]);
        let (out, _, _) = layer(&composite[1]);
        assert_eq!(out.blend, Some(BlendMode::SrcOut));
    }

    #[test]
    fn a_colour_matrix_mixing_channels_in_linear_light_is_not_drawn() {
        let mixing = |space: &str| {
            parse(&format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
                <defs><filter id="f" color-interpolation-filters="{space}">
                  <feColorMatrix type="saturate" values="0"/></filter></defs>
                <rect width="5" height="5" fill="#f00" filter="url(#f)"/></svg>"##
            ))
        };
        assert_eq!(mixing("linearRGB").unsupported(), &["filter"]);
        let svg = mixing("sRGB");
        assert!(svg.unsupported().is_empty());
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill);
        let (_, _, inside) = layer(&drawn[0]);
        let (grey, _, _) = layer(&inside[0]);
        assert!(matches!(grey.color, Some(ColorFilter::Matrix(_))));
    }

    #[test]
    fn a_blur_unlike_along_its_axes_on_a_turned_group_is_not_drawn() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <defs><filter id="f"><feGaussianBlur stdDeviation="1 0"/></filter></defs>
            <rect width="5" height="5" transform="rotate(30)" filter="url(#f)"/></svg>"##,
        );
        assert_eq!(svg.unsupported(), &["filter"]);
    }

    #[test]
    fn a_cropped_picture_stays_in_its_box() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10">
            <rect width="20" height="10"/></svg>"##,
        );
        let mut scene = Scene::new();
        scene.set_clip(Rect::new(0.0, 0.0, 500.0, 500.0));
        svg.paint(
            &mut scene,
            Rect::new(0.0, 0.0, 50.0, 50.0),
            BoxFit::Cover,
            AlignmentGeometry::Physical(frus_core::Alignment::CENTER),
            None,
            1.0,
        );
        let Primitive::Path { clip, .. } = &scene.primitives()[0] else {
            panic!()
        };
        assert_eq!(*clip, Rect::new(0.0, 0.0, 50.0, 50.0));
        assert_eq!(
            scene.current_clip(),
            Rect::new(0.0, 0.0, 500.0, 500.0),
            "restored"
        );
    }

    /// A 2×1 PNG: a red pixel, then a blue one.
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP4z8AAQv8BD/kD/YURmXYAAAAASUVORK5CYII=";

    #[test]
    fn an_embedded_png_is_decoded_once_and_drawn_where_the_picture_puts_it() {
        let svg = parse(&format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <image x="2" y="3" width="4" height="2" href="data:image/png;base64,{PNG}"/></svg>"##
        ));
        assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 100.0, 100.0), BoxFit::Fill);
        let Primitive::Image {
            image, rect, tint, ..
        } = &drawn[0]
        else {
            panic!("{drawn:?}")
        };
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(&image.rgba()[..4], &[255, 0, 0, 255]);
        let near = |a: Rect, b: Rect| {
            (a.x - b.x).abs() < 1e-3
                && (a.y - b.y).abs() < 1e-3
                && (a.width - b.width).abs() < 1e-3
                && (a.height - b.height).abs() < 1e-3
        };
        assert!(near(*rect, Rect::new(20.0, 30.0, 40.0, 20.0)), "{rect:?}");
        assert_eq!(*tint, Color::WHITE);
        // Painted again, the same pixels: decoded once, when parsed.
        let again = painted(&svg, Rect::new(0.0, 0.0, 50.0, 50.0), BoxFit::Fill);
        let Primitive::Image { image: second, .. } = &again[0] else {
            panic!()
        };
        assert_eq!(second.id(), image.id());
    }

    #[test]
    fn a_turned_bitmap_is_turned_by_its_layer() {
        let svg = parse(&format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <image width="4" height="2" transform="rotate(30 5 5)"
                   href="data:image/png;base64,{PNG}"/></svg>"##
        ));
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 100.0, 100.0), BoxFit::Fill);
        let Primitive::Layer {
            transform: Some(_),
            primitives,
            ..
        } = &drawn[0]
        else {
            panic!("{drawn:?}")
        };
        assert!(matches!(&primitives[0], Primitive::Image { rect, .. }
            if (rect.width - 40.0).abs() < 1e-3 && (rect.height - 20.0).abs() < 1e-3));
    }

    #[test]
    fn an_embedded_svg_is_drawn_as_vectors_in_its_place() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10">
            <image x="10" width="10" height="10"
                   href="data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxMCIgaGVpZ2h0PSIxMCIgdmlld0JveD0iMCAwIDEwIDEwIj48cmVjdCB4PSIwIiB5PSIwIiB3aWR0aD0iNSIgaGVpZ2h0PSIxMCIgZmlsbD0iI2YwMCIvPjwvc3ZnPg=="/></svg>"##,
        );
        assert_eq!(svg.shape_count(), 1);
        let drawn = painted(&svg, Rect::new(0.0, 0.0, 200.0, 100.0), BoxFit::Fill);
        let p = first_point(&drawn[0]);
        assert!((p.x - 100.0).abs() < 1e-3 && p.y.abs() < 1e-3, "{p:?}");
    }

    #[test]
    fn a_picture_does_not_open_the_files_it_names() {
        let file = std::env::temp_dir().join("frus-svg-embedded.png");
        std::fs::write(&file, base64_decode(PNG)).unwrap();
        let svg = parse(&format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <image width="4" height="2" href="{}"/></svg>"##,
            file.display()
        ));
        let _ = std::fs::remove_file(&file);
        assert!(painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill).is_empty());
    }

    #[test]
    fn a_gif_is_named_and_not_drawn() {
        let svg = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
            <image width="4" height="4"
                   href="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"/></svg>"##,
        );
        assert_eq!(svg.unsupported(), &["image"]);
        assert!(painted(&svg, Rect::new(0.0, 0.0, 10.0, 10.0), BoxFit::Fill).is_empty());
    }

    /// Standard base64, for writing the test PNG out as a file.
    fn base64_decode(text: &str) -> Vec<u8> {
        let value = |c: u8| match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            _ => 63,
        };
        let digits: Vec<u8> = text.bytes().filter(|&c| c != b'=').map(value).collect();
        let mut out = Vec::new();
        for chunk in digits.chunks(4) {
            let mut n = 0u32;
            for (i, d) in chunk.iter().enumerate() {
                n |= u32::from(*d) << (18 - 6 * i);
            }
            out.extend(&n.to_be_bytes()[1..chunk.len()]);
        }
        out
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(Svg::parse(b"not svg").is_err());
    }
}
