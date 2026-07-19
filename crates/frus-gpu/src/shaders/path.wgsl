// Rendu de chemins vectoriels tessellisés : triangles indexés, chaque sommet
// portant sa couleur (sRGB) et son rectangle de découpe. La géométrie (fill /
// stroke) est produite côté CPU par lyon ; ce shader ne fait que projeter et
// découper.

struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) pos: vec2<f32>,   // pixels (espace logique mis à l'échelle DPI)
    @location(1) color: vec4<f32>, // sRGB
    @location(2) clip: vec4<f32>,  // x, y, width, height
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
    @location(1) frag_px: vec2<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
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
    out.frag_px = v.pos;
    out.clip = v.clip;
    return out;
}

// sRGB → linéaire : la cible sRGB ré-encode à l'écriture, donc on envoie du
// linéaire pour restituer exactement la couleur d'auteur (idem quad.wgsl).
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
    return vec4<f32>(srgb_to_linear(in.color.rgb), in.color.a * inside_clip);
}
