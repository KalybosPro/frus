// Mipmap generation (milestone 743): one pass per level, each texel of the level the
// box-filtered average of the texels of the level above it that it covers.
//
// The texture is in an sRGB format: a load decodes to linear and the write encodes back,
// so the average is taken in linear light, where half black and half white is the grey
// the eye sees from afar. Colours are averaged premultiplied by their alpha, so the
// colour of a transparent texel never bleeds into its opaque neighbours.
//
// A level is half the one above, rounded down. An even span averages pairs; an odd one
// (2d + 1 texels down to d) covers three texels per texel with the exact box weights
// (d - x, d, x + 1) / (2d + 1), so no texel is dropped. `axis_taps` in `mipmap.rs` is the
// same arithmetic, tested on the CPU.

@group(0) @binding(0)
var src: texture_2d<f32>;

// One triangle that covers the whole target.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

struct Taps {
    first: i32,
    weights: vec3<f32>,
};

fn axis_taps(span: i32, x: i32) -> Taps {
    if span <= 1 {
        return Taps(0, vec3<f32>(1.0, 0.0, 0.0));
    }
    if span % 2 == 0 {
        return Taps(2 * x, vec3<f32>(0.5, 0.5, 0.0));
    }
    let d = f32(span / 2);
    let xf = f32(x);
    let n = 2.0 * d + 1.0;
    return Taps(2 * x, vec3<f32>((d - xf) / n, d / n, (xf + 1.0) / n));
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(src));
    let p = vec2<i32>(floor(pos.xy));
    let tx = axis_taps(size.x, p.x);
    let ty = axis_taps(size.y, p.y);
    var sum = vec4<f32>(0.0);
    for (var j = 0; j < 3; j++) {
        for (var i = 0; i < 3; i++) {
            let w = tx.weights[i] * ty.weights[j];
            if w > 0.0 {
                let at = min(vec2<i32>(tx.first + i, ty.first + j), size - vec2<i32>(1));
                let c = textureLoad(src, at, 0);
                sum += w * vec4<f32>(c.rgb * c.a, c.a);
            }
        }
    }
    if sum.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(sum.rgb / sum.a, sum.a);
}
