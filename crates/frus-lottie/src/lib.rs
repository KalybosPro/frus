//! `frus-lottie` — **Lottie animations drawn as frus's own vector paths**.
//!
//! A Lottie file (Bodymovin JSON) is parsed once ([`Animation::parse`]); any moment of it is then
//! painted into a [`Scene`] at any size ([`Animation::paint`]). Nothing is rasterised: every
//! shape is a [`Path`] the renderer tessellates at the size shown.
//!
//! [velato] reads the file and evaluates a frame — the layers, their transforms, the shapes and
//! their animated properties — and draws it through a trait this crate implements over the
//! scene: fills, strokes (expanded into outlines: widths, caps, joins, dashes), flat colours and
//! linear and radial gradients, layer opacity, clips and masks, and the blend modes of mattes,
//! and image layers: a PNG or JPEG embedded in the file is decoded once, when it is parsed, and
//! one the file names beside it is the application's to give ([`Animation::with_image`]).
//! Sweep gradients are not drawn yet; [`Animation::unsupported`] names what an animation uses
//! that is not.

#![deny(missing_docs)]

mod normalize;

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use frus_core::{
    Affine, AlignmentGeometry, BlendMode, BoxFit, ClipShape, Color, FillRule, GradientRamp,
    ImageHandle, LayerFilter, Path, Point, Primitive, RampShape, Rect, Scene, Size, TextDirection,
    TileMode,
};
use kurbo::Shape;
use velato::{model::fixed, Composition, RenderSink, Renderer};

/// A parsing failure: the bytes are not a Lottie animation this parser can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LottieError(String);

impl LottieError {
    /// The underlying message.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LottieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lottie parsing failed: {}", self.0)
    }
}

impl std::error::Error for LottieError {}

/// **A parsed Lottie animation**, ready to paint at any moment and any size.
pub struct Animation {
    composition: Composition,
    /// The image assets' pixels, by asset id: decoded from the file, or given.
    images: HashMap<String, ImageHandle>,
    /// velato's working buffers, kept between frames. Behind a lock because painting takes the
    /// animation by shared reference, and the widget layer shares it between threads.
    renderer: Mutex<Renderer>,
    /// What the frames painted so far used that is not drawn.
    unsupported: Mutex<Vec<&'static str>>,
}

/// A parsed animation behind a shared handle, as the widget layer caches it.
pub type AnimationHandle = Arc<Animation>;

impl fmt::Debug for Animation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Animation")
            .field("size", &self.size())
            .field("duration", &self.duration())
            .finish_non_exhaustive()
    }
}

impl Animation {
    /// Parses a Lottie file's contents (JSON, UTF-8).
    ///
    /// Files written by older exporters are brought to the current form first, and what the
    /// renderer cannot draw — text layers, a few blends — is left out and named by
    /// [`Animation::unsupported`] rather than refusing the file; the rest is drawn.
    pub fn parse(bytes: &[u8]) -> Result<Animation, LottieError> {
        let mut value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| LottieError(e.to_string()))?;
        let mut notes = Vec::new();
        normalize::normalize(&mut value, &mut notes).map_err(LottieError)?;
        let composition = Composition::from_json(value).map_err(|e| LottieError(e.to_string()))?;
        let mut images = HashMap::new();
        for (id, asset) in &composition.images {
            if !asset.is_data_url() {
                continue;
            }
            match data_url(&asset.file_name).map(|bytes| frus_image::decode(&bytes)) {
                Some(Ok(image)) => {
                    images.insert(id.clone(), image.into_handle());
                }
                _ => notes.push("image"),
            }
        }
        Ok(Animation {
            composition,
            images,
            renderer: Mutex::new(Renderer::new()),
            unsupported: Mutex::new(notes),
        })
    }

    /// **Gives the pixels of an image asset** the file names rather than embeds — `id` is
    /// the asset's, as [`Animation::missing_images`] lists it. An image given for an
    /// embedded asset replaces it.
    ///
    /// ```
    /// # let json = br#"{"v":"5.7.0","fr":30,"ip":0,"op":30,"w":10,"h":10,"layers":[],
    /// #   "assets":[{"id":"image_0","w":10,"h":10,"u":"images/","p":"img_0.png","e":0}]}"#;
    /// let animation = frus_lottie::Animation::parse(json).unwrap();
    /// let wanted = ("image_0".to_string(), "images/img_0.png".to_string());
    /// assert_eq!(animation.missing_images(), [wanted]);
    /// let pixels = frus_core::ImageData::from_rgba(1, 1, vec![255; 4]).into_handle();
    /// let animation = animation.with_image("image_0", pixels);
    /// assert!(animation.missing_images().is_empty());
    /// ```
    pub fn with_image(mut self, id: impl Into<String>, image: ImageHandle) -> Self {
        self.images.insert(id.into(), image);
        self
    }

    /// The image assets without pixels yet, by id and the location the file gives
    /// (its folder and file name), in id order: an application loads each and hands it to
    /// [`Animation::with_image`]. Until then, the layers showing it draw nothing.
    pub fn missing_images(&self) -> Vec<(String, String)> {
        let mut missing: Vec<_> = self
            .composition
            .images
            .iter()
            .filter(|(id, _)| !self.images.contains_key(*id))
            .map(|(id, asset)| (id.clone(), asset.location()))
            .collect();
        missing.sort();
        missing
    }

    /// The animation's own size, in its own units.
    pub fn size(&self) -> Size {
        Size::new(
            self.composition.width as f32,
            self.composition.height as f32,
        )
    }

    /// How long one play of the animation lasts, in seconds.
    pub fn duration(&self) -> f32 {
        let frames = &self.composition.frames;
        let rate = self.composition.frame_rate.max(1.0);
        ((frames.end - frames.start) / rate).max(0.0) as f32
    }

    /// The animation's frame rate, frames per second.
    pub fn frame_rate(&self) -> f32 {
        self.composition.frame_rate as f32
    }

    /// What the frames painted so far used that is not drawn yet, by name (`"image"` — an
    /// asset without pixels, or one that could not be decoded —, `"sweep gradient"`): empty
    /// when everything was drawn.
    pub fn unsupported(&self) -> Vec<&'static str> {
        self.unsupported
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The frame shown at `progress`, from `0` (the first) to `1` (the last).
    fn frame_at(&self, progress: f32) -> f64 {
        let frames = &self.composition.frames;
        let last = (frames.end - 1.0).max(frames.start);
        frames.start + (last - frames.start) * f64::from(progress.clamp(0.0, 1.0))
    }

    /// **Paints the animation at `progress`** (`0` the first frame, `1` the last) into
    /// `bounds`, fitted by `fit` and placed by `alignment` as an image is, at `opacity`.
    pub fn paint(
        &self,
        scene: &mut Scene,
        bounds: Rect,
        fit: BoxFit,
        alignment: AlignmentGeometry,
        progress: f32,
        opacity: f32,
    ) {
        let size = self.size();
        if size.width <= 0.0 || size.height <= 0.0 || opacity <= 0.0 {
            return;
        }
        let (dst, uv) = fit.apply_aligned(size, bounds, alignment.resolve(TextDirection::Ltr));
        if dst.width <= 0.0 || dst.height <= 0.0 || uv.width <= 0.0 || uv.height <= 0.0 {
            return;
        }
        let (w, h) = (size.width, size.height);
        let place = Affine::translation(-uv.x * w, -uv.y * h)
            .then(Affine::scale(
                dst.width / (uv.width * w),
                dst.height / (uv.height * h),
            ))
            .then(Affine::translation(dst.x, dst.y));
        let outer = scene.current_clip();
        scene.set_clip(intersect(outer, dst));
        let start = scene.primitives().len();
        {
            let mut sink = Sink {
                scene: &mut *scene,
                images: &self.images,
                place,
                layers: Vec::new(),
                unsupported: Vec::new(),
            };
            self.renderer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .append(
                    &self.composition,
                    self.frame_at(progress),
                    kurbo::Affine::IDENTITY,
                    1.0,
                    &mut sink,
                );
            // A layer velato left open (it never does) is closed rather than lost.
            while !sink.layers.is_empty() {
                sink.pop_layer();
            }
            let mut known = self.unsupported.lock().unwrap_or_else(|e| e.into_inner());
            for what in sink.unsupported {
                if !known.contains(&what) {
                    known.push(what);
                }
            }
        }
        if opacity < 1.0 {
            wrap(scene, start, opacity, ClipShape::Rect, None);
        }
        scene.set_clip(outer);
    }
}

/// The bytes of a base64 `data:` address; `None` for any other.
fn data_url(location: &str) -> Option<Vec<u8>> {
    let (header, payload) = location.strip_prefix("data:")?.split_once(',')?;
    if !header.ends_with(";base64") {
        return None;
    }
    let mut out = Vec::with_capacity(payload.len() / 4 * 3);
    let (mut bits, mut count) = (0u32, 0u32);
    for c in payload.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            c if c.is_ascii_whitespace() => continue,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    Some(out)
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// The primitives painted since `start`, moved into one layer.
fn wrap(
    scene: &mut Scene,
    start: usize,
    opacity: f32,
    clip_shape: ClipShape,
    blend: Option<BlendMode>,
) {
    let primitives = scene.split_off(start);
    if primitives.is_empty() {
        return;
    }
    scene.push_primitive(Primitive::Layer {
        primitives,
        opacity,
        clip: scene.current_clip(),
        clip_shape,
        transform: None,
        filter: LayerFilter {
            blend,
            ..LayerFilter::NONE
        },
        owner: scene.current_owner(),
    });
}

/// A layer velato opened.
enum OpenLayer {
    /// Composited as one: where its primitives start, and how it meets what is underneath.
    Composited {
        start: usize,
        opacity: f32,
        clip: Path,
        blend: Option<BlendMode>,
    },
    /// A clip to a rectangle on the screen's axes — the animation's own frame, on every frame
    /// — which the scene's clip rectangle does without a layer: the clip in force before it.
    Rect { previous: Rect },
}

/// velato's drawing, into a frus scene, every coordinate taken through `place`.
struct Sink<'a> {
    scene: &'a mut Scene,
    images: &'a HashMap<String, ImageHandle>,
    place: Affine,
    layers: Vec<OpenLayer>,
    unsupported: Vec<&'static str>,
}

impl Sink<'_> {
    /// `transform` (velato's, to the animation's space) then `place` (to the screen).
    fn to_screen(&self, transform: kurbo::Affine) -> Affine {
        let c = transform.as_coeffs();
        Affine {
            m: [
                c[0] as f32,
                c[1] as f32,
                c[2] as f32,
                c[3] as f32,
                c[4] as f32,
                c[5] as f32,
            ],
        }
        .then(self.place)
    }

    fn note(&mut self, what: &'static str) {
        if !self.unsupported.contains(&what) {
            self.unsupported.push(what);
        }
    }
}

/// How finely curves are followed when an outline is computed, in screen pixels.
const TOLERANCE_PX: f64 = 0.1;

/// The tolerance in a shape's own units, for a shape shown through `to_screen`.
fn tolerance(to_screen: Affine) -> f64 {
    let [a, b, c, d, _, _] = to_screen.m;
    let scale = f64::from((a * d - b * c).abs().sqrt());
    if scale > 1e-6 {
        TOLERANCE_PX / scale
    } else {
        TOLERANCE_PX
    }
}

/// A kurbo path as a frus one, through `map`.
fn converted(elements: impl IntoIterator<Item = kurbo::PathEl>, map: Affine) -> Path {
    let p = |p: kurbo::Point| map.apply(Point::new(p.x as f32, p.y as f32));
    let mut out = Path::new().with_fill_rule(FillRule::NonZero);
    for element in elements {
        out = match element {
            kurbo::PathEl::MoveTo(a) => out.move_to(p(a)),
            kurbo::PathEl::LineTo(a) => out.line_to(p(a)),
            kurbo::PathEl::QuadTo(c, a) => out.quad_to(p(c), p(a)),
            kurbo::PathEl::CurveTo(c1, c2, a) => out.cubic_to(p(c1), p(c2), p(a)),
            kurbo::PathEl::ClosePath => out.close(),
        };
    }
    out
}

fn color(c: peniko::color::AlphaColor<peniko::color::Srgb>) -> Color {
    let [r, g, b, a] = c.components;
    Color::rgba(r, g, b, a)
}

/// A layer's blend as frus's: `None` for plain painting over.
fn blend(mode: peniko::BlendMode) -> Option<BlendMode> {
    use peniko::{Compose, Mix};
    let composed = match mode.compose {
        Compose::SrcOver => None,
        Compose::Clear => Some(BlendMode::Clear),
        Compose::Copy => Some(BlendMode::Src),
        Compose::Dest => Some(BlendMode::Dst),
        Compose::DestOver => Some(BlendMode::DstOver),
        Compose::SrcIn => Some(BlendMode::SrcIn),
        Compose::DestIn => Some(BlendMode::DstIn),
        Compose::SrcOut => Some(BlendMode::SrcOut),
        Compose::DestOut => Some(BlendMode::DstOut),
        Compose::SrcAtop => Some(BlendMode::SrcAtop),
        Compose::DestAtop => Some(BlendMode::DstAtop),
        Compose::Xor => Some(BlendMode::Xor),
        _ => Some(BlendMode::Plus),
    };
    if composed.is_some() {
        return composed;
    }
    match mode.mix {
        Mix::Normal => None,
        Mix::Multiply => Some(BlendMode::Multiply),
        Mix::Screen => Some(BlendMode::Screen),
        Mix::Overlay => Some(BlendMode::Overlay),
        Mix::Darken => Some(BlendMode::Darken),
        Mix::Lighten => Some(BlendMode::Lighten),
        Mix::ColorDodge => Some(BlendMode::ColorDodge),
        Mix::ColorBurn => Some(BlendMode::ColorBurn),
        Mix::HardLight => Some(BlendMode::HardLight),
        Mix::SoftLight => Some(BlendMode::SoftLight),
        Mix::Difference => Some(BlendMode::Difference),
        Mix::Exclusion => Some(BlendMode::Exclusion),
        Mix::Hue => Some(BlendMode::Hue),
        Mix::Saturation => Some(BlendMode::Saturation),
        Mix::Color => Some(BlendMode::Color),
        Mix::Luminosity => Some(BlendMode::Luminosity),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

impl RenderSink for Sink<'_> {
    fn push_layer(
        &mut self,
        mode: impl Into<peniko::BlendMode>,
        alpha: f32,
        transform: kurbo::Affine,
        shape: &impl Shape,
    ) {
        let to_screen = self.to_screen(transform);
        let clip = converted(shape.path_elements(tolerance(to_screen)), to_screen);
        self.layers.push(OpenLayer::Composited {
            start: self.scene.primitives().len(),
            opacity: alpha,
            clip,
            blend: blend(mode.into()),
        });
    }

    fn push_clip_layer(&mut self, transform: kurbo::Affine, shape: &impl Shape) {
        let to_screen = self.to_screen(transform);
        if let Some(rect) = shape.as_rect().filter(|_| to_screen.is_axis_aligned()) {
            let on_screen = to_screen.apply_rect(Rect::new(
                rect.x0 as f32,
                rect.y0 as f32,
                rect.width() as f32,
                rect.height() as f32,
            ));
            let previous = self.scene.current_clip();
            self.scene.set_clip(intersect(previous, on_screen));
            self.layers.push(OpenLayer::Rect { previous });
            return;
        }
        let clip = converted(shape.path_elements(tolerance(to_screen)), to_screen);
        self.layers.push(OpenLayer::Composited {
            start: self.scene.primitives().len(),
            opacity: 1.0,
            clip,
            blend: None,
        });
    }

    fn pop_layer(&mut self) {
        match self.layers.pop() {
            Some(OpenLayer::Composited {
                start,
                opacity,
                clip,
                blend,
            }) => wrap(self.scene, start, opacity, ClipShape::Path(clip), blend),
            Some(OpenLayer::Rect { previous }) => self.scene.set_clip(previous),
            None => {}
        }
    }

    fn draw(
        &mut self,
        stroke: Option<&fixed::Stroke>,
        transform: kurbo::Affine,
        brush: &fixed::Brush,
        shape: &impl Shape,
    ) {
        let to_screen = self.to_screen(transform);
        let tolerance = tolerance(to_screen);
        // A stroke is the outline it covers, computed in the shape's own space (as the
        // reference renderer strokes, before the transform) and filled non-zero.
        let path = match stroke {
            Some(stroke) => converted(
                kurbo::stroke(
                    shape.path_elements(tolerance),
                    stroke,
                    &kurbo::StrokeOpts::default(),
                    tolerance,
                ),
                to_screen,
            ),
            None => converted(shape.path_elements(tolerance), to_screen),
        };
        match brush {
            peniko::Brush::Solid(c) => self.scene.fill_path(&path, color(*c)),
            peniko::Brush::Gradient(gradient) => {
                let stops = gradient
                    .stops
                    .iter()
                    .map(|stop| {
                        let c = stop.color.to_alpha_color::<peniko::color::Srgb>();
                        (stop.offset, color(c))
                    })
                    .collect();
                let tile = match gradient.extend {
                    peniko::Extend::Pad => TileMode::Clamp,
                    peniko::Extend::Repeat => TileMode::Repeated,
                    peniko::Extend::Reflect => TileMode::Mirror,
                };
                let point = |p: kurbo::Point| Point::new(p.x as f32, p.y as f32);
                let shape = match gradient.kind {
                    peniko::GradientKind::Linear(line) => RampShape::Linear {
                        from: point(line.start),
                        to: point(line.end),
                    },
                    peniko::GradientKind::Radial(radial) => RampShape::Radial {
                        center: point(radial.end_center),
                        radius: radial.end_radius,
                        focal: point(radial.start_center),
                        focal_radius: radial.start_radius,
                    },
                    peniko::GradientKind::Sweep(_) => {
                        self.note("sweep gradient");
                        let first = gradient.stops.first().map_or(Color::TRANSPARENT, |s| {
                            color(s.color.to_alpha_color::<peniko::color::Srgb>())
                        });
                        self.scene.fill_path(&path, first);
                        return;
                    }
                };
                self.scene.fill_path_ramp(
                    &path,
                    GradientRamp {
                        shape,
                        stops,
                        tile,
                        transform: to_screen,
                    },
                    1.0,
                );
            }
            peniko::Brush::Image(_) => self.note("image"),
        }
    }

    fn draw_image(
        &mut self,
        asset: &velato::model::ImageAsset,
        mut transform: kurbo::Affine,
        alpha: f64,
    ) {
        let Some(image) = self.images.get(&asset.id) else {
            self.note("image");
            return;
        };
        // Pixels of another size than the asset's are fitted into it, whole and centred.
        let (width, height) = (f64::from(image.width()), f64::from(image.height()));
        if let Some((w, h)) = asset.width.zip(asset.height) {
            if w > 0.0 && h > 0.0 && width > 0.0 && height > 0.0 {
                let scale = (w / width).min(h / height);
                let offset = ((w - width * scale) / 2.0, (h - height * scale) / 2.0);
                transform *= kurbo::Affine::translate(offset) * kurbo::Affine::scale(scale);
            }
        }
        let alpha = if alpha.is_finite() {
            alpha.clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        if alpha > 0.0 {
            let to_screen = self.to_screen(transform);
            self.scene
                .draw_image_mapped(image, to_screen, Color::rgba(1.0, 1.0, 1.0, alpha));
        }
    }
}
