//! Rendu de texte via [`glyphon`](https://docs.rs/glyphon) (cosmic-text + atlas
//! de glyphes wgpu). Dessine les primitives [`Primitive::Text`] dans le render
//! pass, par-dessus les rectangles.

use frus_core::{Color, Point, Primitive, Rect, Scene, TextDecoration};

/// Rapport interligne / taille de police (cohérent avec `frus-text`).
const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Un quad de **décoration de texte** (soulignement, barré…) calculé à partir
/// des lignes mises en forme. Rendu par le pipeline des rectangles, *avant* les
/// glyphes (même couleur → indiscernable ; le texte reste lisible sinon).
pub(crate) struct DecorationQuad {
    pub rect: Rect,
    pub color: Color,
    pub clip: Rect,
}

/// Position du soulignement sous la ligne de base (fraction de la taille).
const UNDERLINE_OFFSET: f32 = 0.12;
/// Position du barré au-dessus de la ligne de base (≈ mi-hauteur d'x).
const STRIKETHROUGH_OFFSET: f32 = 0.28;
/// Position de la ligne au-dessus (≈ hauteur d'ascendante).
const OVERLINE_OFFSET: f32 = 0.90;

/// Émet les quads d'une ligne décorée : `[x0, x1]` en avance depuis `origin`,
/// `baseline` relative au haut du paragraphe, épaisseur dérivée de `size`.
fn push_line_quads(
    quads: &mut Vec<DecorationQuad>,
    origin: Point,
    x0: f32,
    x1: f32,
    baseline: f32,
    size: f32,
    decoration: TextDecoration,
    color: Color,
    clip: Rect,
) {
    if x1 <= x0 {
        return;
    }
    let thickness = (size / 14.0).max(1.0);
    let mut line = |y: f32| {
        quads.push(DecorationQuad {
            rect: Rect::new(
                origin.x + x0,
                origin.y + baseline + y - thickness / 2.0,
                x1 - x0,
                thickness,
            ),
            color,
            clip,
        });
    };
    if decoration.underline {
        line(UNDERLINE_OFFSET * size);
    }
    if decoration.strikethrough {
        line(-STRIKETHROUGH_OFFSET * size);
    }
    if decoration.overline {
        line(-OVERLINE_OFFSET * size);
    }
}

fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Détient l'état glyphon nécessaire au rendu de texte.
pub(crate) struct TextPainter {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    renderer: glyphon::TextRenderer,
}

impl TextPainter {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        // Police embarquée + repli système : même politique que la mesure texte
        // (`frus-text`), pour un rendu déterministe et un défaut résoluble partout.
        let font_system = frus_text::new_font_system();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);
        let mut atlas = glyphon::TextAtlas::new(device, queue, &cache, format);
        let renderer =
            glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderer,
        }
    }

    /// Prépare le rendu du texte de la scène. À appeler avant le render pass.
    /// Renvoie les **quads de décoration** (soulignement, barré…) calculés à
    /// partir des lignes mises en forme — à dessiner par le pipeline des
    /// rectangles.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        width: u32,
        height: u32,
    ) -> Vec<DecorationQuad> {
        self.viewport.update(queue, glyphon::Resolution { width, height });
        let mut decorations = Vec::new();

        // Cible sRGB : on envoie du linéaire (comme les quads) pour éviter le
        // double encodage (texte délavé). L'alpha reste tel quel.
        let to_glyphon = |color: &frus_core::Color| {
            let linear = color.to_linear();
            glyphon::Color::rgba(
                to_u8(linear.r),
                to_u8(linear.g),
                to_u8(linear.b),
                to_u8(color.a),
            )
        };
        // Découpe = clip de la primitive, borné à la surface.
        let to_bounds = |clip: &frus_core::Rect| glyphon::TextBounds {
            left: clip.x.max(0.0) as i32,
            top: clip.y.max(0.0) as i32,
            right: (clip.x + clip.width).min(width as f32) as i32,
            bottom: (clip.y + clip.height).min(height as f32) as i32,
        };

        // Construit un buffer glyphon par primitive de texte (simple ou riche).
        let mut buffers = Vec::new();
        for primitive in scene.primitives() {
            match primitive {
                Primitive::Text {
                    position,
                    text,
                    size,
                    color,
                    weight,
                    italic,
                    max_width,
                    decoration,
                    decoration_color,
                    clip,
                    ..
                } => {
                    let metrics = glyphon::Metrics::new(*size, *size * LINE_HEIGHT_FACTOR);
                    let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                    // Un paragraphe se replie à sa largeur de mise en page. Un
                    // texte libre reste **non contraint** (`None`) — surtout pas
                    // borné à la surface : en RTL, cosmic-text aligne à droite de
                    // la largeur du buffer, ce qui pousserait les glyphes hors
                    // écran (bord droit) une fois décalés par `position.x`.
                    buffer.set_size(&mut self.font_system, *max_width, Some(height as f32));
                    // Graisse + italique : cosmic-text choisit la face correspondante
                    // de la famille (repli sur la plus proche si absente).
                    let attrs = glyphon::Attrs::new()
                        // Famille par script (arabe → Noto) : pas de repli
                        // cross-famille sur Android, on choisit à la source.
                        .family(frus_text::family_for(text))
                        .weight(glyphon::Weight(frus_text::available_weight(*weight)))
                        .style(if *italic {
                            glyphon::Style::Italic
                        } else {
                            glyphon::Style::Normal
                        });
                    buffer.set_text(&mut self.font_system, text, attrs, glyphon::Shaping::Advanced);
                    buffer.shape_until_scroll(&mut self.font_system, false);

                    // Décorations : une ligne par run de mise en forme, de la
                    // première à la dernière avance de glyphe.
                    if !decoration.is_none() {
                        let deco_color = decoration_color.unwrap_or(*color);
                        for run in buffer.layout_runs() {
                            let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last())
                            else {
                                continue;
                            };
                            push_line_quads(
                                &mut decorations,
                                *position,
                                first.x,
                                last.x + last.w,
                                run.line_y,
                                *size,
                                *decoration,
                                deco_color,
                                *clip,
                            );
                        }
                    }

                    buffers.push((buffer, position.x, position.y, to_glyphon(color), to_bounds(clip)));
                }
                Primitive::RichText {
                    position,
                    runs,
                    max_width,
                    clip,
                    ..
                } => {
                    if runs.is_empty() {
                        continue;
                    }
                    // Métriques de base : le plus grand run (les runs plus petits
                    // portent leurs propres métriques par-span).
                    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);
                    let metrics = glyphon::Metrics::new(base, base * LINE_HEIGHT_FACTOR);
                    let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                    // Idem texte simple : un paragraphe riche se replie à sa
                    // largeur de mise en page ; sinon non contraint (`None`),
                    // jamais borné à la surface (alignement RTL hors écran).
                    buffer.set_size(&mut self.font_system, *max_width, Some(height as f32));
                    let spans = runs.iter().enumerate().map(|(index, run)| {
                        (
                            run.text.as_str(),
                            glyphon::Attrs::new()
                                .family(frus_text::family_for(&run.text))
                                .weight(glyphon::Weight(frus_text::available_weight(run.weight)))
                                .style(if run.italic {
                                    glyphon::Style::Italic
                                } else {
                                    glyphon::Style::Normal
                                })
                                .metrics(glyphon::Metrics::new(
                                    run.size,
                                    run.size * LINE_HEIGHT_FACTOR,
                                ))
                                .color(to_glyphon(&run.color))
                                // Rattache chaque glyphe à son run source (pour
                                // les décorations par-span).
                                .metadata(index),
                        )
                    });
                    buffer.set_rich_text(
                        &mut self.font_system,
                        spans,
                        glyphon::Attrs::new(),
                        glyphon::Shaping::Advanced,
                    );
                    buffer.shape_until_scroll(&mut self.font_system, false);

                    // Décorations par run : les glyphes consécutifs d'un même run
                    // (métadonnée) forment un segment décoré d'un seul tenant.
                    if runs.iter().any(|r| !r.decoration.is_none()) {
                        for lrun in buffer.layout_runs() {
                            let glyphs = lrun.glyphs;
                            let mut start = 0;
                            while start < glyphs.len() {
                                let meta = glyphs[start].metadata;
                                let mut end = start + 1;
                                while end < glyphs.len() && glyphs[end].metadata == meta {
                                    end += 1;
                                }
                                if let Some(run) = runs.get(meta) {
                                    if !run.decoration.is_none() {
                                        let last = &glyphs[end - 1];
                                        push_line_quads(
                                            &mut decorations,
                                            *position,
                                            glyphs[start].x,
                                            last.x + last.w,
                                            lrun.line_y,
                                            run.size,
                                            run.decoration,
                                            run.decoration_color.unwrap_or(run.color),
                                            *clip,
                                        );
                                    }
                                }
                                start = end;
                            }
                        }
                    }

                    // Chaque run porte sa couleur par attrs ; le défaut ne sert
                    // qu'aux glyphes sans couleur (il n'y en a pas).
                    let default_color = to_glyphon(&runs[0].color);
                    buffers.push((buffer, position.x, position.y, default_color, to_bounds(clip)));
                }
                // Rectangles et chemins ne concernent pas le rendu du texte.
                Primitive::Rect { .. } | Primitive::Path { .. } => {}
            }
        }

        let areas = buffers
            .iter()
            .map(|(buffer, left, top, color, bounds)| glyphon::TextArea {
                buffer,
                left: *left,
                top: *top,
                scale: 1.0,
                bounds: *bounds,
                default_color: *color,
                custom_glyphs: &[],
            });

        if let Err(err) = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        ) {
            log::warn!("glyphon prepare a échoué : {err:?}");
        }
        decorations
    }

    /// Dessine le texte préparé dans un render pass ouvert.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if let Err(err) = self.renderer.render(&self.atlas, &self.viewport, pass) {
            log::warn!("glyphon render a échoué : {err:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Point, Scene};

    /// Rend `scene` (sur fond noir) via le chemin hors écran partagé et compte
    /// les pixels « allumés » (canal rouge > 16). `None` si aucun GPU dispo.
    fn lit_pixels_for(scene: &Scene) -> Option<usize> {
        let frame = crate::offscreen::render_offscreen(scene, 128, 128, Color::BLACK)?;
        Some(frame.rgba.chunks_exact(4).filter(|px| px[0] > 16).count())
    }

    /// Preuve de rasterisation : un texte blanc produit des pixels non-noirs.
    #[test]
    fn renders_text_to_non_background_pixels() {
        let mut scene = Scene::new();
        scene.text(Point::new(4.0, 4.0), "Hello", 48.0, Color::WHITE);
        match lit_pixels_for(&scene) {
            None => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
            Some(lit) => {
                assert!(lit > 0, "le texte devrait produire des pixels non-noirs ({lit})")
            }
        }
    }

    /// L'**arabe** rasterise via la face Naskh embarquée (`family_for` route le
    /// script arabe vers "Noto Naskh Arabic"). Preuve que le chemin de rendu —
    /// pas seulement la mesure — façonne bien les glyphes arabes.
    #[test]
    fn renders_arabic_to_non_background_pixels() {
        let mut scene = Scene::new();
        scene.text(Point::new(4.0, 40.0), "مهامي", 40.0, Color::WHITE);
        match lit_pixels_for(&scene) {
            None => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
            Some(lit) => {
                assert!(lit > 20, "l'arabe devrait rasteriser des glyphes ({lit})")
            }
        }
    }

    /// Le texte **riche** (`set_rich_text`, runs mêlés tailles/graisses) rasterise
    /// aussi — preuve de bout en bout du nouveau chemin GPU.
    #[test]
    fn renders_rich_text_to_non_background_pixels() {
        use frus_core::{FontWeight, TextRun};
        let run = |text: &str, size: f32, weight: FontWeight| TextRun {
            text: text.to_string(),
            size,
            weight,
            italic: false,
            color: Color::WHITE,
            decoration: TextDecoration::NONE,
            decoration_color: None,
        };
        let mut scene = Scene::new();
        scene.rich_text(
            Point::new(4.0, 4.0),
            vec![run("Ri", 40.0, FontWeight::Regular), run("ch", 24.0, FontWeight::Bold)],
        );
        match lit_pixels_for(&scene) {
            None => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
            Some(lit) => {
                assert!(lit > 0, "le texte riche devrait produire des pixels non-noirs ({lit})")
            }
        }
    }

    /// Le **soulignement** ajoute des pixels par rapport au même texte nu —
    /// preuve par readback que les quads de décoration traversent tout le
    /// chemin (calcul depuis les lignes mises en forme + passe des rectangles).
    #[test]
    fn underline_lights_more_pixels_than_plain_text() {
        use frus_core::TextStyle;
        let plain = {
            let mut scene = Scene::new();
            scene.text_styled(Point::new(4.0, 4.0), "Hello", &TextStyle::new(40.0), Color::WHITE);
            scene
        };
        let underlined = {
            let mut scene = Scene::new();
            scene.text_styled(
                Point::new(4.0, 4.0),
                "Hello",
                &TextStyle::new(40.0).underline(),
                Color::WHITE,
            );
            scene
        };
        match (lit_pixels_for(&plain), lit_pixels_for(&underlined)) {
            (Some(bare), Some(deco)) => {
                assert!(
                    deco > bare + 50,
                    "le soulignement doit ajouter des pixels (nu {bare}, décoré {deco})"
                );
            }
            _ => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
        }
    }

    /// Le **barré par-span** d'un texte riche n'ajoute des pixels que sur le run
    /// décoré (chemin métadonnées → segments par run).
    #[test]
    fn rich_text_strikethrough_is_per_run() {
        use frus_core::{FontWeight, TextRun};
        let run = |text: &str, decoration: TextDecoration| TextRun {
            text: text.to_string(),
            size: 40.0,
            weight: FontWeight::Regular,
            italic: false,
            color: Color::WHITE,
            decoration,
            decoration_color: None,
        };
        let plain = {
            let mut scene = Scene::new();
            scene.rich_text(
                Point::new(4.0, 4.0),
                vec![run("ab", TextDecoration::NONE), run("cd", TextDecoration::NONE)],
            );
            scene
        };
        let struck = {
            let mut scene = Scene::new();
            scene.rich_text(
                Point::new(4.0, 4.0),
                vec![run("ab", TextDecoration::NONE), run("cd", TextDecoration::STRIKETHROUGH)],
            );
            scene
        };
        match (lit_pixels_for(&plain), lit_pixels_for(&struck)) {
            (Some(bare), Some(deco)) => {
                assert!(
                    deco > bare,
                    "le barré du second run doit ajouter des pixels (nu {bare}, décoré {deco})"
                );
            }
            _ => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
        }
    }
}
