// Rendu instancié de rectangles : coins arrondis, bordure, dégradé, ombre (SDF).

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
    @location(2) color: vec4<f32>,    // remplissage (début du dégradé)
    @location(3) color2: vec4<f32>,   // fin du dégradé
    @location(4) border: vec4<f32>,   // couleur de bordure
    @location(5) params: vec4<f32>,   // radius, border_width, blur, _
    @location(6) gradient: vec4<f32>, // dir.x, dir.y, _, _
    @location(7) clip: vec4<f32>,     // x, y, width, height
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_px: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) half_size: vec2<f32>,
    @location(3) @interpolate(flat) radius: f32,
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
    out.radius = inst.params.x;
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

// Distance signée à un rectangle arrondi centré (négative à l'intérieur).
fn sdf_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Découpe.
    let inside_clip = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );

    let r = min(in.radius, min(in.half_size.x, in.half_size.y));
    let d = sdf_round_box(in.local_px, in.half_size, r);

    // Bord net (0.5px) ou doux (rayon de flou) pour les ombres.
    let softness = max(in.blur, 0.5);
    let alpha = (1.0 - smoothstep(-softness, softness, d)) * inside_clip;

    // Remplissage : dégradé linéaire (uni si dir = 0 et color2 = color).
    let t = clamp(dot(in.uv - vec2<f32>(0.5, 0.5), in.gradient) + 0.5, 0.0, 1.0);
    var fill = mix(in.color, in.color2, t);

    // Bordure : anneau près du bord.
    if (in.border_width > 0.0) {
        let bt = smoothstep(-in.border_width - 0.5, -in.border_width + 0.5, d);
        fill = mix(fill, in.border, bt);
    }

    return vec4<f32>(fill.rgb, fill.a * alpha);
}
