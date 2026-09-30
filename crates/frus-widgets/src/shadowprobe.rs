//! Reading a surface's height back out of a frame (milestone 606): what the tests of every
//! widget that casts a shadow share, so each asks the same question the same way.
//!
//! A surface of any height casts **three** shadows, in the order
//! [`BoxShadow::for_elevation`](frus_core::BoxShadow::for_elevation) gives them. Their three
//! blurs together name the height: no two heights of the reference's table share them.

use frus_core::{BoxShadow, Color, Primitive};

/// Every shadow in `primitives`, innermost layers included: `(blur, colour)`, in paint order.
pub(crate) fn shadows(primitives: &[Primitive]) -> Vec<(f32, Color)> {
    fn walk(primitives: &[Primitive], out: &mut Vec<(f32, Color)>) {
        for p in primitives {
            match p {
                Primitive::Rect { blur, color, .. } if *blur > 0.0 => out.push((*blur, *color)),
                Primitive::Layer { primitives, .. } => walk(primitives, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(primitives, &mut out);
    out
}

/// How many surfaces cast a shadow: three shadows each.
pub(crate) fn casts(primitives: &[Primitive]) -> usize {
    shadows(primitives).len() / 3
}

/// The height of the first surface that casts a shadow, to a twentieth; `0.0` when none
/// does.
pub(crate) fn height(primitives: &[Primitive]) -> f32 {
    heights(primitives).first().copied().unwrap_or(0.0)
}

/// The height of every surface that casts a shadow, in paint order, to a twentieth.
pub(crate) fn heights(primitives: &[Primitive]) -> Vec<f32> {
    let found = shadows(primitives);
    found
        .as_chunks::<3>()
        .0
        .iter()
        .map(|[a, b, c]| from_blurs([a.0, b.0, c.0]))
        .collect()
}

/// The height whose three shadows are blurred as `blurs` are.
fn from_blurs([a, b, c]: [f32; 3]) -> f32 {
    (1..=480)
        .map(|step| step as f32 / 20.0)
        .min_by(|x, y| {
            let off = |e: f32| {
                let [p, q, r] = BoxShadow::for_elevation(e, Color::BLACK);
                (p.blur - a).abs() + (q.blur - b).abs() + (r.blur - c).abs()
            };
            off(*x).total_cmp(&off(*y))
        })
        .unwrap_or(0.0)
}

/// The colour the first surface's shadow was cast in: its first shadow carries a fifth of
/// that colour's strength.
pub(crate) fn colour(primitives: &[Primitive]) -> Option<Color> {
    shadows(primitives)
        .first()
        .map(|(_, c)| c.with_alpha(((c.a / 0.2).min(1.0) * 1000.0).round() / 1000.0))
}
