//! **Pixel effects** applied to a layer when it is composited: a colour transform, an
//! image filter, a mask.
//!
//! A [`Primitive::Layer`](crate::Primitive::Layer) is already rendered on its own
//! before being composited, which is what makes group opacity correct. That separate
//! texture is also, exactly, the place where an effect over *a whole subtree* can be
//! applied — so the effects here are fields of a layer rather than a kind of
//! primitive.
//!
//! ## The order they apply in
//!
//! One layer may carry all three, and the order is fixed:
//!
//! 1. the [`ImageFilter`] — it treats the layer as an image, so it must run while the
//!    pixels are still the layer's own;
//! 2. the [`ColorFilter`] — a function of one pixel, applied to the filtered result;
//! 3. the [`ShaderMask`] — a colour blended over what the first two produced;
//!
//! and then, as before, the group opacity and the clip shape.
//!
//! ## Colour space
//!
//! Blending and the colour matrix are defined on **sRGB-encoded** values, the space
//! colours are authored in, not on the linear light the GPU holds. The two disagree
//! sharply: a multiply in linear light darkens roughly twice as fast as the same
//! multiply on the encoded values, so a filter written against one and evaluated in
//! the other is not slightly off, it is a different filter. A blur is the exception
//! and is done in **linear** light, because a blur is an average of *light*, and
//! averaging encoded values makes a bright edge over a dark one visibly grey.

use crate::{Color, Point, Rect};

/// How a source colour is combined with the destination underneath it.
///
/// The Porter–Duff set plus the separable blends, evaluated on **premultiplied,
/// sRGB-encoded** values. `SrcOver` is the ordinary "paint this on top" and the
/// default nearly everywhere; the rest exist because a mask or a colour filter is
/// often defined by one of them — a fade-out is `DstIn`, a tint is `Modulate`, a
/// highlight is `Screen`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// The source only; the destination is discarded.
    Src,
    /// The destination only; the source is discarded.
    Dst,
    /// The source over the destination — the ordinary one.
    #[default]
    SrcOver,
    /// The destination over the source.
    DstOver,
    /// The source, kept only where the destination is opaque.
    SrcIn,
    /// The destination, kept only where the source is opaque.
    DstIn,
    /// The source, kept only where the destination is *transparent*.
    SrcOut,
    /// The destination, kept only where the source is transparent.
    DstOut,
    /// The source over the destination, clipped to the destination's shape.
    SrcAtop,
    /// The destination over the source, clipped to the source's shape.
    DstAtop,
    /// Whichever of the two is alone there.
    Xor,
    /// The two added, clamped at white.
    Plus,
    /// The two multiplied, **ignoring** coverage — the classic tint, and what a
    /// [`ShaderMask`] uses by default.
    Modulate,
    /// The two multiplied, respecting coverage: always darker.
    Multiply,
    /// The inverse of multiplying the inverses: always lighter.
    Screen,
    /// `Multiply` where the destination is dark, `Screen` where it is light.
    Overlay,
    /// The darker of the two, per channel.
    Darken,
    /// The lighter of the two, per channel.
    Lighten,
}

impl BlendMode {
    /// The mode's rank in the shader's branch — the one number the GPU is given.
    /// Public because `frus-gpu` is a separate crate; there is no other reason to
    /// look at it.
    pub const fn code(self) -> u32 {
        match self {
            BlendMode::Src => 0,
            BlendMode::Dst => 1,
            BlendMode::SrcOver => 2,
            BlendMode::DstOver => 3,
            BlendMode::SrcIn => 4,
            BlendMode::DstIn => 5,
            BlendMode::SrcOut => 6,
            BlendMode::DstOut => 7,
            BlendMode::SrcAtop => 8,
            BlendMode::DstAtop => 9,
            BlendMode::Xor => 10,
            BlendMode::Plus => 11,
            BlendMode::Modulate => 12,
            BlendMode::Multiply => 13,
            BlendMode::Screen => 14,
            BlendMode::Overlay => 15,
            BlendMode::Darken => 16,
            BlendMode::Lighten => 17,
        }
    }
}

/// A function applied to **every pixel** of a subtree, independently of its
/// neighbours: greyscale, a tint, a contrast curve.
///
/// The general form is a matrix; the named constructors are ordinary matrices with
/// familiar names, and are there so that `ColorFilter::grayscale()` does not have to
/// be twenty numbers at the call site.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorFilter {
    /// A **5×4 row-major** matrix over `(r, g, b, a, 1)`, on sRGB-encoded values in
    /// `0..1`. Row `i` gives output channel `i`; the fifth column is a constant
    /// added to it.
    ///
    /// ```text
    /// r' = m[0]·r  + m[1]·g  + m[2]·b  + m[3]·a  + m[4]
    /// g' = m[5]·r  + …                           + m[9]
    /// b' = m[10]·r + …                           + m[14]
    /// a' = m[15]·r + …                           + m[19]
    /// ```
    Matrix([f32; 20]),
    /// A colour blended into every pixel with `mode`. `ColorFilter::Mode(c,
    /// BlendMode::SrcIn)` replaces the subtree with `c` at the subtree's own
    /// coverage — a silhouette.
    Mode(Color, BlendMode),
}

impl ColorFilter {
    /// The identity: every pixel unchanged. The matrix a named filter is a
    /// perturbation of.
    pub const IDENTITY: [f32; 20] = [
        1.0, 0.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];

    /// The luminance weights of the sRGB primaries — how much each contributes to
    /// perceived brightness. Green carries most of it, which is why a naive
    /// `(r + g + b) / 3` greyscale looks wrong.
    const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

    /// Colour drained away entirely: every channel becomes the pixel's luminance.
    pub const fn grayscale() -> ColorFilter {
        ColorFilter::saturate(0.0)
    }

    /// Saturation scaled by `amount`: `0` is [`grayscale`](Self::grayscale), `1`
    /// leaves the pixel alone, and above `1` pushes colours apart.
    ///
    /// Each channel is interpolated between its own value and the shared luminance,
    /// which is the only saturation that leaves a grey pixel grey.
    pub const fn saturate(amount: f32) -> ColorFilter {
        let s = amount;
        let (lr, lg, lb) = (Self::LUMA[0], Self::LUMA[1], Self::LUMA[2]);
        ColorFilter::Matrix([
            lr + s * (1.0 - lr),
            lg - s * lg,
            lb - s * lb,
            0.0,
            0.0,
            //
            lr - s * lr,
            lg + s * (1.0 - lg),
            lb - s * lb,
            0.0,
            0.0,
            //
            lr - s * lr,
            lg - s * lg,
            lb + s * (1.0 - lb),
            0.0,
            0.0,
            //
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ])
    }

    /// Every channel inverted — a photographic negative. Alpha is untouched, so the
    /// shape stays exactly where it was.
    pub const fn invert() -> ColorFilter {
        ColorFilter::Matrix([
            -1.0, 0.0, 0.0, 0.0, 1.0, //
            0.0, -1.0, 0.0, 0.0, 1.0, //
            0.0, 0.0, -1.0, 0.0, 1.0, //
            0.0, 0.0, 0.0, 1.0, 0.0,
        ])
    }

    /// Every channel scaled by `amount`: below `1` darkens, above `1` brightens.
    pub const fn brightness(amount: f32) -> ColorFilter {
        ColorFilter::Matrix([
            amount, 0.0, 0.0, 0.0, 0.0, //
            0.0, amount, 0.0, 0.0, 0.0, //
            0.0, 0.0, amount, 0.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, 0.0,
        ])
    }

    /// Contrast scaled about mid-grey: `0` flattens everything to grey, `1` changes
    /// nothing, above `1` pushes light and dark apart.
    pub const fn contrast(amount: f32) -> ColorFilter {
        let t = 0.5 * (1.0 - amount);
        ColorFilter::Matrix([
            amount, 0.0, 0.0, 0.0, t, //
            0.0, amount, 0.0, 0.0, t, //
            0.0, 0.0, amount, 0.0, t, //
            0.0, 0.0, 0.0, 1.0, 0.0,
        ])
    }

    /// The warm brown cast of an old print.
    pub const fn sepia() -> ColorFilter {
        ColorFilter::Matrix([
            0.393, 0.769, 0.189, 0.0, 0.0, //
            0.349, 0.686, 0.168, 0.0, 0.0, //
            0.272, 0.534, 0.131, 0.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, 0.0,
        ])
    }
}

/// A filter that treats the subtree **as an image**, so a pixel's result depends on
/// its neighbours: a blur, a spread, a shrink.
///
/// There is no `matrix` variant. A layer already carries an affine `transform`, which
/// is what a matrix image filter is, and the widget that reaches it is `Transform`; a
/// second spelling here would be the same feature under a different name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageFilter {
    /// A Gaussian blur, with a **separate** standard deviation per axis, in logical
    /// pixels. `sigma_x` and `sigma_y` may differ, which is how a directional smear
    /// is written.
    Blur { sigma_x: f32, sigma_y: f32 },
    /// The brightest neighbour within the radius wins: light shapes **grow**. The
    /// building block of an outer glow or a fattened outline.
    Dilate { radius_x: f32, radius_y: f32 },
    /// The dimmest neighbour within the radius wins: light shapes **shrink**.
    Erode { radius_x: f32, radius_y: f32 },
}

impl ImageFilter {
    /// A Gaussian blur, the same in both directions.
    pub const fn blur(sigma: f32) -> ImageFilter {
        ImageFilter::Blur {
            sigma_x: sigma,
            sigma_y: sigma,
        }
    }

    /// The filter's reach in pixels, per axis — how far a pixel can pull from. A
    /// Gaussian is truncated at three standard deviations, past which it contributes
    /// under half a percent.
    pub fn radius(self) -> (f32, f32) {
        match self {
            ImageFilter::Blur { sigma_x, sigma_y } => (sigma_x * 3.0, sigma_y * 3.0),
            ImageFilter::Dilate { radius_x, radius_y }
            | ImageFilter::Erode { radius_x, radius_y } => (radius_x, radius_y),
        }
    }

    /// `true` when the filter would leave every pixel alone, so the whole pre-pass
    /// can be skipped. A zero radius is the only such case, and it is a common one:
    /// an animated blur passes through zero at both ends.
    pub fn is_identity(self) -> bool {
        let (rx, ry) = self.radius();
        rx <= 0.0 && ry <= 0.0
    }

    /// Scales the filter's geometry per axis, as DPI does. A radius is a length and
    /// follows its own axis.
    pub fn scaled_xy(self, sx: f32, sy: f32) -> ImageFilter {
        match self {
            ImageFilter::Blur { sigma_x, sigma_y } => ImageFilter::Blur {
                sigma_x: sigma_x * sx,
                sigma_y: sigma_y * sy,
            },
            ImageFilter::Dilate { radius_x, radius_y } => ImageFilter::Dilate {
                radius_x: radius_x * sx,
                radius_y: radius_y * sy,
            },
            ImageFilter::Erode { radius_x, radius_y } => ImageFilter::Erode {
                radius_x: radius_x * sx,
                radius_y: radius_y * sy,
            },
        }
    }

    /// The shader's rank for this kind.
    pub const fn code(self) -> u32 {
        match self {
            ImageFilter::Blur { .. } => 0,
            ImageFilter::Dilate { .. } => 1,
            ImageFilter::Erode { .. } => 2,
        }
    }
}

/// The colour a [`ShaderMask`] blends over its subtree, as a two-stop fade.
///
/// The geometry is **absolute**, in the same space as the primitives it covers —
/// the same choice [`PathGradient`](crate::PathGradient) makes, and for the same
/// reason: a fade should be aimed at something real, and a scene primitive holding
/// a fraction of a box no longer knows which box.
///
/// The widget is where fractions belong, and `ShaderMask` there does take them: it
/// resolves a [`FractionalMask`] against its own box on the way into the scene.
///
/// Two stops rather than a list, which is what the gradients elsewhere in this
/// framework offer; a mask needing more is a `CustomPaint` away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaskShader {
    /// A straight fade from `from` to `to`, clamped outside that span.
    Linear {
        /// Where `from_color` holds.
        from: Point,
        /// Where `to_color` holds.
        to: Point,
        /// The colour at `from`.
        from_color: Color,
        /// The colour at `to`.
        to_color: Color,
    },
    /// A fade outwards from `center`, reaching `to_color` at `radius`.
    Radial {
        /// The centre.
        center: Point,
        /// Where `to_color` holds, in pixels from the centre.
        radius: f32,
        /// The colour at the centre.
        from_color: Color,
        /// The colour at `radius` and beyond.
        to_color: Color,
    },
}

impl MaskShader {
    /// The shader rank for this kind.
    pub const fn code(self) -> u32 {
        match self {
            MaskShader::Linear { .. } => 0,
            MaskShader::Radial { .. } => 1,
        }
    }

    /// Scales the geometry per axis, as a DPI or paint scale does. A radius has no
    /// axis of its own and follows the average, like every other scalar length in a
    /// scene.
    pub fn scaled_xy(self, sx: f32, sy: f32) -> MaskShader {
        match self {
            MaskShader::Linear {
                from,
                to,
                from_color,
                to_color,
            } => MaskShader::Linear {
                from: Point::new(from.x * sx, from.y * sy),
                to: Point::new(to.x * sx, to.y * sy),
                from_color,
                to_color,
            },
            MaskShader::Radial {
                center,
                radius,
                from_color,
                to_color,
            } => MaskShader::Radial {
                center: Point::new(center.x * sx, center.y * sy),
                radius: radius * (sx + sy) * 0.5,
                from_color,
                to_color,
            },
        }
    }

    /// Moves the geometry with the subtree it covers.
    pub fn translated(self, dx: f32, dy: f32) -> MaskShader {
        match self {
            MaskShader::Linear {
                from,
                to,
                from_color,
                to_color,
            } => MaskShader::Linear {
                from: Point::new(from.x + dx, from.y + dy),
                to: Point::new(to.x + dx, to.y + dy),
                from_color,
                to_color,
            },
            MaskShader::Radial {
                center,
                radius,
                from_color,
                to_color,
            } => MaskShader::Radial {
                center: Point::new(center.x + dx, center.y + dy),
                radius,
                from_color,
                to_color,
            },
        }
    }
}

/// A colour blended over a subtree — the `ShaderMask` widget's payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShaderMask {
    /// The colour, as a fade over the masked box.
    pub shader: MaskShader,
    /// How that colour meets the subtree. [`BlendMode::Modulate`] — the default —
    /// multiplies, so the shader's alpha becomes the subtree's transparency, which
    /// is what makes a mask a mask.
    pub blend: BlendMode,
}

impl ShaderMask {
    /// A mask that multiplies `shader` into the subtree.
    pub const fn new(shader: MaskShader) -> ShaderMask {
        ShaderMask {
            shader,
            blend: BlendMode::Modulate,
        }
    }

    /// Meets the subtree with `blend` instead of multiplying.
    pub const fn blend(mut self, blend: BlendMode) -> ShaderMask {
        self.blend = blend;
        self
    }
}

/// A [`MaskShader`] written in **fractions of the box it covers** — `(0, 0)` the
/// top-left corner, `(1, 1)` the bottom-right.
///
/// This is what a caller writes, because a caller knows the shape of the effect it
/// wants and not the pixels the box will land on. [`LayerFilter::resolve_mask`]
/// turns it into the absolute geometry a scene holds, once the box is known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FractionalMask {
    /// A straight fade from `from` to `to`.
    Linear {
        /// Where `from_color` holds, in fractions of the box.
        from: (f32, f32),
        /// Where `to_color` holds, in fractions of the box.
        to: (f32, f32),
        /// The colour at `from`.
        from_color: Color,
        /// The colour at `to`.
        to_color: Color,
    },
    /// A fade outwards from `center`, reaching `to_color` at `radius` — a fraction
    /// of the box smaller side.
    Radial {
        /// The centre, in fractions of the box.
        center: (f32, f32),
        /// Where `to_color` holds, as a fraction of the box smaller side.
        radius: f32,
        /// The colour at the centre.
        from_color: Color,
        /// The colour at `radius` and beyond.
        to_color: Color,
    },
}

impl FractionalMask {
    /// A top-to-bottom fade from opaque to transparent: the common "the list fades
    /// out at the bottom" mask, written once.
    pub fn fade_out_bottom() -> FractionalMask {
        FractionalMask::Linear {
            from: (0.0, 0.0),
            to: (0.0, 1.0),
            from_color: Color::WHITE,
            to_color: Color::WHITE.fade(0.0),
        }
    }
}

/// Everything a layer applies to its own pixels at compositing time. All three are
/// optional and independent; [`LayerFilter::is_none`] is the fast path the renderer
/// takes when nothing is asked for, which is nearly always.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayerFilter {
    /// Treats the layer as an image: a blur, a dilate, an erode.
    pub image: Option<ImageFilter>,
    /// A function of one pixel: a tint, a greyscale, a contrast curve.
    pub color: Option<ColorFilter>,
    /// A colour blended over the result.
    pub mask: Option<ShaderMask>,
}

impl LayerFilter {
    /// Nothing at all — the default, and what every layer that is only an opacity
    /// group or a clip carries.
    pub const NONE: LayerFilter = LayerFilter {
        image: None,
        color: None,
        mask: None,
    };

    /// `true` when this layer asks for no effect, so compositing can take its
    /// original path unchanged.
    pub fn is_none(&self) -> bool {
        self.image.is_none() && self.color.is_none() && self.mask.is_none()
    }

    /// Folds `other` into this filter, keeping what is already set.
    ///
    /// This is what lets two filter widgets wrapped one inside the other share a
    /// single layer instead of nesting two. It returns `None` when both sides ask
    /// for the **same** slot, because there the outer one is a filter *of the
    /// inner's result* and merging would silently drop one of them.
    pub fn merge(self, other: LayerFilter) -> Option<LayerFilter> {
        let clash = (self.image.is_some() && other.image.is_some())
            || (self.color.is_some() && other.color.is_some())
            || (self.mask.is_some() && other.mask.is_some());
        if clash {
            return None;
        }
        Some(LayerFilter {
            image: self.image.or(other.image),
            color: self.color.or(other.color),
            mask: self.mask.or(other.mask),
        })
    }

    /// Scales the geometry per axis, as DPI does. A colour matrix has no length in
    /// it and is left alone; the blur radius and the mask both have one.
    pub fn scaled_xy(self, sx: f32, sy: f32) -> LayerFilter {
        LayerFilter {
            image: self.image.map(|f| f.scaled_xy(sx, sy)),
            color: self.color,
            mask: self.mask.map(|m| ShaderMask {
                shader: m.shader.scaled_xy(sx, sy),
                blend: m.blend,
            }),
        }
    }

    /// Moves the geometry with the subtree it covers. Only the mask has any: a blur
    /// radius is a length, not a place.
    pub fn translated(self, dx: f32, dy: f32) -> LayerFilter {
        LayerFilter {
            mask: self.mask.map(|m| ShaderMask {
                shader: m.shader.translated(dx, dy),
                blend: m.blend,
            }),
            ..self
        }
    }

    /// Resolves a mask written in **fractions of `box_rect`** into the absolute
    /// geometry a scene holds. A radial radius is a fraction of the box smaller
    /// side, so a circle stays a circle in a box that is not square.
    pub fn resolve_mask(shader: FractionalMask, box_rect: Rect) -> MaskShader {
        let at = |(fx, fy): (f32, f32)| {
            Point::new(
                box_rect.x + box_rect.width * fx,
                box_rect.y + box_rect.height * fy,
            )
        };
        match shader {
            FractionalMask::Linear {
                from,
                to,
                from_color,
                to_color,
            } => MaskShader::Linear {
                from: at(from),
                to: at(to),
                from_color,
                to_color,
            },
            FractionalMask::Radial {
                center,
                radius,
                from_color,
                to_color,
            } => MaskShader::Radial {
                center: at(center),
                radius: radius * box_rect.width.min(box_rect.height),
                from_color,
                to_color,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturating_by_one_is_the_identity() {
        let ColorFilter::Matrix(m) = ColorFilter::saturate(1.0) else {
            panic!("saturate is a matrix");
        };
        for (got, want) in m.iter().zip(ColorFilter::IDENTITY.iter()) {
            assert!((got - want).abs() < 1e-6, "{m:?}");
        }
    }

    #[test]
    fn grayscale_leaves_a_grey_pixel_alone() {
        let ColorFilter::Matrix(m) = ColorFilter::grayscale() else {
            panic!("grayscale is a matrix");
        };
        // Each row's r, g, b weights sum to one, so a pixel with r == g == b keeps it.
        for row in 0..3 {
            let sum: f32 = m[row * 5] + m[row * 5 + 1] + m[row * 5 + 2];
            assert!((sum - 1.0).abs() < 1e-5, "row {row} sums to {sum}");
        }
    }

    #[test]
    fn a_zero_blur_is_skipped_but_a_small_one_is_not() {
        assert!(ImageFilter::blur(0.0).is_identity());
        assert!(!ImageFilter::blur(0.01).is_identity());
    }

    #[test]
    fn dpi_scales_a_blur_by_axis() {
        let f = ImageFilter::Blur {
            sigma_x: 4.0,
            sigma_y: 2.0,
        }
        .scaled_xy(2.0, 3.0);
        assert_eq!(
            f,
            ImageFilter::Blur {
                sigma_x: 8.0,
                sigma_y: 6.0
            }
        );
    }

    #[test]
    fn merging_two_different_slots_gives_one_layer() {
        let outer = LayerFilter {
            color: Some(ColorFilter::grayscale()),
            ..LayerFilter::NONE
        };
        let inner = LayerFilter {
            image: Some(ImageFilter::blur(4.0)),
            ..LayerFilter::NONE
        };
        let merged = outer.merge(inner).expect("different slots merge");
        assert!(merged.color.is_some() && merged.image.is_some());
    }

    #[test]
    fn merging_the_same_slot_refuses() {
        let a = LayerFilter {
            color: Some(ColorFilter::grayscale()),
            ..LayerFilter::NONE
        };
        let b = LayerFilter {
            color: Some(ColorFilter::invert()),
            ..LayerFilter::NONE
        };
        // The greyscale of an inverted picture is not the inversion of a greyscale
        // one, so there is no single layer that means both.
        assert!(a.merge(b).is_none());
    }

    #[test]
    fn an_empty_filter_is_none() {
        assert!(LayerFilter::NONE.is_none());
        assert!(!LayerFilter {
            mask: Some(ShaderMask::new(LayerFilter::resolve_mask(
                FractionalMask::fade_out_bottom(),
                Rect::new(0.0, 0.0, 10.0, 10.0),
            ))),
            ..LayerFilter::NONE
        }
        .is_none());
    }

    #[test]
    fn every_blend_mode_has_its_own_code() {
        let modes = [
            BlendMode::Src,
            BlendMode::Dst,
            BlendMode::SrcOver,
            BlendMode::DstOver,
            BlendMode::SrcIn,
            BlendMode::DstIn,
            BlendMode::SrcOut,
            BlendMode::DstOut,
            BlendMode::SrcAtop,
            BlendMode::DstAtop,
            BlendMode::Xor,
            BlendMode::Plus,
            BlendMode::Modulate,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
        ];
        let mut codes: Vec<u32> = modes.iter().map(|m| m.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), modes.len());
    }
}
