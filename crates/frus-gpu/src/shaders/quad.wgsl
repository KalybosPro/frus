// Instanced rectangle rendering: rounded corners, border, gradient, shadow (SDF).

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

// The gradients' colour ramps: one row per gradient, sRGB values (milestone 644).
@group(1) @binding(0)
var ramp: texture_2d<f32>;
@group(1) @binding(1)
var ramp_sampler: sampler;

struct VertexInput {
    @location(0) unit_pos: vec2<f32>,
};

struct InstanceInput {
    @location(1) rect: vec4<f32>,     // x, y, width, height (pixels)
    @location(2) color: vec4<f32>,    // the fill, or the gradient's start
    @location(3) color2: vec4<f32>,   // the gradient's end
    @location(4) border: vec4<f32>,   // the border colour
    @location(5) params: vec4<f32>,   // code (kind + 4 tile), border_width, blur, ramp row
    @location(6) gradient: vec4<f32>, // the legacy direction, or a gradient's geometry
    @location(7) clip: vec4<f32>,     // x, y, width, height
    @location(8) radii: vec4<f32>,    // per-corner radii: tl, tr, br, bl
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_px: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) half_size: vec2<f32>,
    @location(3) @interpolate(flat) radii: vec4<f32>,
    @location(4) @interpolate(flat) border_width: f32,
    @location(5) @interpolate(flat) blur: f32,
    @location(6) @interpolate(flat) color: vec4<f32>,
    @location(7) @interpolate(flat) color2: vec4<f32>,
    @location(8) @interpolate(flat) border: vec4<f32>,
    @location(9) @interpolate(flat) gradient: vec4<f32>,
    @location(10) frag_px: vec2<f32>,
    @location(11) @interpolate(flat) clip: vec4<f32>,
    @location(12) @interpolate(flat) shade: vec2<f32>,
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
    out.local_px = (vert.unit_pos - vec2<f32>(0.5, 0.5)) * inst.rect.zw;
    out.uv = vert.unit_pos;
    out.half_size = inst.rect.zw * 0.5;
    out.radii = inst.radii;
    out.border_width = inst.params.y;
    out.blur = inst.params.z;
    out.color = inst.color;
    out.color2 = inst.color2;
    out.border = inst.border;
    out.gradient = inst.gradient;
    out.shade = vec2<f32>(inst.params.x, inst.params.w);
    out.frag_px = pos_px;
    out.clip = inst.clip;
    return out;
}

// Converts an sRGB colour — as authored in the scene — to linear. The target being
// sRGB, the GPU re-encodes linear→sRGB on write, so sending linear reproduces
// exactly the intended colour; encoding twice would wash it out.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lower = c / 12.92;
    let higher = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(higher, lower, c <= vec3<f32>(0.04045));
}

// Signed distance to a centred rounded rectangle, negative inside.
fn sdf_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

// The corner radius matching `p`'s quadrant (centred coordinates, y pointing
// down). radii = (tl, tr, br, bl).
fn corner_radius(p: vec2<f32>, radii: vec4<f32>) -> f32 {
    if (p.x < 0.0) {
        return select(radii.w, radii.x, p.y < 0.0); // left: top → tl, bottom → bl
    }
    return select(radii.z, radii.y, p.y < 0.0);     // right: top → tr, bottom → br
}

// **A gradient's colour at `p`** (pixels from the rectangle's centre), milestone 644:
// `t` along a line, between two circles, or round a centre, tiled, then read from the
// gradient's row of the ramps. `extra` is the focal point, the turn and whether there is a
// focal circle.
fn gradient_fill(code: i32, local: vec2<f32>, g: vec4<f32>, extra: vec4<f32>, row: f32) -> vec4<f32> {
    let kind = code % 4;
    let tile = code / 4;
    // The gradient turned about the centre: the point turned back.
    var p = local;
    let turn = extra.z;
    if (turn != 0.0) {
        let c = cos(turn);
        let s = sin(turn);
        p = vec2<f32>(c * p.x + s * p.y, -s * p.x + c * p.y);
    }
    var t = 0.0;
    var seen = 1.0;
    if (kind == 1) {
        let d = g.zw - g.xy;
        t = dot(p - g.xy, d) / max(dot(d, d), 1e-6);
    } else if (kind == 2) {
        if (extra.w < 0.5) {
            t = length(p - g.xy) / max(g.z, 1e-6);
        } else {
            // Between the focal circle (centre extra.xy, radius g.w) and the outer one
            // (centre g.xy, radius g.z): the largest t whose circle passes through p.
            let c0 = extra.xy;
            let r0 = g.w;
            let cd = g.xy - c0;
            let dr = g.z - r0;
            let pd = p - c0;
            let a = dot(cd, cd) - dr * dr;
            let b = dot(pd, cd) + r0 * dr;
            let c = dot(pd, pd) - r0 * r0;
            if (abs(a) < 1e-4) {
                t = c / max(2.0 * b, 1e-6);
            } else {
                let disc = b * b - a * c;
                if (disc < 0.0) {
                    seen = 0.0;
                } else {
                    let root = sqrt(disc);
                    t = (b + root) / a;
                    if (r0 + t * dr < 0.0) {
                        t = (b - root) / a;
                    }
                }
            }
        }
    } else {
        let v = p - g.xy;
        var angle = atan2(v.y, v.x);
        if (angle < 0.0) {
            angle = angle + 6.28318530718;
        }
        t = (angle - g.z) / max(g.w - g.z, 1e-6);
    }
    // Past the ends: clamped, repeated, mirrored, or nothing.
    if (tile == 0) {
        t = clamp(t, 0.0, 1.0);
    } else if (tile == 1) {
        t = fract(t);
    } else if (tile == 2) {
        let m = t - 2.0 * floor(t * 0.5);
        t = select(2.0 - m, m, m <= 1.0);
    } else if (t < 0.0 || t > 1.0) {
        seen = 0.0;
    }
    let size = vec2<f32>(textureDimensions(ramp));
    let uv = vec2<f32>((t * (size.x - 1.0) + 0.5) / size.x, (row + 0.5) / size.y);
    let color = textureSampleLevel(ramp, ramp_sampler, uv, 0.0);
    return vec4<f32>(color.rgb, color.a * seen);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Clipping.
    let inside_clip = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );

    let r = min(corner_radius(in.local_px, in.radii), min(in.half_size.x, in.half_size.y));
    let d = sdf_round_box(in.local_px, in.half_size, r);

    // A hard edge (half a pixel either side of it), or, for a shadow, a soft one that fades
    // **inside** the quad: a shadow's rect already reaches its blur beyond the shape, so
    // the fade runs from the shape's edge less the blur, at full strength, out to the
    // quad's edge, at nothing. Ramping across the quad's edge instead left the outer half
    // of every shadow unrasterised, and cut it off at half its strength (milestone 606).
    var alpha = 1.0 - smoothstep(-0.5, 0.5, d);
    let code = i32(round(in.shade.x));
    if (in.blur > 0.0) {
        alpha = 1.0 - smoothstep(-2.0 * in.blur, 0.0, d);
        // The blur style (milestone 645), read where a flat fill leaves the ramp row
        // free. The shape's own edge runs `blur` inside the quad's.
        let style = i32(round(in.shade.y));
        let inside = 1.0 - smoothstep(-0.5, 0.5, d + in.blur);
        if (code == 0 && style == 1) {
            alpha = max(alpha, inside);
        } else if (code == 0 && style == 2) {
            alpha = alpha * (1.0 - inside);
        } else if (code == 0 && style == 3) {
            alpha = alpha * inside;
        }
    }
    alpha = alpha * inside_clip;

    // The fill: a gradient from the ramps, or the legacy two-colour fade (solid when
    // dir = 0 and color2 = color).
    var fill: vec4<f32>;
    if (code == 0) {
        let t = clamp(dot(in.uv - vec2<f32>(0.5, 0.5), in.gradient.xy) + 0.5, 0.0, 1.0);
        fill = mix(in.color, in.color2, t);
    } else {
        fill = gradient_fill(code, in.local_px, in.gradient, in.color2, in.shade.y);
    }

    // The border: a ring along the edge.
    if (in.border_width > 0.0) {
        let bt = smoothstep(-in.border_width - 0.5, -in.border_width + 0.5, d);
        fill = mix(fill, in.border, bt);
    }

    return vec4<f32>(srgb_to_linear(fill.rgb), fill.a * alpha);
}
