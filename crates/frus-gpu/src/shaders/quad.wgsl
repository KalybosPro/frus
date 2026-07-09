// Rendu instancié de rectangles, avec coins arrondis et bordure (SDF).
//
// Un quad unité (coins 0..1) est répété pour chaque instance. Le vertex shader
// place le quad et transmet au fragment la position locale (px depuis le centre)
// et les paramètres de forme ; le fragment évalue la distance signée à un
// rectangle arrondi pour l'anti-aliasing et la bordure.

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) unit_pos: vec2<f32>,
};

struct InstanceInput {
    @location(1) rect: vec4<f32>,          // x, y, width, height (pixels)
    @location(2) fill: vec4<f32>,          // couleur de remplissage
    @location(3) border: vec4<f32>,        // couleur de bordure
    @location(4) params: vec4<f32>,        // radius, border_width, _, _
    @location(5) clip: vec4<f32>,          // x, y, width, height du clip
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_px: vec2<f32>,
    @location(1) @interpolate(flat) half_size: vec2<f32>,
    @location(2) @interpolate(flat) radius: f32,
    @location(3) @interpolate(flat) border_width: f32,
    @location(4) @interpolate(flat) fill: vec4<f32>,
    @location(5) @interpolate(flat) border: vec4<f32>,
    @location(6) frag_px: vec2<f32>,
    @location(7) @interpolate(flat) clip: vec4<f32>,
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
    out.half_size = inst.rect.zw * 0.5;
    out.radius = inst.params.x;
    out.border_width = inst.params.y;
    out.fill = inst.fill;
    out.border = inst.border;
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
    // Découpe : rien hors du rectangle de clip.
    let inside_clip = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );

    // Le rayon ne peut excéder la demi-taille.
    let r = min(in.radius, min(in.half_size.x, in.half_size.y));
    let d = sdf_round_box(in.local_px, in.half_size, r);

    // Couverture anti-aliasée sur ~1px au bord extérieur.
    let alpha = (1.0 - smoothstep(-0.5, 0.5, d)) * inside_clip;

    // Bordure : transition vers la couleur de bordure près du bord (d ∈ [-bw, 0]).
    var color = in.fill;
    if (in.border_width > 0.0) {
        let t = smoothstep(-in.border_width - 0.5, -in.border_width + 0.5, d);
        color = mix(in.fill, in.border, t);
    }

    return vec4<f32>(color.rgb, color.a * alpha);
}
