//! The [`Scene`]: a pure display list, independent of any rendering backend.
//!
//! It describes *what to draw* — primitives — while knowing nothing about the
//! GPU. `frus-gpu` consumes it to produce GPU commands; `frus-widgets` produces
//! it from a widget tree.
//!
//! Every primitive carries a **clip rectangle**, set through
//! [`Scene::set_clip`] before primitives are added.

use crate::{
    Affine, BorderRadius, Color, FontWeight, ImageHandle, Path, PathVerb, Point, Rect, Size,
    Stroke, TextDecoration, TextRun, TextStyle,
};

/// The transform applied to a **layer** ([`Primitive::Layer`]) at compositing time:
/// an arbitrary [`Affine`] (translation, per-axis scale, rotation, or any
/// composition of them) in screen pixels. The layer is first rendered **flat** into
/// a texture, as with group opacity, then composited **transformed** — so a single
/// pass transforms a whole subtree (rects, text, images) without touching any
/// individual primitive's shaders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerTransform {
    /// The local (flat) → screen transform.
    pub affine: Affine,
}

impl LayerTransform {
    /// Wraps any [`Affine`].
    pub const fn new(affine: Affine) -> Self {
        Self { affine }
    }

    /// A rotation of `angle` radians about `pivot`, in screen pixels.
    pub fn rotation(angle: f32, pivot: Point) -> Self {
        Self {
            affine: Affine::rotation(angle).about(pivot),
        }
    }

    /// Conjugates the transform by a scale, so it follows `Primitive::scaled`:
    /// `S ∘ M ∘ S⁻¹`, which is what makes a DPI change scale the whole layer, its
    /// transform included.
    pub fn scaled(self, factor: f32) -> Self {
        self.scaled_xy(factor, factor)
    }

    /// Like [`LayerTransform::scaled`], but per axis.
    pub fn scaled_xy(self, sx: f32, sy: f32) -> Self {
        let conj = Affine::scale(1.0 / sx, 1.0 / sy)
            .then(self.affine)
            .then(Affine::scale(sx, sy));
        Self { affine: conj }
    }

    /// Conjugates by a translation, so it follows `Primitive::translated`.
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        let conj = Affine::translation(-dx, -dy)
            .then(self.affine)
            .then(Affine::translation(dx, dy));
        Self { affine: conj }
    }
}

/// How a path's fill **fades**: along a line, or outwards from a centre.
///
/// The geometry is in the same space as the path, deliberately, so a fade can be
/// aimed at something real — the edge a glow springs from, the ellipse whose cap is
/// the glow — rather than at whatever bounding box the geometry happens to have.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathGradient {
    /// The fill colour at `from`, `to_color` at `to`, clamped outside that span.
    Linear {
        /// The colour at `to`. The colour at `from` is the primitive's `fill`.
        to_color: Color,
        /// Where the fill colour holds.
        from: Point,
        /// Where `to_color` holds.
        to: Point,
    },
    /// The fill colour on the ellipse of radii `inner × radii` about `center`,
    /// `to_color` on the ellipse of radii `radii`, clamped outside.
    ///
    /// A curved edge needs this. A linear fade can only reach zero along a *line*,
    /// so a shape whose boundary curves away from that line stops while the fade is
    /// unfinished, and the leftover shows as an edge — which is the very thing a
    /// glow must not have (milestone 302).
    Radial {
        /// The colour on the outer ellipse.
        to_color: Color,
        /// The centre of both ellipses.
        center: Point,
        /// The outer ellipse's radii, where `to_color` holds.
        radii: Size,
        /// Where the fade starts, as a fraction of `radii`. `0` fades from the
        /// centre outwards; a value near `1` confines it to a thin rind.
        inner: f32,
    },
}

impl PathGradient {
    /// Scales the geometry per axis, as a DPI or paint scale does.
    pub fn scaled_xy(self, sx: f32, sy: f32) -> Self {
        match self {
            Self::Linear { to_color, from, to } => Self::Linear {
                to_color,
                from: Point::new(from.x * sx, from.y * sy),
                to: Point::new(to.x * sx, to.y * sy),
            },
            Self::Radial {
                to_color,
                center,
                radii,
                inner,
            } => Self::Radial {
                to_color,
                center: Point::new(center.x * sx, center.y * sy),
                radii: Size::new(radii.width * sx, radii.height * sy),
                inner,
            },
        }
    }

    /// Moves the geometry with the shape it describes.
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        match self {
            Self::Linear { to_color, from, to } => Self::Linear {
                to_color,
                from: Point::new(from.x + dx, from.y + dy),
                to: Point::new(to.x + dx, to.y + dy),
            },
            Self::Radial {
                to_color,
                center,
                radii,
                inner,
            } => Self::Radial {
                to_color,
                center: Point::new(center.x + dx, center.y + dy),
                radii,
                inner,
            },
        }
    }

    /// The colour the fade ends on.
    pub fn to_color(self) -> Color {
        match self {
            Self::Linear { to_color, .. } | Self::Radial { to_color, .. } => to_color,
        }
    }

    /// Fades the end colour with the fill it belongs to.
    pub fn faded(self, opacity: f32) -> Self {
        match self {
            Self::Linear { to_color, from, to } => Self::Linear {
                to_color: to_color.fade(opacity),
                from,
                to,
            },
            Self::Radial {
                to_color,
                center,
                radii,
                inner,
            } => Self::Radial {
                to_color: to_color.fade(opacity),
                center,
                radii,
                inner,
            },
        }
    }
}

/// The clip shape of a [`Primitive::Layer`], **inscribed** in its `clip` rectangle.
/// Compositing multiplies the layer's alpha by the shape's coverage, with
/// antialiased edges. This is the building block of the `ClipRRect`, `ClipOval`
/// and `ClipPath` widgets.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ClipShape {
    /// A crisp rectangular clip — the layer's `clip`, unchanged.
    #[default]
    Rect,
    /// A **rounded-corner** rectangle, with a **per-corner** radius
    /// ([`BorderRadius`], logical px) clamped to half the `clip`'s smaller side. A
    /// uniform radius is still `BorderRadius::uniform(r)`.
    RRect(BorderRadius),
    /// An **ellipse** inscribed in the `clip` — a circle when the `clip` is square.
    Oval,
    /// An **arbitrary path**, in absolute screen coordinates: compositing renders it
    /// into a coverage **mask** which it multiplies into the layer's alpha. The
    /// building block of `ClipPath` — stars, notches, free-form shapes.
    Path(Path),
}

impl ClipShape {
    /// Follows a **per-axis** scaling of the layer (DPI, paint scale): only a
    /// rounded radius changes, by the average of the two factors, since it has no
    /// axis. The ellipse follows its `clip`, and the rectangle has nothing to scale.
    pub fn scaled_xy(self, sx: f32, sy: f32) -> ClipShape {
        match self {
            ClipShape::RRect(br) => ClipShape::RRect(br.scale((sx + sy) * 0.5)),
            // The path is in absolute coordinates, so DPI scaling follows it
            // (uniformly — at DPI, `sx == sy`).
            ClipShape::Path(p) => ClipShape::Path(p.scaled((sx + sy) * 0.5)),
            other => other,
        }
    }
}

/// A drawing primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    /// A rectangle: rounded corners, border, gradient and/or soft shadow.
    Rect {
        rect: Rect,
        /// Fill colour — the start colour when there is a gradient.
        color: Color,
        /// The gradient's end colour (`== color` when flat).
        color2: Color,
        /// Gradient direction in `[0,1]²` space; `(0,0)` means flat.
        gradient_dir: [f32; 2],
        /// Corner radii, per corner.
        radius: BorderRadius,
        border_width: f32,
        border_color: Color,
        /// Edge softening, in pixels (0 = crisp; > 0 = a blurred shadow).
        blur: f32,
        /// Clip rectangle: nothing is drawn outside it.
        clip: Rect,
        /// The emitting widget's identity, used for exit animation.
        owner: u64,
    },
    /// A line of text, anchored by its top-left corner.
    Text {
        position: Point,
        text: String,
        size: f32,
        color: Color,
        /// Font weight.
        weight: FontWeight,
        /// Italic.
        italic: bool,
        /// Wrap width: beyond it the text wraps (`None` = no wrapping, in which
        /// case explicit `\n` characters make the lines).
        max_width: Option<f32>,
        /// Decoration lines (underline, strikethrough, and so on).
        decoration: TextDecoration,
        /// Decoration colour; `None` means the text's own colour.
        decoration_color: Option<Color>,
        /// Clip rectangle.
        clip: Rect,
        /// The box the text was laid out in — the emitting widget's, so an
        /// over-estimate of what the glyphs actually cover, never an under-estimate.
        /// The renderer needs it to know what the text covers; without it text can
        /// only be painted above everything else in the frame. `Rect::UNBOUNDED` when
        /// a scene is built by hand rather than by the widget walk, which the renderer
        /// reads as "unknown, keep it on top".
        bounds: Rect,
        /// The emitting widget's identity.
        owner: u64,
    },
    /// **Rich** text: a sequence of resolved runs, mixing styles and colours, laid
    /// out as one piece, on a single shared baseline.
    RichText {
        position: Point,
        runs: Vec<TextRun>,
        /// Wrap width: beyond it the text wraps (`None` = no wrapping).
        max_width: Option<f32>,
        /// Clip rectangle.
        clip: Rect,
        /// The box the text was laid out in. See [`Primitive::Text`].
        bounds: Rect,
        /// The emitting widget's identity.
        owner: u64,
    },
    /// A **vector path**: arbitrary 2D geometry, filled (`fill`) and/or stroked
    /// (`stroke`). The building block of icons and custom drawing.
    Path {
        path: Path,
        /// Interior fill colour (`None` = no fill), and the gradient's start colour
        /// when there is one.
        fill: Option<Color>,
        /// A gradient across the fill, straight or radial; `None` leaves it flat.
        gradient: Option<PathGradient>,
        /// Outline (colour plus width); `None` = no outline.
        stroke: Option<Stroke>,
        /// Clip rectangle.
        clip: Rect,
        /// The emitting widget's identity.
        owner: u64,
    },
    /// A bitmap **image**, sampled into a destination rectangle.
    Image {
        /// Shared handle to the pixels (GPU-cached by [`crate::ImageData::id`]).
        image: ImageHandle,
        /// Destination rectangle, already fitted per the [`crate::BoxFit`].
        rect: Rect,
        /// The sampled sub-region of the texture, in `0..1` (the `Cover` crop).
        uv: Rect,
        /// Multiplicative tint (white = unchanged; the alpha drives fading).
        tint: Color,
        /// Clip rectangle.
        clip: Rect,
        /// The emitting widget's identity.
        owner: u64,
    },
    /// A **layer**: a subgroup of primitives composited **as one** at `opacity`.
    /// It is rendered separately into a texture at full opacity, then composited,
    /// which is what makes the group alpha correct: no double blending where the
    /// inner primitives overlap.
    Layer {
        /// The group's primitives, in absolute coordinates like the parent scene.
        primitives: Vec<Primitive>,
        /// Group opacity applied to the whole layer (`0..1`).
        opacity: f32,
        /// The layer's clip rectangle.
        clip: Rect,
        /// The clip **shape** inscribed in `clip` (rounded or elliptical); its
        /// coverage modulates alpha at compositing time. [`ClipShape::Rect`] is a
        /// plain rectangular clip, and the default.
        clip_shape: ClipShape,
        /// The affine transform (rotation) applied at compositing. `None` means the
        /// layer is simply composited in place, with group opacity.
        transform: Option<LayerTransform>,
        /// The emitting widget's identity.
        owner: u64,
    },
}

impl Primitive {
    /// The identity of the widget that emitted this primitive.
    pub fn owner(&self) -> u64 {
        match self {
            Primitive::Rect { owner, .. } => *owner,
            Primitive::Text { owner, .. } => *owner,
            Primitive::RichText { owner, .. } => *owner,
            Primitive::Path { owner, .. } => *owner,
            Primitive::Image { owner, .. } => *owner,
            Primitive::Layer { owner, .. } => *owner,
        }
    }

    /// Scales the **geometry** by `factor` — position, size, radius, border, blur,
    /// clip, font size. Colours and text are unchanged. This is how a logical scene
    /// becomes a physical one (DPI).
    pub fn scaled(&self, factor: f32) -> Primitive {
        self.scaled_xy(factor, factor)
    }

    /// Scales the geometry **per axis** (`sx` horizontal, `sy` vertical).
    /// Rectangles and images stretch exactly; **scalar** quantities (corner radius,
    /// border, blur, path) follow the average of the two factors, having no axis of
    /// their own; font size follows `sy` and wrap width follows `sx` — approximations
    /// with no consequence when `sx == sy`, as with a uniform scale or DPI. This is
    /// the basis of **non-uniform** subtree scaling.
    pub fn scaled_xy(&self, sx: f32, sy: f32) -> Primitive {
        let avg = (sx + sy) * 0.5;
        match self.clone() {
            Primitive::Rect {
                rect,
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                clip,
                owner,
            } => Primitive::Rect {
                rect: rect.scale_xy(sx, sy),
                color,
                color2,
                gradient_dir,
                radius: radius.scale(avg),
                border_width: border_width * avg,
                border_color,
                blur: blur * avg,
                clip: clip.scale_xy(sx, sy),
                owner,
            },
            Primitive::Text {
                position,
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip,
                bounds,
                owner,
            } => Primitive::Text {
                position: position.scale_xy(sx, sy),
                text,
                size: size * sy,
                color,
                weight,
                italic,
                max_width: max_width.map(|w| w * sx),
                decoration,
                decoration_color,
                clip: clip.scale_xy(sx, sy),
                bounds: bounds.scale_xy(sx, sy),
                owner,
            },
            Primitive::RichText {
                position,
                mut runs,
                max_width,
                clip,
                bounds,
                owner,
            } => {
                for run in &mut runs {
                    run.size *= sy;
                }
                Primitive::RichText {
                    position: position.scale_xy(sx, sy),
                    runs,
                    max_width: max_width.map(|w| w * sx),
                    clip: clip.scale_xy(sx, sy),
                    bounds: bounds.scale_xy(sx, sy),
                    owner,
                }
            }
            Primitive::Path {
                path,
                fill,
                gradient,
                stroke,
                clip,
                owner,
            } => Primitive::Path {
                path: path.scaled(avg),
                fill,
                gradient: gradient.map(|g| g.scaled_xy(sx, sy)),
                stroke: stroke.map(|s| Stroke::new(s.color, s.width * avg)),
                clip: clip.scale_xy(sx, sy),
                owner,
            },
            Primitive::Image {
                image,
                rect,
                uv,
                tint,
                clip,
                owner,
            } => Primitive::Image {
                image,
                rect: rect.scale_xy(sx, sy),
                // The UV is in 0..1, so it is independent of scale.
                uv,
                tint,
                clip: clip.scale_xy(sx, sy),
                owner,
            },
            Primitive::Layer {
                primitives,
                opacity,
                clip,
                clip_shape,
                transform,
                owner,
            } => Primitive::Layer {
                primitives: primitives.iter().map(|p| p.scaled_xy(sx, sy)).collect(),
                opacity,
                clip: clip.scale_xy(sx, sy),
                clip_shape: clip_shape.scaled_xy(sx, sy),
                transform: transform.map(|t| t.scaled_xy(sx, sy)),
                owner,
            },
        }
    }

    /// Offsets the **geometry** by `(dx, dy)` (position, clip); colours, sizes and
    /// text are unchanged. Combined with [`Primitive::scaled`], it scales a subtree
    /// **about a pivot**:
    /// `p.scaled(f).translated(pivot.x * (1 - f), pivot.y * (1 - f))`.
    pub fn translated(&self, dx: f32, dy: f32) -> Primitive {
        match self.clone() {
            Primitive::Rect {
                rect,
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                clip,
                owner,
            } => Primitive::Rect {
                rect: rect.translate(dx, dy),
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                clip: clip.translate(dx, dy),
                owner,
            },
            Primitive::Text {
                position,
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip,
                bounds,
                owner,
            } => Primitive::Text {
                position: Point::new(position.x + dx, position.y + dy),
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip: clip.translate(dx, dy),
                bounds: bounds.translate(dx, dy),
                owner,
            },
            Primitive::RichText {
                position,
                runs,
                max_width,
                clip,
                bounds,
                owner,
            } => Primitive::RichText {
                position: Point::new(position.x + dx, position.y + dy),
                runs,
                max_width,
                clip: clip.translate(dx, dy),
                bounds: bounds.translate(dx, dy),
                owner,
            },
            Primitive::Path {
                path,
                fill,
                gradient,
                stroke,
                clip,
                owner,
            } => Primitive::Path {
                path: path.translated(dx, dy),
                fill,
                gradient: gradient.map(|g| g.translated(dx, dy)),
                stroke,
                clip: clip.translate(dx, dy),
                owner,
            },
            Primitive::Image {
                image,
                rect,
                uv,
                tint,
                clip,
                owner,
            } => Primitive::Image {
                image,
                rect: rect.translate(dx, dy),
                uv,
                tint,
                clip: clip.translate(dx, dy),
                owner,
            },
            Primitive::Layer {
                primitives,
                opacity,
                clip,
                clip_shape,
                transform,
                owner,
            } => {
                Primitive::Layer {
                    primitives: primitives.iter().map(|p| p.translated(dx, dy)).collect(),
                    opacity,
                    clip: clip.translate(dx, dy),
                    // Radius and ellipse are translation-invariant; only the `clip`
                    // rectangle moves, which is handled above.
                    clip_shape,
                    transform: transform.map(|t| t.translated(dx, dy)),
                    owner,
                }
            }
        }
    }

    /// This primitive's (approximate) bounding box, in scene coordinates.
    /// Text is not measured in this module, so a `Text` or `RichText` primitive
    /// returns a **point** rectangle at its position, enough as an x reference.
    pub fn bounds(&self) -> Rect {
        match self {
            Primitive::Rect { rect, .. } | Primitive::Image { rect, .. } => *rect,
            Primitive::Text { position, .. } | Primitive::RichText { position, .. } => {
                Rect::new(position.x, position.y, 0.0, 0.0)
            }
            Primitive::Path { path, .. } => {
                // The control points' bounding box — conservative bounds, since it
                // encloses the Bézier controls and so is never smaller than the curve.
                // Accumulated without allocating: min/max updated point by point.
                let mut bbox: Option<(f32, f32, f32, f32)> = None;
                let mut include = |p: Point| {
                    bbox = Some(match bbox {
                        None => (p.x, p.y, p.x, p.y),
                        Some((x0, y0, x1, y1)) => {
                            (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y))
                        }
                    });
                };
                for v in path.verbs() {
                    match *v {
                        PathVerb::MoveTo(p) | PathVerb::LineTo(p) => include(p),
                        PathVerb::QuadTo { ctrl, to } => {
                            include(ctrl);
                            include(to);
                        }
                        PathVerb::CubicTo { c1, c2, to } => {
                            include(c1);
                            include(c2);
                            include(to);
                        }
                        PathVerb::Close => {}
                    }
                }
                match bbox {
                    None => Rect::new(0.0, 0.0, 0.0, 0.0),
                    Some((x0, y0, x1, y1)) => Rect::new(x0, y0, x1 - x0, y1 - y0),
                }
            }
            Primitive::Layer { primitives, .. } => primitives
                .iter()
                .map(|p| p.bounds())
                .reduce(|a, b| a.union(b))
                .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
        }
    }

    /// A copy of this primitive with its **clip** replaced by `clip` — used to
    /// "un-clip" a captured primitive replayed elsewhere, such as the ghost of a
    /// dragged header, which overflows its source column.
    pub fn with_clip(&self, clip: Rect) -> Primitive {
        match self.clone() {
            Primitive::Rect {
                rect,
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                owner,
                ..
            } => Primitive::Rect {
                rect,
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                clip,
                owner,
            },
            Primitive::Text {
                position,
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                bounds,
                owner,
                ..
            } => Primitive::Text {
                position,
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip,
                bounds,
                owner,
            },
            Primitive::RichText {
                position,
                runs,
                max_width,
                bounds,
                owner,
                ..
            } => Primitive::RichText {
                position,
                runs,
                max_width,
                clip,
                bounds,
                owner,
            },
            Primitive::Path {
                path,
                fill,
                gradient,
                stroke,
                owner,
                ..
            } => Primitive::Path {
                path,
                fill,
                gradient,
                stroke,
                clip,
                owner,
            },
            Primitive::Image {
                image,
                rect,
                uv,
                tint,
                owner,
                ..
            } => Primitive::Image {
                image,
                rect,
                uv,
                tint,
                clip,
                owner,
            },
            Primitive::Layer {
                primitives,
                opacity,
                clip_shape,
                transform,
                owner,
                ..
            } => Primitive::Layer {
                primitives,
                opacity,
                clip,
                clip_shape,
                transform,
                owner,
            },
        }
    }

    /// Scales the geometry by `factor` **about `pivot`**, which stays fixed:
    /// `pos' = pivot + (pos - pivot) * factor`. Sizes, font, radii and strokes all
    /// follow the scale.
    pub fn scaled_about(&self, pivot: Point, factor: f32) -> Primitive {
        self.scaled_about_xy(pivot, factor, factor)
    }

    /// Like [`Primitive::scaled_about`], but with **per-axis** factors (`sx`, `sy`)
    /// — a non-uniform scale about `pivot`.
    pub fn scaled_about_xy(&self, pivot: Point, sx: f32, sy: f32) -> Primitive {
        self.scaled_xy(sx, sy)
            .translated(pivot.x * (1.0 - sx), pivot.y * (1.0 - sy))
    }
}

/// A 2D scene: the declarative description of what to draw.
#[derive(Clone, Debug)]
pub struct Scene {
    primitives: Vec<Primitive>,
    current_clip: Rect,
    current_owner: u64,
    /// The box of the widget currently painting — what text primitives record so the
    /// renderer knows what they cover. `UNBOUNDED` until a walk sets it.
    current_bounds: Rect,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            primitives: Vec::new(),
            current_clip: Rect::UNBOUNDED,
            current_owner: 0,
            current_bounds: Rect::UNBOUNDED,
        }
    }
}

impl Scene {
    /// Creates an empty scene, with a neutral clip.
    pub fn new() -> Self {
        Self::default()
    }

    /// Empties the scene so it can be reused on the next frame.
    pub fn clear(&mut self) {
        self.primitives.clear();
        self.current_clip = Rect::UNBOUNDED;
        self.current_owner = 0;
        self.current_bounds = Rect::UNBOUNDED;
    }

    /// Sets the clip rectangle applied to subsequent primitives.
    pub fn set_clip(&mut self, clip: Rect) {
        self.current_clip = clip;
    }

    /// The current clip rectangle, for intersecting with local bounds.
    pub fn current_clip(&self) -> Rect {
        self.current_clip
    }

    /// Sets the emitting widget's identity for subsequent primitives.
    pub fn set_owner(&mut self, owner: u64) {
        self.current_owner = owner;
    }

    /// Declares the box the widget about to paint was laid out in. Text primitives
    /// record it, which is how the renderer knows what a line of text covers — a
    /// `Primitive::Text` otherwise says only where it begins. Set by the widget walk
    /// before every `paint`; a scene built by hand leaves it `UNBOUNDED`, and the
    /// renderer then keeps that text above everything, as it always did.
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.current_bounds = bounds;
    }

    /// The box currently being painted into.
    pub fn current_bounds(&self) -> Rect {
        self.current_bounds
    }

    /// Appends an **already-formed** primitive, with its clip and owner already
    /// baked in. This replays a cached subtree (a repaint boundary) as-is, without
    /// repainting it.
    pub fn push_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Removes and returns the primitives from index `start` onwards, preserving
    /// order. This **wraps** an already-painted subtree in a layer
    /// ([`Primitive::Layer`]): paint the subtree, then move its range of primitives
    /// into a layer, giving it group opacity.
    pub fn split_off(&mut self, start: usize) -> Vec<Primitive> {
        self.primitives.split_off(start)
    }

    /// Replays an existing primitive at reduced opacity — an exit fade.
    pub fn push_faded(&mut self, primitive: &Primitive, opacity: f32) {
        let faded = match primitive.clone() {
            Primitive::Rect {
                rect,
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                clip,
                owner,
            } => Primitive::Rect {
                rect,
                color: color.fade(opacity),
                color2: color2.fade(opacity),
                gradient_dir,
                radius,
                border_width,
                border_color: border_color.fade(opacity),
                blur,
                clip,
                owner,
            },
            Primitive::Text {
                position,
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip,
                bounds,
                owner,
            } => Primitive::Text {
                position,
                text,
                size,
                color: color.fade(opacity),
                weight,
                italic,
                max_width,
                decoration,
                decoration_color: decoration_color.map(|c| c.fade(opacity)),
                clip,
                bounds,
                owner,
            },
            Primitive::RichText {
                position,
                mut runs,
                max_width,
                clip,
                bounds,
                owner,
            } => {
                for run in &mut runs {
                    run.color = run.color.fade(opacity);
                    run.decoration_color = run.decoration_color.map(|c| c.fade(opacity));
                }
                Primitive::RichText {
                    position,
                    runs,
                    max_width,
                    clip,
                    bounds,
                    owner,
                }
            }
            Primitive::Path {
                path,
                fill,
                gradient,
                stroke,
                clip,
                owner,
            } => Primitive::Path {
                path,
                fill: fill.map(|c| c.fade(opacity)),
                gradient: gradient.map(|g| g.faded(opacity)),
                stroke: stroke.map(|s| Stroke::new(s.color.fade(opacity), s.width)),
                clip,
                owner,
            },
            Primitive::Image {
                image,
                rect,
                uv,
                tint,
                clip,
                owner,
            } => Primitive::Image {
                image,
                rect,
                uv,
                tint: tint.fade(opacity),
                clip,
                owner,
            },
            Primitive::Layer {
                primitives,
                opacity: group,
                clip,
                clip_shape,
                transform,
                owner,
            } => Primitive::Layer {
                primitives,
                // Fading a layer means dimming its group opacity.
                opacity: group * opacity,
                clip,
                clip_shape,
                transform,
                owner,
            },
        };
        self.primitives.push(faded);
    }

    /// Adds a solid rectangle, with square corners and no border.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius: BorderRadius::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Adds a rectangle with rounded corners (uniform through `f32`, or per corner
    /// through [`BorderRadius`]) and/or a border.
    pub fn draw_rect(
        &mut self,
        rect: Rect,
        color: Color,
        radius: impl Into<BorderRadius>,
        border_width: f32,
        border_color: Color,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius: radius.into(),
            border_width,
            border_color,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Adds a linear-gradient rectangle (`color` → `color2` along `dir`).
    // Eight, and every one of them a property of the same rectangle. Grouping them
    // into a struct is an API change with call sites, not a lint fix.
    #[allow(clippy::too_many_arguments)]
    pub fn gradient_rect(
        &mut self,
        rect: Rect,
        color: Color,
        color2: Color,
        dir: [f32; 2],
        radius: impl Into<BorderRadius>,
        border_width: f32,
        border_color: Color,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2,
            gradient_dir: dir,
            radius: radius.into(),
            border_width,
            border_color,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Adds a soft shadow — a rounded rectangle with a blurred edge, no border.
    pub fn shadow(&mut self, rect: Rect, color: Color, radius: impl Into<BorderRadius>, blur: f32) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius: radius.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Fills a vector path with a flat colour, using the *non-zero* rule.
    pub fn fill_path(&mut self, path: &Path, color: Color) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill: Some(color),
            gradient: None,
            stroke: None,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Fills a path with a **linear gradient**: `from_color` at `from`, `to_color` at
    /// `to`, clamped outside that span.
    ///
    /// The two points are in the path's own space rather than relative to its box, so
    /// the fade can be aimed at the thing it is about — the edge a glow springs from,
    /// the depth of a band — which is what an ellipse cap needs, its bounding box
    /// being mostly off screen.
    pub fn fill_path_gradient(
        &mut self,
        path: &Path,
        from_color: Color,
        to_color: Color,
        from: Point,
        to: Point,
    ) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill: Some(from_color),
            gradient: Some(PathGradient::Linear { to_color, from, to }),
            stroke: None,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Fills a path with a **radial gradient**: `from_color` on the ellipse of radii
    /// `inner × radii` about `center`, `to_color` on the ellipse of radii `radii`.
    ///
    /// This is what a shape with a **curved** boundary needs in order to fade to
    /// nothing all the way round: a straight fade reaches zero on a line, so it still
    /// leaves an edge wherever the shape turns away from that line.
    pub fn fill_path_radial(
        &mut self,
        path: &Path,
        from_color: Color,
        to_color: Color,
        center: Point,
        radii: Size,
        inner: f32,
    ) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill: Some(from_color),
            gradient: Some(PathGradient::Radial {
                to_color,
                center,
                radii,
                inner,
            }),
            stroke: None,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Strokes a path's outline (colour plus width), with no fill.
    pub fn stroke_path(&mut self, path: &Path, color: Color, width: f32) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill: None,
            gradient: None,
            stroke: Some(Stroke::new(color, width)),
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Adds a path filled **and/or** stroked — both passes in one primitive.
    pub fn paint_path(&mut self, path: &Path, fill: Option<Color>, stroke: Option<Stroke>) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill,
            gradient: None,
            stroke,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Draws an image into `rect`, sampling the `uv` sub-region (in `0..1`) and
    /// tinting it by `tint` (white = unchanged). Low level: see [`Scene::image`]
    /// for automatic fitting through [`crate::BoxFit`].
    pub fn draw_image(&mut self, image: &ImageHandle, rect: Rect, uv: Rect, tint: Color) {
        self.primitives.push(Primitive::Image {
            image: image.clone(),
            rect,
            uv,
            tint,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Draws an image fitted into `rect` per `fit` (aspect preserved, letterboxed
    /// or cropped), untinted.
    pub fn image(&mut self, image: &ImageHandle, rect: Rect, fit: crate::BoxFit) {
        let (dst, uv) = fit.apply(image.size(), rect);
        self.draw_image(image, dst, uv, Color::WHITE);
    }

    /// Composes a **layer**: `build` fills a subgroup of primitives, which is then
    /// composited **as one** at `opacity` (`0..1`). Unlike an opacity applied
    /// primitive by primitive, the group alpha stays correct where the inner
    /// primitives overlap. The layer inherits the current clip and owner.
    pub fn layer(&mut self, opacity: f32, build: impl FnOnce(&mut Scene)) {
        let mut inner = Scene::new();
        inner.current_clip = self.current_clip;
        inner.current_owner = self.current_owner;
        build(&mut inner);
        self.primitives.push(Primitive::Layer {
            primitives: inner.primitives,
            opacity,
            clip: self.current_clip,
            clip_shape: ClipShape::Rect,
            transform: None,
            owner: self.current_owner,
        });
    }

    /// Adds a line of text, anchored by its top-left corner, regular and upright.
    /// See [`Scene::text_styled`] for weight and italics.
    pub fn text(&mut self, position: Point, text: impl Into<String>, size: f32, color: Color) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size,
            color,
            weight: FontWeight::Regular,
            italic: false,
            max_width: None,
            decoration: TextDecoration::NONE,
            decoration_color: None,
            clip: self.current_clip,
            bounds: self.current_bounds,
            owner: self.current_owner,
        });
    }

    /// Adds **rich** text: resolved runs, mixing styles and colours, laid out as one
    /// piece, anchored by its top-left corner.
    pub fn rich_text(&mut self, position: Point, runs: Vec<TextRun>) {
        self.primitives.push(Primitive::RichText {
            position,
            runs,
            max_width: None,
            clip: self.current_clip,
            bounds: self.current_bounds,
            owner: self.current_owner,
        });
    }

    /// Adds a **rich paragraph**: the runs wrap beyond `max_width`, and the render's
    /// wrapping matches the layout's.
    pub fn rich_text_wrapped(&mut self, position: Point, runs: Vec<TextRun>, max_width: f32) {
        self.primitives.push(Primitive::RichText {
            position,
            runs,
            max_width: Some(max_width),
            clip: self.current_clip,
            bounds: self.current_bounds,
            owner: self.current_owner,
        });
    }

    /// Adds a styled line of text — size, weight and italics from the [`TextStyle`].
    /// `color` is the **resolved** colour: the style's optional `color` has already
    /// been settled by the caller, usually against the theme.
    pub fn text_styled(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: &TextStyle,
        color: Color,
    ) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size: style.size,
            color,
            weight: style.weight,
            italic: style.italic,
            max_width: None,
            decoration: style.decoration,
            decoration_color: style.decoration_color,
            clip: self.current_clip,
            bounds: self.current_bounds,
            owner: self.current_owner,
        });
    }

    /// Adds a **paragraph**: styled text that wraps beyond `max_width`, the render's
    /// wrapping matching the layout's.
    pub fn text_wrapped(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: &TextStyle,
        color: Color,
        max_width: f32,
    ) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size: style.size,
            color,
            weight: style.weight,
            italic: style.italic,
            max_width: Some(max_width),
            decoration: style.decoration,
            decoration_color: style.decoration_color,
            clip: self.current_clip,
            bounds: self.current_bounds,
            owner: self.current_owner,
        });
    }

    /// The number of primitives in the scene.
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    /// `true` when the scene holds no primitives at all.
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    /// The primitives, in insertion order, which is drawing order.
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    /// A copy of the scene with all geometry scaled by `factor` — the logical →
    /// physical conversion for HiDPI rendering.
    pub fn scaled(&self, factor: f32) -> Scene {
        Scene {
            primitives: self.primitives.iter().map(|p| p.scaled(factor)).collect(),
            current_clip: self.current_clip.scale(factor),
            current_owner: self.current_owner,
            current_bounds: self.current_bounds.scale(factor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Rect};

    #[test]
    fn fill_rect_pushes_primitive_with_current_clip() {
        let mut scene = Scene::new();
        assert!(scene.is_empty());
        scene.fill_rect(Rect::new(1.0, 2.0, 3.0, 4.0), Color::WHITE);
        assert_eq!(scene.len(), 1);
        assert_eq!(
            scene.primitives()[0],
            Primitive::Rect {
                rect: Rect::new(1.0, 2.0, 3.0, 4.0),
                color: Color::WHITE,
                color2: Color::WHITE,
                gradient_dir: [0.0, 0.0],
                radius: BorderRadius::ZERO,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                blur: 0.0,
                clip: Rect::UNBOUNDED,
                owner: 0,
            }
        );
    }

    /// A gradient describes geometry, so it has to move with the geometry it
    /// describes — otherwise a fade aimed at the top of a list would stay behind when
    /// the DPI scale or a layer offset moved the shape.
    #[test]
    fn a_radial_gradient_travels_with_its_shape() {
        let mut scene = Scene::new();
        scene.fill_path_radial(
            &Path::oval(Rect::new(0.0, 0.0, 40.0, 20.0)),
            Color::WHITE,
            Color::TRANSPARENT,
            Point::new(20.0, 10.0),
            Size::new(20.0, 10.0),
            0.8,
        );
        let moved = scene.primitives()[0].clone().scaled_xy(2.0, 3.0);
        let Primitive::Path { gradient, .. } = moved.translated(5.0, 7.0) else {
            panic!("a filled path");
        };
        assert_eq!(
            gradient,
            Some(PathGradient::Radial {
                to_color: Color::TRANSPARENT,
                center: Point::new(45.0, 37.0),
                radii: Size::new(40.0, 30.0),
                // The rind is a ratio, so it is the one thing scaling must not touch.
                inner: 0.8,
            })
        );
        assert_eq!(gradient.unwrap().faded(0.5).to_color().a, 0.0);
    }

    #[test]
    fn push_faded_scales_alpha_and_keeps_owner() {
        let mut scene = Scene::new();
        scene.set_owner(42);
        scene.fill_rect(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::rgba(1.0, 0.0, 0.0, 1.0),
        );
        let source = scene.primitives()[0].clone();
        assert_eq!(source.owner(), 42);

        let mut target = Scene::new();
        target.push_faded(&source, 0.5);
        if let Primitive::Rect { color, owner, .. } = target.primitives()[0] {
            assert_eq!(color.a, 0.5);
            assert_eq!(owner, 42);
        } else {
            panic!("expected a rect");
        }
    }

    #[test]
    fn scaled_multiplies_geometry_not_colors() {
        let mut scene = Scene::new();
        scene.draw_rect(
            Rect::new(2.0, 4.0, 10.0, 20.0),
            Color::rgb(1.0, 0.0, 0.0),
            3.0,
            1.0,
            Color::WHITE,
        );
        scene.text(Point::new(5.0, 6.0), "hi", 18.0, Color::BLACK);

        let big = scene.scaled(2.0);
        match &big.primitives()[0] {
            Primitive::Rect {
                rect,
                radius,
                border_width,
                color,
                ..
            } => {
                assert_eq!(*rect, Rect::new(4.0, 8.0, 20.0, 40.0));
                assert_eq!(*radius, BorderRadius::uniform(6.0));
                assert_eq!(*border_width, 2.0);
                assert_eq!(*color, Color::rgb(1.0, 0.0, 0.0)); // colour unchanged
            }
            _ => panic!("expected a rect"),
        }
        match &big.primitives()[1] {
            Primitive::Text {
                position,
                size,
                text,
                ..
            } => {
                assert_eq!(*position, Point::new(10.0, 12.0));
                assert_eq!(*size, 36.0);
                assert_eq!(text, "hi"); // text unchanged
            }
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn layer_captures_subprimitives_and_opacity() {
        let mut scene = Scene::new();
        scene.set_owner(7);
        scene.set_clip(Rect::new(0.0, 0.0, 50.0, 50.0));
        scene.layer(0.5, |inner| {
            inner.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE);
            inner.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), Color::BLACK);
        });
        assert_eq!(scene.len(), 1);
        match &scene.primitives()[0] {
            Primitive::Layer {
                primitives,
                opacity,
                clip,
                owner,
                ..
            } => {
                assert_eq!(primitives.len(), 2);
                assert_eq!(*opacity, 0.5);
                assert_eq!(*clip, Rect::new(0.0, 0.0, 50.0, 50.0));
                assert_eq!(*owner, 7);
            }
            _ => panic!("expected a layer"),
        }
    }

    #[test]
    fn fading_a_layer_scales_its_group_opacity() {
        let mut scene = Scene::new();
        scene.layer(0.8, |inner| {
            inner.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE)
        });
        let layer = scene.primitives()[0].clone();
        let mut target = Scene::new();
        target.push_faded(&layer, 0.5);
        match &target.primitives()[0] {
            Primitive::Layer { opacity, .. } => assert!((*opacity - 0.4).abs() < 1e-6),
            _ => panic!("expected a layer"),
        }
    }

    #[test]
    fn scaling_a_layer_scales_its_children() {
        let mut scene = Scene::new();
        scene.layer(1.0, |inner| {
            inner.fill_rect(Rect::new(2.0, 3.0, 4.0, 5.0), Color::WHITE)
        });
        let big = scene.scaled(2.0);
        match &big.primitives()[0] {
            Primitive::Layer { primitives, .. } => match &primitives[0] {
                Primitive::Rect { rect, .. } => assert_eq!(*rect, Rect::new(4.0, 6.0, 8.0, 10.0)),
                _ => panic!("expected a rect"),
            },
            _ => panic!("expected a layer"),
        }
    }

    #[test]
    fn set_clip_applies_to_following_primitives() {
        let mut scene = Scene::new();
        let clip = Rect::new(0.0, 0.0, 10.0, 10.0);
        scene.set_clip(clip);
        scene.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::BLACK);
        if let Primitive::Rect { clip: c, .. } = scene.primitives()[0] {
            assert_eq!(c, clip);
        } else {
            panic!("expected a rect");
        }
    }
}
