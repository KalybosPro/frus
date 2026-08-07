// Instanced rectangle rendering: rounded corners, border, gradient, shadow (SDF).

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) unit_pos: vec2<f32>,
};

struct InstanceInput {
    @location(1) rect: vec4<f32>,     // x, y, width, height (pixels)
    @location(2) color: vec4<f32>,    // the fill, or the gradient's start
    @location(3) color2: vec4<f32>,   // the gradient's end
    @location(4) border: vec4<f32>,   // the border colour
    @location(5) params: vec4<f32>,   // _, border_width, blur, _
    @location(6) gradient: vec4<f32>, // dir.x, dir.y, _, _
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
    @location(9) @interpolate(flat) gradient: vec2<f32>,
    @location(10) frag_px: vec2<f32>,
    @location(11) @interpolate(flat) clip: vec4<f32>,
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
    out.gradient = inst.gradient.xy;
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

    // A hard edge (0.5px), or a soft one of the blur radius for shadows.
    let softness = max(in.blur, 0.5);
    let alpha = (1.0 - smoothstep(-softness, softness, d)) * inside_clip;

    // The fill: a linear gradient, solid when dir = 0 and color2 = color.
    let t = clamp(dot(in.uv - vec2<f32>(0.5, 0.5), in.gradient) + 0.5, 0.0, 1.0);
    var fill = mix(in.color, in.color2, t);

    // The border: a ring along the edge.
    if (in.border_width > 0.0) {
        let bt = smoothstep(-in.border_width - 0.5, -in.border_width + 0.5, d);
        fill = mix(fill, in.border, bt);
    }

    return vec4<f32>(srgb_to_linear(fill.rgb), fill.a * alpha);
}
