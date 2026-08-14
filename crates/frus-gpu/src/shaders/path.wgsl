// Rendering of tessellated vector paths: indexed triangles, each vertex carrying
// its colour (sRGB), its gradient and its clip rectangle. The geometry, fill or
// stroke, is produced on the CPU by lyon; this shader projects, fades and clips.
//
// The gradient is described, not baked: every vertex of a path carries the same
// description, and the fragment resolves it from the pixel's own position. Baking it
// per vertex would make the fade a function of the tessellation — fine for a straight
// fade, which is affine, and wrong for a radial one.

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) pos: vec2<f32>,       // pixels: logical space scaled by the DPI
    @location(1) color: vec4<f32>,     // sRGB, the fill or the gradient's start
    @location(2) color2: vec4<f32>,    // the gradient's end; == color when flat
    @location(3) geometry: vec4<f32>,  // (from, to), or (centre, radii) if radial
    @location(4) kind: vec2<f32>,      // (0 flat / 1 straight / 2 radial, inner)
    @location(5) clip: vec4<f32>,      // x, y, width, height
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
    @location(1) @interpolate(flat) color2: vec4<f32>,
    @location(2) @interpolate(flat) geometry: vec4<f32>,
    @location(3) @interpolate(flat) kind: vec2<f32>,
    @location(4) @interpolate(flat) clip: vec4<f32>,
    @location(5) frag_px: vec2<f32>,
};

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    let ndc = vec2<f32>(
        v.pos.x / viewport.size.x * 2.0 - 1.0,
        1.0 - v.pos.y / viewport.size.y * 2.0,
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = v.color;
    out.color2 = v.color2;
    out.geometry = v.geometry;
    out.kind = v.kind;
    out.clip = v.clip;
    out.frag_px = v.pos;
    return out;
}

// sRGB → linear: an sRGB target re-encodes on write, so we send linear to
// reproduce the authored colour exactly, as quad.wgsl does.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lower = c / 12.92;
    let higher = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(higher, lower, c <= vec3<f32>(0.04045));
}

// How far through the fade this pixel is, before clamping.
fn gradient_t(kind: vec2<f32>, geometry: vec4<f32>, p: vec2<f32>) -> f32 {
    if (kind.x < 0.5) {
        return 0.0;
    }
    if (kind.x < 1.5) {
        let axis = geometry.zw - geometry.xy;
        let len2 = dot(axis, axis);
        if (len2 <= 0.0) {
            return 0.0;
        }
        return dot(p - geometry.xy, axis) / len2;
    }
    // Radial: the distance to the centre measured in radii, so the far end of the
    // fade is the ellipse itself whichever way the boundary curves.
    let radii = max(geometry.zw, vec2<f32>(1e-6, 1e-6));
    let d = length((p - geometry.xy) / radii);
    let inner = min(kind.y, 0.999);
    return (d - inner) / (1.0 - inner);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let inside_clip = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );
    let t = gradient_t(in.kind, in.geometry, in.frag_px);
    let color = mix(in.color, in.color2, clamp(t, 0.0, 1.0));
    return vec4<f32>(srgb_to_linear(color.rgb), color.a * inside_clip);
}
