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
    @location(1) clip: vec4<f32>,    // x, y, width, height (px)
    @location(2) inv_lin: vec4<f32>, // inverse affine, partie lineaire : ia, ib, ic, id
    @location(3) inv_tr_op: vec4<f32>, // ie, if, opacite, _
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) frag_px: vec2<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) opacity: f32,
    @location(4) @interpolate(flat) inv_lin: vec4<f32>,
    @location(5) @interpolate(flat) inv_tr: vec2<f32>,
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
    out.opacity = inst.inv_tr_op.z;
    out.inv_lin = inst.inv_lin;
    out.inv_tr = inst.inv_tr_op.xy;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Transformation du calque : la texture contient le contenu **à plat** (non
    // transformé) à sa position écran. Pour peindre le calque transformé par M, on
    // échantillonne à la position **contre-transformée** M⁻¹(p) : le pixel écran p
    // reçoit le contenu qui, transformé par M, atterrit en p. `inv_lin = ia,ib,ic,id`,
    // `inv_tr = ie,if` : src = (ia·x + ic·y + ie, ib·x + id·y + if).
    let p = in.frag_px;
    let src_px = vec2<f32>(
        in.inv_lin.x * p.x + in.inv_lin.z * p.y + in.inv_tr.x,
        in.inv_lin.y * p.x + in.inv_lin.w * p.y + in.inv_tr.y,
    );
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
