// The **image filter** pre-pass: one axis of a separable filter over a layer's
// already-rendered texture, into another texture of the same size.
//
// Separable means the two axes are run as two passes. A Gaussian is separable
// exactly — a 2D Gaussian is the product of two 1D ones — and dilate and erode are
// separable because a maximum (or a minimum) over a rectangle is the maximum of the
// per-row maxima. Two passes of `2n+1` taps instead of one of `(2n+1)²` is the
// difference between a blur that costs nothing and one that costs the frame.
//
// The samples are **premultiplied** and **linear**: premultiplied because averaging
// straight colour lets a transparent pixel drag its (meaningless) colour into its
// neighbours, and linear because a blur is an average of light. The layer texture is
// in an sRGB format, so a sample is already linear and a write re-encodes; nothing
// converts here.

struct Params {
    // The texture size in pixels — the tap step is one texel.
    size: vec2<f32>,
    // The axis this pass runs along: (1, 0) or (0, 1).
    dir: vec2<f32>,
    // How far a pixel may pull from, in pixels, along `dir`.
    radius: f32,
    // 0 = blur, 1 = dilate, 2 = erode.
    kind: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var tex: texture_2d<f32>;
@group(0) @binding(2)
var samp: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) unit_pos: vec2<f32>) -> VertexOutput {
    let ndc = vec2<f32>(unit_pos.x * 2.0 - 1.0, 1.0 - unit_pos.y * 2.0);
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = unit_pos;
    return out;
}

// Taps **per side**. The count is fixed and the step scales with the radius, so a
// wide blur costs exactly what a narrow one does. Past roughly this many taps a
// Gaussian is undersampled and bands; below it, it is smooth.
const TAPS: i32 = 12;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / params.size;
    let center = textureSample(tex, samp, in.uv);
    if (params.radius <= 0.0) {
        return center;
    }
    let step = params.radius / f32(TAPS);
    let offset = params.dir * step * texel;

    if (params.kind < 0.5) {
        // Gaussian. The radius is three standard deviations, so tap `i` sits at
        // `3i/TAPS` sigmas whatever the radius is — which makes the weights
        // constants rather than something to recompute.
        var sum = center * 1.0;
        var total = 1.0;
        for (var i: i32 = 1; i <= TAPS; i = i + 1) {
            let d = 3.0 * f32(i) / f32(TAPS);
            let w = exp(-0.5 * d * d);
            let o = offset * f32(i);
            sum = sum + textureSample(tex, samp, in.uv + o) * w;
            sum = sum + textureSample(tex, samp, in.uv - o) * w;
            total = total + 2.0 * w;
        }
        return sum / total;
    }

    if (params.kind < 1.5) {
        // Dilate: the brightest neighbour wins, per channel, alpha included — so a
        // shape grows outwards.
        var acc = center;
        for (var i: i32 = 1; i <= TAPS; i = i + 1) {
            let o = offset * f32(i);
            acc = max(acc, textureSample(tex, samp, in.uv + o));
            acc = max(acc, textureSample(tex, samp, in.uv - o));
        }
        return acc;
    }

    // Erode: the dimmest neighbour wins, so a shape shrinks. Outside the texture the
    // sampler clamps to the edge, which is the right answer here: an eroded shape
    // should not be eaten away by an imaginary transparent border.
    var acc = center;
    for (var i: i32 = 1; i <= TAPS; i = i + 1) {
        let o = offset * f32(i);
        acc = min(acc, textureSample(tex, samp, in.uv + o));
        acc = min(acc, textureSample(tex, samp, in.uv - o));
    }
    return acc;
}
