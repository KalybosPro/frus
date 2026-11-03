// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **Masks and filters as layers.**
//!
//! Neither is drawn by a renderer of its own. Each becomes groups of the picture's items
//! whose layers do the work the scene already knows how to do:
//!
//! - a **mask** is a layer appended to the masked group that meets the group's pixels with
//!   `DstIn`, keeping them where it is opaque. For an alpha mask that layer is the mask's
//!   content; for a luminance mask, the content again inside it, turned into its luminance by
//!   a colour matrix and met with `DstIn` too, so the coverage is luminance times alpha;
//! - a **filter** is evaluated as a graph of item lists, one per named result: a blur is a
//!   blurred layer, an offset moves the items, a flood is a rectangle, a merge lists its
//!   inputs one over the other, `in` and `out` are layers blending by `SrcIn` and `SrcOut`,
//!   a colour matrix a colour-filtered layer.
//!
//! A filter using a primitive with no such layer — lighting, turbulence, a convolution, a
//! displacement, arithmetic compositing, a colour matrix that mixes channels in linear light
//! — is not drawn at all, and the picture names `"filter"` among what it does not draw.

use std::sync::Arc;

use frus_core::{Affine, BlendMode, ColorFilter, FillRule, Path, Rect};
use usvg::filter::{ColorInterpolation, ColorMatrixKind, CompositeOperator, Input, Kind};

use crate::{axis_scales, color, group_items, mapped, Effect, Item, Paint};

/// `rect`, in the space `to_root` takes to the picture's, as an outline in the picture's.
fn rect_outline(rect: usvg::NonZeroRect, to_root: Affine) -> Path {
    let rect = Rect::new(rect.x(), rect.y(), rect.width(), rect.height());
    mapped(&Path::rect(rect).with_fill_rule(FillRule::NonZero), to_root)
}

/// **Appends a mask's layers to `items`**, the masked group's content in the picture's
/// coordinates; `to_root` is the masked group's transform. `false` when the mask hides
/// everything — it has nothing in it.
///
/// A mask that is itself masked masks the group again, as the reference renderer does.
pub(crate) fn mask_layers(
    mask: &usvg::Mask,
    to_root: Affine,
    items: &mut Vec<Item>,
    unsupported: &mut Vec<&'static str>,
) -> bool {
    if mask.root().children().is_empty() {
        return false;
    }
    // The mask's content lives in a space of its own, which the masked group's transform
    // takes to the picture's.
    let content: Vec<Item> = group_items(mask.root(), unsupported)
        .into_iter()
        .map(|item| item.mapped(to_root))
        .collect();
    let region = Item::layer(
        Effect::default(),
        Some(rect_outline(mask.rect(), to_root)),
        content.clone(),
    );
    let coverage = match mask.kind() {
        usvg::MaskType::Alpha => vec![region],
        // The content's coverage, then its luminance kept of it: `DstIn` by a layer whose
        // alpha is the luminance multiplies the two.
        usvg::MaskType::Luminance => vec![
            region,
            Item::layer(
                Effect {
                    color: Some(ColorFilter::Matrix(LUMINANCE_TO_ALPHA)),
                    blend: Some(BlendMode::DstIn),
                    ..Effect::default()
                },
                None,
                content,
            ),
        ],
    };
    items.push(Item::layer(
        Effect {
            blend: Some(BlendMode::DstIn),
            untinted: true,
            ..Effect::default()
        },
        None,
        coverage,
    ));
    match mask.mask() {
        Some(inner) => mask_layers(inner, to_root, items, unsupported),
        None => true,
    }
}

/// The luminance of the straight, sRGB-encoded colour as alpha, with the weights the
/// reference renderer uses.
const LUMINANCE_TO_ALPHA: [f32; 20] = [
    0.0, 0.0, 0.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, //
    0.2126, 0.7152, 0.0722, 0.0, 0.0,
];

/// **`source` through `filters`**, one after the other, in the picture's coordinates;
/// `to_root` is the filtered group's transform. `None` when a filter uses a primitive that
/// is not drawn.
pub(crate) fn filtered(
    filters: &[Arc<usvg::filter::Filter>],
    source: Vec<Item>,
    to_root: Affine,
) -> Option<Vec<Item>> {
    filters
        .iter()
        .try_fold(source, |source, filter| apply(filter, source, to_root))
}

fn apply(filter: &usvg::filter::Filter, source: Vec<Item>, to_root: Affine) -> Option<Vec<Item>> {
    let region = filter.rect();
    let mut results: Vec<(&str, Vec<Item>)> = Vec::new();
    for primitive in filter.primitives() {
        let input = |input: &Input| -> Vec<Item> {
            match input {
                Input::SourceGraphic => source.clone(),
                // The source in black, keeping its alpha.
                Input::SourceAlpha => vec![Item::layer(
                    Effect {
                        color: Some(ColorFilter::Matrix(ALPHA_ONLY)),
                        ..Effect::default()
                    },
                    None,
                    source.clone(),
                )],
                // The latest result of that name; an unknown one is the source, as the
                // reference renderer has it.
                Input::Reference(name) => results
                    .iter()
                    .rev()
                    .find(|(result, _)| result == name)
                    .map_or_else(|| source.clone(), |(_, items)| items.clone()),
            }
        };
        // Colours meeting colours in sRGB rather than in linear light: the operations that
        // depend on it are drawn only when the picture asks for sRGB, since frus's layers
        // would otherwise do it in the wrong space.
        let srgb = primitive.color_interpolation() == ColorInterpolation::SRGB;
        let mut output = match primitive.kind() {
            Kind::GaussianBlur(fe) => {
                let input = input(fe.input());
                match blur(fe.std_dev_x().get(), fe.std_dev_y().get(), to_root)? {
                    Some(blur) => vec![Item::layer(
                        Effect {
                            blur: Some(blur),
                            ..Effect::default()
                        },
                        None,
                        input,
                    )],
                    None => input,
                }
            }
            Kind::Offset(fe) => offset(input(fe.input()), fe.dx(), fe.dy(), to_root),
            Kind::Flood(fe) => vec![Item::Shape {
                path: rect_outline(primitive.rect(), to_root),
                paint: Paint::Solid(color(fe.color(), fe.opacity().get())),
                opacity: 1.0,
            }],
            Kind::Merge(fe) => fe.inputs().iter().flat_map(&input).collect(),
            Kind::DropShadow(fe) => {
                let input = input(fe.input());
                // The input's silhouette in the shadow's colour, blurred, then moved; the
                // input over it.
                let shadow = Item::layer(
                    Effect {
                        blur: blur(fe.std_dev_x().get(), fe.std_dev_y().get(), to_root)?,
                        color: Some(ColorFilter::Mode(
                            color(fe.color(), fe.opacity().get()),
                            BlendMode::SrcIn,
                        )),
                        ..Effect::default()
                    },
                    None,
                    input.clone(),
                );
                let mut output = offset(vec![shadow], fe.dx(), fe.dy(), to_root);
                output.extend(input);
                output
            }
            Kind::Composite(fe) => {
                let mode = match fe.operator() {
                    CompositeOperator::Over => None,
                    CompositeOperator::In => Some(BlendMode::SrcIn),
                    CompositeOperator::Out => Some(BlendMode::SrcOut),
                    CompositeOperator::Atop if srgb => Some(BlendMode::SrcAtop),
                    CompositeOperator::Xor if srgb => Some(BlendMode::Xor),
                    CompositeOperator::Lighter if srgb => Some(BlendMode::Plus),
                    _ => return None,
                };
                over(input(fe.input2()), input(fe.input1()), mode)
            }
            Kind::Blend(fe) if fe.mode() == usvg::BlendMode::Normal => {
                over(input(fe.input2()), input(fe.input1()), None)
            }
            Kind::ColorMatrix(fe) => vec![Item::layer(
                Effect {
                    color: Some(ColorFilter::Matrix(color_matrix(fe.kind(), srgb)?)),
                    ..Effect::default()
                },
                None,
                input(fe.input()),
            )],
            _ => return None,
        };
        // A primitive's subregion clips its result; an offset's is its input's, and a
        // flood already fills its own.
        let clipped = !matches!(primitive.kind(), Kind::Offset(_) | Kind::Flood(_));
        if clipped && !same_rect(primitive.rect(), region) {
            output = vec![Item::layer(
                Effect::default(),
                Some(rect_outline(primitive.rect(), to_root)),
                output,
            )];
        }
        results.push((primitive.result(), output));
    }
    // A filter with nothing in it draws nothing, as the reference renderer has it.
    let (_, output) = results.pop().unwrap_or_default();
    // The filter region clips the whole.
    Some(vec![Item::layer(
        Effect::default(),
        Some(rect_outline(region, to_root)),
        output,
    )])
}

/// The source in black, keeping its alpha: `SourceAlpha`.
const ALPHA_ONLY: [f32; 20] = [
    0.0, 0.0, 0.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 1.0, 0.0,
];

fn same_rect(a: usvg::NonZeroRect, b: usvg::NonZeroRect) -> bool {
    let near = |x: f32, y: f32| (x - y).abs() <= 1e-3 * (1.0 + x.abs().max(y.abs()));
    near(a.x(), b.x())
        && near(a.y(), b.y())
        && near(a.width(), b.width())
        && near(a.height(), b.height())
}

/// `top` over `bottom` — or meeting it by `mode` — in a layer of their own, so that the
/// blend meets `bottom` alone.
fn over(bottom: Vec<Item>, top: Vec<Item>, mode: Option<BlendMode>) -> Vec<Item> {
    match mode {
        None => bottom.into_iter().chain(top).collect(),
        Some(mode) => {
            let mut items = bottom;
            items.push(Item::layer(
                Effect {
                    blend: Some(mode),
                    ..Effect::default()
                },
                None,
                top,
            ));
            vec![Item::layer(Effect::default(), None, items)]
        }
    }
}

/// `items` moved by `(dx, dy)` in the filtered group's space.
fn offset(items: Vec<Item>, dx: f32, dy: f32, to_root: Affine) -> Vec<Item> {
    let [a, b, c, d, _, _] = to_root.m;
    let (x, y) = (a * dx + c * dy, b * dx + d * dy);
    if x == 0.0 && y == 0.0 {
        return items;
    }
    let by = Affine::translation(x, y);
    items.into_iter().map(|item| item.mapped(by)).collect()
}

/// A blur's standard deviations in the picture's units, from the filtered group's:
/// `Some(None)` for no blur at all, `None` for one that is not drawn — a blur longer one
/// way than the other, on a group turned or slanted, which frus's layers would blur along
/// the screen's axes and not along the group's.
fn blur(x: f32, y: f32, to_root: Affine) -> Option<Option<(f32, f32)>> {
    let (sx, sy) = axis_scales(to_root);
    // A tiny deviation blurs nothing, as the reference renderer has it.
    let keep = |v: f32| if v < 0.05 { 0.0 } else { v };
    let (x, y) = (keep(x * sx), keep(y * sy));
    if x == 0.0 && y == 0.0 {
        return Some(None);
    }
    if !to_root.is_axis_aligned() && (x - y).abs() > 1e-3 * x.max(y) {
        return None;
    }
    Some(Some((x, y)))
}

/// An `feColorMatrix` as frus's colour matrix, which works on sRGB-encoded values.
///
/// In sRGB, any. In linear light, the default, only one whose colour channels are each
/// left alone or set to a constant — a constant being re-encoded — and whose alpha does not
/// depend on the colour: the shadow-tinting matrices design tools export. A matrix mixing
/// colours in linear light is not drawn.
fn color_matrix(kind: &ColorMatrixKind, srgb: bool) -> Option<[f32; 20]> {
    let mut m: [f32; 20] = match kind {
        ColorMatrixKind::Matrix(values) => values.as_slice().try_into().ok()?,
        ColorMatrixKind::Saturate(s) => {
            let s = s.get().max(0.0);
            [
                0.213 + 0.787 * s,
                0.715 - 0.715 * s,
                0.072 - 0.072 * s,
                0.0,
                0.0,
                0.213 - 0.213 * s,
                0.715 + 0.285 * s,
                0.072 - 0.072 * s,
                0.0,
                0.0,
                0.213 - 0.213 * s,
                0.715 - 0.715 * s,
                0.072 + 0.928 * s,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
            ]
        }
        ColorMatrixKind::HueRotate(degrees) => {
            let (a2, a1) = degrees.to_radians().sin_cos();
            [
                0.213 + 0.787 * a1 - 0.213 * a2,
                0.715 - 0.715 * a1 - 0.715 * a2,
                0.072 - 0.072 * a1 + 0.928 * a2,
                0.0,
                0.0,
                0.213 - 0.213 * a1 + 0.143 * a2,
                0.715 + 0.285 * a1 + 0.140 * a2,
                0.072 - 0.072 * a1 - 0.283 * a2,
                0.0,
                0.0,
                0.213 - 0.213 * a1 - 0.787 * a2,
                0.715 - 0.715 * a1 + 0.715 * a2,
                0.072 + 0.928 * a1 + 0.072 * a2,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
            ]
        }
        ColorMatrixKind::LuminanceToAlpha => [
            0.0, 0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0, 0.0, //
            0.2125, 0.7154, 0.0721, 0.0, 0.0,
        ],
    };
    if srgb {
        return Some(m);
    }
    for channel in 0..3 {
        let row = &mut m[channel * 5..channel * 5 + 5];
        let kept = (0..4).all(|i| row[i] == if i == channel { 1.0 } else { 0.0 }) && row[4] == 0.0;
        let constant = row[..4].iter().all(|&v| v == 0.0);
        if constant {
            row[4] = encode(row[4].clamp(0.0, 1.0));
        } else if !kept {
            return None;
        }
    }
    m[15..18].iter().all(|&v| v == 0.0).then_some(m)
}

/// A linear-light value encoded as sRGB.
fn encode(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}
