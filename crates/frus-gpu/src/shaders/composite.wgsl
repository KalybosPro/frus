// Compositing d'un **calque** : un quad plein-écran échantillonne la texture du
// calque (déjà rendue) et la recompose à l'opacité de groupe, avec découpe.
// L'échantillon est déjà linéaire (texture sRGB) : pas de reconversion.

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
    @location(1) clip: vec4<f32>,   // x, y, width, height (px)
    @location(2) params: vec4<f32>, // opacité, angle (rad), pivot.x, pivot.y
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) frag_px: vec2<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) opacity: f32,
    @location(4) @interpolate(flat) rot: vec3<f32>, // angle, pivot.x, pivot.y
};

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    // Le quad unité couvre tout l'écran ; la texture du calque est à la taille de
    // la surface, donc uv = position normalisée.
    let ndc = vec2<f32>(vert.unit_pos.x * 2.0 - 1.0, 1.0 - vert.unit_pos.y * 2.0);
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = vert.unit_pos;
    out.frag_px = vert.unit_pos * viewport.size;
    out.clip = inst.clip;
    out.opacity = inst.params.x;
    out.rot = inst.params.yzw;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Rotation du calque : la texture contient le contenu **à plat** (non tourné) à
    // sa position écran. Pour peindre le calque tourné de `angle` autour du pivot,
    // on échantillonne à la position **contre-tournée** de `-angle` : le pixel écran
    // p reçoit le contenu qui, tourné de +angle, atterrit en p.
    let angle = in.rot.x;
    let pivot = in.rot.yz;
    var src_px = in.frag_px;
    if (angle != 0.0) {
        let d = in.frag_px - pivot;
        let c = cos(-angle);
        let s = sin(-angle);
        src_px = pivot + vec2<f32>(d.x * c - d.y * s, d.x * s + d.y * c);
    }
    let src_uv = src_px / viewport.size;

    // Hors de la texture après contre-rotation : rien (bord transparent).
    let in_bounds = f32(
        src_uv.x >= 0.0 && src_uv.x <= 1.0 && src_uv.y >= 0.0 && src_uv.y <= 1.0
    );
    // Découpe testée sur le pixel écran (rectangle aligné sur les axes).
    let inside_clip = f32(
        in.frag_px.x >= in.clip.x
        && in.frag_px.x <= in.clip.x + in.clip.z
        && in.frag_px.y >= in.clip.y
        && in.frag_px.y <= in.clip.y + in.clip.w
    );
    let sample = textureSample(tex, samp, src_uv);
    // Alpha droit ×  opacité de groupe : le blend (SrcAlpha, 1-SrcAlpha) réalise
    // le « over » correct — pas de double-superposition interne au calque.
    return vec4<f32>(sample.rgb, sample.a * in.opacity * inside_clip * in_bounds);
}
