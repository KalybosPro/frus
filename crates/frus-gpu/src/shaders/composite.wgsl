// **Layer** compositing: a full-screen quad samples the layer's already-rendered
// texture and recomposes it at the group opacity, with clipping. The sample is
// already linear, the texture being sRGB, so nothing is converted back.

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

@group(1) @binding(0)
var tex: texture_2d<f32>;
@group(1) @binding(1)
var samp: sampler;
// The clip mask (ClipPath): the path rendered in white; solid white otherwise.
@group(1) @binding(2)
var mask: texture_2d<f32>;

struct VertexInput {
    @location(0) unit_pos: vec2<f32>,
};

struct InstanceInput {
    @location(1) clip: vec4<f32>,    // x, y, width, height (px)
    @location(2) inv_lin: vec4<f32>, // affine inverse, linear part: ia, ib, ic, id
    @location(3) inv_tr_op: vec4<f32>, // ie, if, opacity, _
    @location(4) shape: vec4<f32>,   // kind (0=rect,1=rrect,2=oval), _, _, _
    @location(5) radii: vec4<f32>,   // rrect radii: tl, tr, br, bl
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) frag_px: vec2<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) opacity: f32,
    @location(4) @interpolate(flat) inv_lin: vec4<f32>,
    @location(5) @interpolate(flat) inv_tr: vec2<f32>,
    @location(6) @interpolate(flat) shape: vec4<f32>,
    @location(7) @interpolate(flat) radii: vec4<f32>,
};

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    // The unit quad covers the whole screen, and the layer texture is the size of
    // the surface, so uv is simply the normalised position.
    let ndc = vec2<f32>(vert.unit_pos.x * 2.0 - 1.0, 1.0 - vert.unit_pos.y * 2.0);
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = vert.unit_pos;
    out.frag_px = vert.unit_pos * viewport.size;
    out.clip = inst.clip;
    out.opacity = inst.inv_tr_op.z;
    out.inv_lin = inst.inv_lin;
    out.inv_tr = inst.inv_tr_op.xy;
    out.shape = inst.shape;
    out.radii = inst.radii;
    return out;
}

// Signed distance to a rounded rectangle centred at the origin, of half-size
// `half` and radius `r`: negative inside, positive outside. The basis of rounded
// corners.
fn sd_rounded_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// The corner radius matching `p`'s quadrant (centred, y pointing down).
// radii = (tl, tr, br, bl).
fn corner_radius(p: vec2<f32>, radii: vec4<f32>) -> f32 {
    if (p.x < 0.0) {
        return select(radii.w, radii.x, p.y < 0.0); // left: top → tl, bottom → bl
    }
    return select(radii.z, radii.y, p.y < 0.0);     // right: top → tr, bottom → br
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // The layer transform: the texture holds the content **flat** — untransformed —
    // at its screen position. To paint the layer transformed by M we sample at the
    // **counter-transformed** position M⁻¹(p), so screen pixel p receives the content
    // that, transformed by M, lands on p. With `inv_lin = ia,ib,ic,id` and
    // `inv_tr = ie,if`: src = (ia·x + ic·y + ie, ib·x + id·y + if).
    let p = in.frag_px;
    let src_px = vec2<f32>(
        in.inv_lin.x * p.x + in.inv_lin.z * p.y + in.inv_tr.x,
        in.inv_lin.y * p.x + in.inv_lin.w * p.y + in.inv_tr.y,
    );
    let src_uv = src_px / viewport.size;

    // Outside the texture after counter-rotation: nothing, a transparent edge.
    let in_bounds = f32(
        src_uv.x >= 0.0 && src_uv.x <= 1.0 && src_uv.y >= 0.0 && src_uv.y <= 1.0
    );
    // Clip coverage, tested on the screen pixel and **inscribed** in `clip`: a hard
    // rectangle (kind 0), rounded corners (kind 1) or an ellipse (kind 2). The curved
    // shapes are antialiased over roughly 1 px through the signed distance.
    let center = in.clip.xy + in.clip.zw * 0.5;
    let half = in.clip.zw * 0.5;
    let q = in.frag_px - center;
    let kind = in.shape.x;
    // The rectangular test, shared by a bare rect and a path's **bounds**.
    let in_rect = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );
    var clip_cov: f32;
    if (kind < 0.5) {
        clip_cov = in_rect; // a hard rectangle, the original behaviour
    } else if (kind < 1.5) {
        // Rounded corners, **per corner**: the quadrant's radius, capped at half the
        // smaller dimension.
        let r = min(corner_radius(q, in.radii), min(half.x, half.y));
        let d = sd_rounded_box(q, half, r);
        clip_cov = 1.0 - smoothstep(-0.5, 0.5, d);
    } else if (kind < 2.5) {
        // The inscribed ellipse: an approximate, gradient-normalised edge distance.
        let e = q / max(half, vec2<f32>(1.0, 1.0));
        let g = length(e);
        let d = (g - 1.0) * min(half.x, half.y);
        clip_cov = 1.0 - smoothstep(-0.5, 0.5, d);
    } else {
        clip_cov = in_rect; // a path: rectangular bounds; the shape comes from the mask
    }
    // The coverage mask: solid white outside ClipPath → a neutral multiplication.
    let mask_a = textureSample(mask, samp, in.uv).a;
    let inside_clip = clip_cov * mask_a;
    let sample = textureSample(tex, samp, src_uv);
    // Straight alpha × the group opacity: the (SrcAlpha, 1-SrcAlpha) blend performs
    // the correct "over", with no double-blending inside the layer.
    return vec4<f32>(sample.rgb, sample.a * in.opacity * inside_clip * in_bounds);
}
