// Rendu instancié de rectangles.
//
// Un quad unité (coins 0..1) est répété pour chaque instance. Chaque instance
// porte un rectangle en pixels logiques (x, y, w, h) et une couleur. Le vertex
// shader place le quad dans le rectangle, puis convertit les pixels (origine
// haut-gauche, Y bas) en coordonnées NDC (origine centre, Y haut).

struct Viewport {
    // Taille de la surface en pixels (largeur, hauteur).
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct VertexInput {
    // Coin du quad unité : (0,0), (1,0), (1,1) ou (0,1).
    @location(0) unit_pos: vec2<f32>,
};

struct InstanceInput {
    @location(1) rect: vec4<f32>,   // x, y, width, height (pixels)
    @location(2) color: vec4<f32>,  // r, g, b, a
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    // Position en pixels logiques dans la fenêtre.
    let pos_px = inst.rect.xy + vert.unit_pos * inst.rect.zw;

    // Pixels (origine haut-gauche, Y bas) -> NDC (origine centre, Y haut).
    let ndc = vec2<f32>(
        pos_px.x / viewport.size.x * 2.0 - 1.0,
        1.0 - pos_px.y / viewport.size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
