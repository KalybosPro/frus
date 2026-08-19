// **Layer** compositing: a full-screen quad samples the layer's already-rendered
// texture and recomposes it at the group opacity, with clipping. The sample is
// already linear, the texture being sRGB, so nothing is converted back.
//
// A layer may also carry a **colour filter** and a **mask**, and those are applied
// here, between the sample and the composite. Both are defined on sRGB-encoded,
// straight-alpha values — the space colours are authored in — so the fragment
// un-premultiplies, converts to sRGB, filters, and puts it all back. With no filter
// that round trip is skipped entirely and the output is what it always was.
//
// The third effect, the image filter, is not here: it needs a pixel's neighbours,
// so it runs as a separate pre-pass over the layer texture before this one samples
// it (see `fx.wgsl`).

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

// The layer's colour filter and shader mask. Per layer rather than per instance
// because a colour matrix alone is twenty numbers, and vertex attributes are a
// scarcer resource than a uniform slice: each layer already has its own bind group
// for its texture, so this rides along at no extra cost.
struct FilterParams {
    // (colour kind, mask kind, colour blend, mask blend). Colour kind: 0 none,
    // 1 matrix, 2 a colour blended in. Mask kind: 0 none, 1 linear, 2 radial.
    flags: vec4<f32>,
    // The colour matrix, row by row, then the constant column.
    m0: vec4<f32>,
    m1: vec4<f32>,
    m2: vec4<f32>,
    m3: vec4<f32>,
    m4: vec4<f32>,
    // The colour of a `Mode` colour fx.
    mode_color: vec4<f32>,
    // Linear: (from.x, from.y, to.x, to.y). Radial: (center.x, center.y, radius, _).
    mask_geom: vec4<f32>,
    mask_c0: vec4<f32>,
    mask_c1: vec4<f32>,
};

@group(1) @binding(3)
var<uniform> fx: FilterParams;

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

// linear -> sRGB. The inverse of what the sampler already did for us; needed
// because the filters below are defined on the encoded values, not on light.
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lower = c * 12.92;
    let higher = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, c <= vec3<f32>(0.0031308));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lower = c / 12.92;
    let higher = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(higher, lower, c <= vec3<f32>(0.04045));
}

// One channel of a separable blend, on **straight** values.
fn separable(sc: f32, dc: f32, mode: u32) -> f32 {
    if (mode == 13u) {                      // Multiply
        return sc * dc;
    } else if (mode == 14u) {               // Screen
        return sc + dc - sc * dc;
    } else if (mode == 15u) {               // Overlay: Hard-light with the roles swapped
        if (dc <= 0.5) {
            return 2.0 * sc * dc;
        }
        return 1.0 - 2.0 * (1.0 - sc) * (1.0 - dc);
    } else if (mode == 16u) {               // Darken
        return min(sc, dc);
    }
    return max(sc, dc);                     // 17 Lighten
}

// `src` over `dst` under `mode`, both **premultiplied**. The Porter-Duff modes are
// linear in the premultiplied values, which is why they are one line each; the
// separable modes are not, so they are unpremultiplied first and the standard
// composition is spelled out.
fn blend(src: vec4<f32>, dst: vec4<f32>, mode: u32) -> vec4<f32> {
    let sa = src.a;
    let da = dst.a;
    if (mode == 0u) { return src; }                                  // Src
    if (mode == 1u) { return dst; }                                  // Dst
    if (mode == 2u) { return src + dst * (1.0 - sa); }               // SrcOver
    if (mode == 3u) { return dst + src * (1.0 - da); }               // DstOver
    if (mode == 4u) { return src * da; }                             // SrcIn
    if (mode == 5u) { return dst * sa; }                             // DstIn
    if (mode == 6u) { return src * (1.0 - da); }                     // SrcOut
    if (mode == 7u) { return dst * (1.0 - sa); }                     // DstOut
    if (mode == 8u) { return src * da + dst * (1.0 - sa); }          // SrcAtop
    if (mode == 9u) { return dst * sa + src * (1.0 - da); }          // DstAtop
    if (mode == 10u) {                                               // Xor
        return src * (1.0 - da) + dst * (1.0 - sa);
    }
    if (mode == 11u) {                                               // Plus
        return min(src + dst, vec4<f32>(1.0));
    }
    if (mode == 12u) {                                               // Modulate
        return src * dst;
    }
    // The separable set: unpremultiply, blend per channel, recompose.
    let sc = select(src.rgb / sa, vec3<f32>(0.0), sa <= 0.0);
    let dc = select(dst.rgb / da, vec3<f32>(0.0), da <= 0.0);
    let b = vec3<f32>(
        separable(sc.r, dc.r, mode),
        separable(sc.g, dc.g, mode),
        separable(sc.b, dc.b, mode),
    );
    let rgb = (1.0 - da) * src.rgb + (1.0 - sa) * dst.rgb + sa * da * b;
    return vec4<f32>(rgb, sa + da - sa * da);
}

// The mask's colour at a screen pixel: a two-stop fade, straight and sRGB-encoded
// like the colours it was authored from.
fn mask_color(p: vec2<f32>, kind: f32) -> vec4<f32> {
    var t: f32;
    if (kind < 1.5) {
        // Linear: the projection of `p` onto the from-to segment, clamped.
        let start = fx.mask_geom.xy;
        let axis = fx.mask_geom.zw - start;
        let len2 = dot(axis, axis);
        t = select(dot(p - start, axis) / len2, 0.0, len2 <= 0.0);
    } else {
        // Radial: the distance from the centre, over the radius.
        let center = fx.mask_geom.xy;
        let radius = fx.mask_geom.z;
        t = select(length(p - center) / radius, 1.0, radius <= 0.0);
    }
    return mix(fx.mask_c0, fx.mask_c1, clamp(t, 0.0, 1.0));
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
    // The group opacity and every coverage term, as one factor.
    let cover = in.opacity * inside_clip * in_bounds;
    let color_kind = fx.flags.x;
    let mask_kind = fx.flags.y;
    if (color_kind < 0.5 && mask_kind < 0.5) {
        // The overwhelmingly common case: no filter at all.
        //
        // The layer texture is **premultiplied** — it was painted over a transparent
        // target, which is what the (SrcAlpha, 1-SrcAlpha) blend leaves behind — so
        // scaling both halves by the coverage and letting the target blend with
        // (One, 1-SrcAlpha) is the correct "over", with no double-blending inside the
        // layer and none at its edge either.
        return vec4<f32>(sample.rgb * cover, sample.a * cover);
    }

    // The layer texture holds premultiplied linear light. The filters are defined on
    // straight, sRGB-encoded values, so we go there and come back.
    let a = sample.a;
    let straight = select(sample.rgb / a, vec3<f32>(0.0), a <= 0.0);
    var c = vec4<f32>(linear_to_srgb(straight), a);

    if (color_kind > 0.5 && color_kind < 1.5) {
        // The 5x4 matrix, on (r, g, b, a, 1).
        let v = c;
        c = vec4<f32>(
            dot(fx.m0, v) + fx.m4.x,
            dot(fx.m1, v) + fx.m4.y,
            dot(fx.m2, v) + fx.m4.z,
            dot(fx.m3, v) + fx.m4.w,
        );
        c = clamp(c, vec4<f32>(0.0), vec4<f32>(1.0));
    } else if (color_kind > 1.5) {
        // A colour blended into every pixel.
        let src = vec4<f32>(fx.mode_color.rgb * fx.mode_color.a, fx.mode_color.a);
        let dst = vec4<f32>(c.rgb * c.a, c.a);
        let out = blend(src, dst, u32(fx.flags.z));
        c = vec4<f32>(select(out.rgb / out.a, vec3<f32>(0.0), out.a <= 0.0), out.a);
    }

    if (mask_kind > 0.5) {
        let m = mask_color(in.frag_px, mask_kind);
        let src = vec4<f32>(m.rgb * m.a, m.a);
        let dst = vec4<f32>(c.rgb * c.a, c.a);
        let out = blend(src, dst, u32(fx.flags.w));
        c = vec4<f32>(select(out.rgb / out.a, vec3<f32>(0.0), out.a <= 0.0), out.a);
    }

    // Back to premultiplied linear, in the same convention as the fast path above.
    let lin = srgb_to_linear(clamp(c.rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
    return vec4<f32>(lin * c.a * cover, c.a * cover);
}
