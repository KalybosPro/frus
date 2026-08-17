//! What a translucent token actually paints.
//!
//! The framework's disabled rule is written in the reference's numbers — a container at
//! 12 % of `on_surface`, its content at 38 %. Those numbers are a *design* language, and
//! a design language assumes an alpha blend in the space the colours are written in:
//! sRGB. Ours does not blend there. The render target is `Rgba8UnormSrgb`, so the
//! hardware decodes the destination to linear, blends, and re-encodes — which is the
//! physically correct thing to do and the normal wgpu pipeline, but it makes a given
//! alpha land much higher up the tone scale than the same alpha does in the reference.
//!
//! In the dark scheme the gap is not subtle: a 12 % wash of `on_surface` is meant to
//! read around tone 24 and paints at about tone 38 — roughly what 33 % would give in
//! sRGB. Every disabled control in a dark app is therefore louder than intended, which
//! is the thread behind a run of device reports (milestones 324, 325, 328): a live
//! outline that could not be told from a disabled one, and a live fill quieter than the
//! disabled fill beside it.
//!
//! This test does not assert that the current behaviour is right. It **pins** it, so
//! that changing the blend space, or pre-compositing the disabled tokens, is a
//! deliberate and visible change rather than a silent one. When it fails, read
//! `docs/milestone-328.md` before blessing it.

use frus_test::render_widget;
use frus_widgets::{Container, Theme, Widget};

/// sRGB → linear, the transfer function an `Rgba8UnormSrgb` target applies.
fn to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// A channel blended the way the hardware does it: in linear light.
fn linear_blend(src: f32, dst: f32, alpha: f32) -> u8 {
    let v = to_srgb(alpha * to_linear(src) + (1.0 - alpha) * to_linear(dst));
    (v * 255.0).round() as u8
}

/// The same blend done in sRGB, which is what the reference's opacity tokens assume.
fn srgb_blend(src: f32, dst: f32, alpha: f32) -> u8 {
    ((alpha * src + (1.0 - alpha) * dst) * 255.0).round() as u8
}

#[test]
fn a_translucent_token_blends_in_linear_light_not_in_srgb() {
    let theme = Theme::dark();
    let alpha = frus_widgets::DISABLED_CONTAINER_OPACITY;
    let wash = theme.scheme.on_surface.fade(alpha);
    // A patch of the disabled wash, on the theme's own background, with nothing else in
    // the frame to composite against.
    let root: Container<()> = Container::new()
        .color(theme.background)
        .child(Container::<()>::new().width(80.0).height(80.0).color(wash));
    let shot = match render_widget(&root as &dyn Widget<()>, 120, 120, &theme) {
        Some(shot) => shot,
        // No GPU in this environment: the claim is about the pipeline, and without one
        // there is no pipeline to ask.
        None => return,
    };
    let painted = shot.pixel(40, 40);

    let (src, dst) = (theme.scheme.on_surface, theme.background);
    let linear = [
        linear_blend(src.r, dst.r, alpha),
        linear_blend(src.g, dst.g, alpha),
        linear_blend(src.b, dst.b, alpha),
    ];
    let srgb = [
        srgb_blend(src.r, dst.r, alpha),
        srgb_blend(src.g, dst.g, alpha),
        srgb_blend(src.b, dst.b, alpha),
    ];

    for (i, channel) in ["r", "g", "b"].iter().enumerate() {
        assert!(
            painted[i].abs_diff(linear[i]) <= 2,
            "{channel}: painted {} is not the linear blend {} — the blend space changed, \
             which repaints every disabled control in the framework. Read \
             docs/milestone-328.md before blessing this.",
            painted[i],
            linear[i]
        );
    }
    // And the distance from the number the token was written for. This is the finding,
    // not an incidental: an eighth of the way to white is landing a third of the way.
    assert!(
        painted[0] as i32 - srgb[0] as i32 >= 30,
        "the two blend spaces have converged (painted {}, sRGB model {}) — if the \
         pipeline now blends the way the opacity tokens assume, this test has served \
         its purpose and the disabled guards can be tightened",
        painted[0],
        srgb[0]
    );
}
