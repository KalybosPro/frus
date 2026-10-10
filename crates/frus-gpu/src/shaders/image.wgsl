// Image rendering: instanced quads sampling a texture — one pass per image, the
// texture being bound in group 1. Each instance carries its destination rectangle,
// its UV sub-region (the crop), its tint and its clip.

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

@group(1) @binding(0)
var tex: texture_2d<f32>;
@group(1) @binding(1)
var samp: sampler;

struct VertexInput {
    @location(0) unit_pos: vec2<f32>,
};

struct InstanceInput {
    @location(1) rect: vec4<f32>, // x, y, width, height (px)
    @location(2) uv: vec4<f32>,   // x, y, width, height (0..1)
    @location(3) tint: vec4<f32>, // sRGB, multiplied in
    @location(4) clip: vec4<f32>, // x, y, width, height (px)
    @location(5) region: vec4<f32>, // low x, low y, high x, high y (0..1)
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) tint: vec4<f32>,
    @location(2) frag_px: vec2<f32>,
    @location(3) @interpolate(flat) clip: vec4<f32>,
    @location(4) @interpolate(flat) region: vec4<f32>,
};

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    let pos_px = inst.rect.xy + vert.unit_pos * inst.rect.zw;
    let ndc = vec2<f32>(
        pos_px.x / viewport.size.x * 2.0 - 1.0,
        1.0 - pos_px.y / viewport.size.y * 2.0,
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = inst.uv.xy + vert.unit_pos * inst.uv.zw;
    out.tint = inst.tint;
    out.frag_px = pos_px;
    out.clip = inst.clip;
    out.region = inst.region;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lower = c / 12.92;
    let higher = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(higher, lower, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let inside_clip = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );
    // The texture is in an sRGB format, so the sample is already linear. The tint,
    // an authored sRGB colour, is linearised before being multiplied in.
    //
    // Never past the region's outer texel centres (milestone 645). For a strict draw
    // the region is the part drawn, so a filtered sample does not reach the texels
    // around it — a slice of a frame stretched would otherwise bleed its neighbours
    // along its edges. Otherwise it is the whole texture: the edge clamp it always had.
    let half = vec2<f32>(0.5) / vec2<f32>(textureDimensions(tex));
    let lo = in.region.xy + half;
    let hi = in.region.zw - half;
    // A region narrower than a texel samples its middle.
    let mid = (lo + hi) * 0.5;
    let uv = clamp(in.uv, min(lo, mid), max(hi, mid));
    let sample = textureSample(tex, samp, uv);
    let rgb = sample.rgb * srgb_to_linear(in.tint.rgb);
    let alpha = sample.a * in.tint.a * inside_clip;
    return vec4<f32>(rgb, alpha);
}
