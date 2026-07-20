//! Rendu **hors écran** : la scène dessinée dans une texture puis relue par le
//! CPU — aucune fenêtre requise. C'est la brique des tests de rendu (goldens
//! `frus-test`) et la seule façon d'observer le pipeline sous WSL/CI.
//!
//! Le pipeline est **le même** que celui de la fenêtre ([`crate::Renderer`]) :
//! quads (+ quads de décoration de texte) puis glyphes, cible sRGB — les
//! octets relus correspondent à ce qu'une capture d'écran donnerait.

use frus_core::{Color, Scene};

use crate::compositor::Painters;

/// Une frame rendue hors écran : octets RGBA **sRGB**, ligne par ligne.
pub struct OffscreenFrame {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` octets, ordre RGBA, origine en haut-gauche.
    pub rgba: Vec<u8>,
    /// Nombre d'échantillons MSAA effectivement employé (1 = pas de lissage,
    /// GPU sans support MSAA). Informatif — utile aux tests d'anti-aliasing.
    pub samples: u32,
}

/// Rend `scene` dans une texture `width`×`height` effacée à `clear`, et relit
/// les pixels. `None` si aucun adaptateur GPU n'est disponible (machine sans
/// GPU ni rasteriseur logiciel) — les appelants de test s'ignorent alors.
pub fn render_offscreen(
    scene: &Scene,
    width: u32,
    height: u32,
    clear: Color,
) -> Option<OffscreenFrame> {
    let (device, queue, sample_count) = headless_device()?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frus.offscreen.target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Même pipeline que `Renderer::render`, via le compositeur (calques compris).
    // Le clear est en sRGB (couleur d'auteur) : la cible sRGB attend du linéaire.
    let clear_linear = clear.to_linear();
    let mut painters = Painters::new(&device, &queue, format, sample_count);
    painters.render(
        &device,
        &queue,
        format,
        &view,
        width,
        height,
        scene,
        Some(wgpu::Color {
            r: clear_linear.r as f64,
            g: clear_linear.g as f64,
            b: clear_linear.b as f64,
            a: clear.a as f64,
        }),
    );

    // Relecture : wgpu impose des lignes alignées sur 256 octets — on rembourre
    // puis on retire le rembourrage côté CPU.
    let unpadded_bytes_per_row = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("frus.offscreen.readback"),
        size: (padded_bytes_per_row * height) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frus.offscreen.copy") });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;

    let data = slice.get_mapped_range();
    let mut rgba = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        rgba.extend_from_slice(&data[start..start + unpadded_bytes_per_row as usize]);
    }

    Some(OffscreenFrame {
        width,
        height,
        rgba,
        samples: sample_count,
    })
}

/// Instancie un device wgpu sans surface, avec le nombre d'échantillons MSAA
/// supporté pour la cible sRGB. `None` si aucun adaptateur.
fn headless_device() -> Option<(wgpu::Device, wgpu::Queue, u32)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let sample_count =
        crate::compositor::preferred_sample_count(&adapter, wgpu::TextureFormat::Rgba8UnormSrgb);
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("frus.offscreen.device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .ok()?;
    Some((device, queue, sample_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Path, Point, Rect};

    /// Le chemin hors écran rend la même chose que le pipeline fenêtré : un
    /// rectangle rouge plein → pixel central rouge, hors du rect → clear.
    #[test]
    fn renders_rect_and_reads_back_srgb() {
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 20.0, 20.0), Color::rgb(1.0, 0.0, 0.0));
        // Largeur non multiple de 64 : exerce le rembourrage de relecture.
        let Some(frame) = render_offscreen(&scene, 70, 40, Color::BLACK) else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        assert_eq!(frame.rgba.len(), 70 * 40 * 4);
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        assert_eq!(px(10, 10), [255, 0, 0], "dans le rect → rouge");
        assert_eq!(px(60, 30), [0, 0, 0], "hors du rect → clear");
    }

    /// Un **triangle vectoriel** rempli en vert : un point bien à l'intérieur est
    /// vert, un coin hors du triangle reste au clear. Preuve que la tessellation
    /// + le pipeline de chemins produisent bien des pixels.
    #[test]
    fn fills_a_vector_triangle() {
        // Triangle couvrant le bas de la surface : sommet en haut au centre,
        // base sur toute la largeur en bas.
        let triangle = Path::new()
            .move_to(Point::new(32.0, 4.0))
            .line_to(Point::new(60.0, 60.0))
            .line_to(Point::new(4.0, 60.0))
            .close();
        let mut scene = Scene::new();
        scene.fill_path(&triangle, Color::rgb(0.0, 1.0, 0.0));

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // Près de la base, au centre : bien à l'intérieur → vert.
        assert_eq!(px(32, 54), [0, 255, 0], "intérieur du triangle → vert");
        // Coin haut-gauche : au-dessus/à côté du sommet → hors triangle → clear.
        assert_eq!(px(4, 4), [0, 0, 0], "hors du triangle → clear");
    }

    /// Le **contour** (stroke) d'un chemin peint des pixels sur le trait mais pas
    /// au centre (chemin non rempli).
    #[test]
    fn strokes_a_path_outline_only() {
        // Un carré tracé (non rempli) de 40×40 centré.
        let square = Path::rect(Rect::new(12.0, 12.0, 40.0, 40.0));
        let mut scene = Scene::new();
        scene.stroke_path(&square, Color::rgb(0.0, 0.0, 1.0), 4.0);

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // Sur le bord gauche du carré → bleu ; au centre → clear (pas de fill).
        assert_eq!(px(12, 32), [0, 0, 255], "sur le contour → bleu");
        assert_eq!(px(32, 32), [0, 0, 0], "au centre → clear (non rempli)");
    }

    /// Échantillonnage de texture : une image 2×2 (rouge/vert/bleu/blanc) étirée
    /// (`Fill`) sur toute la surface — chaque quadrant lit sa couleur. Preuve du
    /// téléversement + de l'échantillonnage + du mapping UV.
    #[test]
    fn samples_a_texture_by_quadrant() {
        use frus_core::{BoxFit, ImageData};
        // 2×2 : (0,0) rouge, (1,0) vert, (0,1) bleu, (1,1) blanc.
        let pixels = vec![
            255, 0, 0, 255, /* R */ 0, 255, 0, 255, /* G */
            0, 0, 255, 255, /* B */ 255, 255, 255, 255, /* W */
        ];
        let image = ImageData::from_rgba(2, 2, pixels).into_handle();
        let mut scene = Scene::new();
        scene.image(&image, Rect::new(0.0, 0.0, 64.0, 64.0), BoxFit::Fill);

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // Un point bien au cœur de chaque quadrant (loin des bords interpolés).
        assert_eq!(px(10, 10), [255, 0, 0], "haut-gauche → rouge");
        assert_eq!(px(54, 10), [0, 255, 0], "haut-droit → vert");
        assert_eq!(px(10, 54), [0, 0, 255], "bas-gauche → bleu");
        assert_eq!(px(54, 54), [255, 255, 255], "bas-droit → blanc");
    }

    /// **Anti-aliasing (MSAA)** : le bord **oblique** d'un triangle produit des
    /// pixels **partiellement** couverts (mélange fond/forme) — signature du
    /// multi-échantillon, impossible avec un rendu net. Si le GPU ne supporte pas
    /// le MSAA (`samples == 1`), le test s'ignore.
    #[test]
    fn msaa_smooths_a_diagonal_edge() {
        // Triangle rectangle : hypoténuse sur l'anti-diagonale x + y = 64.
        let triangle = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(64.0, 0.0))
            .line_to(Point::new(0.0, 64.0))
            .close();
        let mut scene = Scene::new();
        scene.fill_path(&triangle, Color::rgb(0.0, 1.0, 0.0));

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        if frame.samples == 1 {
            eprintln!("MSAA non supporté par ce GPU : test ignoré");
            return;
        }
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // Le long du bord oblique, au moins un pixel doit être un vert
        // **intermédiaire** (ni plein 255, ni fond 0) — preuve du lissage. Un
        // rendu net ne produirait que du 0 ou du 255. On balaye toute la surface :
        // seuls les pixels du bord partiellement couverts sont intermédiaires.
        let found_partial = (0..frame.height).any(|y| {
            (0..frame.width).any(|x| {
                let g = px(x, y)[1];
                g > 20 && g < 235
            })
        });
        assert!(found_partial, "un bord oblique lissé a des pixels de vert intermédiaire");
        // Loin dans le triangle → plein vert ; loin dehors → fond noir.
        assert_eq!(px(4, 4), [0, 255, 0], "intérieur → vert plein");
        assert_eq!(px(60, 60), [0, 0, 0], "extérieur → fond");
    }

    /// **Compositing de calque** : deux rectangles rouges **opaques** qui se
    /// chevauchent, groupés dans un calque à opacité 0.5. L'alpha de groupe est
    /// **uniforme** — le chevauchement a la même couleur qu'une simple couverture
    /// (pas de double-superposition), et c'est bien ~moitié rouge sur fond noir.
    #[test]
    fn layer_group_opacity_is_uniform_over_overlap() {
        let mut scene = Scene::new();
        scene.layer(0.5, |inner| {
            inner.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), Color::rgb(1.0, 0.0, 0.0));
            inner.fill_rect(Rect::new(24.0, 24.0, 40.0, 40.0), Color::rgb(1.0, 0.0, 0.0));
        });

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        let single = px(8, 8); // couvert par le 1er rectangle seulement
        let overlap = px(32, 32); // couvert par les deux
        assert_eq!(single, overlap, "l'alpha de groupe est uniforme sur le chevauchement");
        // ~50 % rouge sur noir : ni plein rouge (255) ni fond (0).
        assert!(single[0] > 120 && single[0] < 215, "≈ moitié rouge (R={})", single[0]);
        assert_eq!(single[1], 0, "pas de vert");
        assert_eq!(single[2], 0, "pas de bleu");
        // Hors des deux rectangles : fond noir.
        assert_eq!(px(60, 8), [0, 0, 0], "hors du calque → fond");
    }
}
