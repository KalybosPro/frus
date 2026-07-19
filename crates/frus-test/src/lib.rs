//! `frus-test` — les outils de test du framework (§13 du cahier d'idées).
//!
//! Trois étages, du moins au plus intégré :
//!
//! 1. **`update` pur** : rien à fournir — l'architecture Elm rend l'état
//!    testable sans GPU ni fenêtre (`assert_eq!` sur la struct après un
//!    message). Ce crate n'intervient pas.
//! 2. **Snapshot de scène/widget** : [`render_scene`]/[`render_widget`]
//!    rendent hors écran (même pipeline que la fenêtre) et renvoient un
//!    [`Snapshot`] interrogeable pixel par pixel.
//! 3. **Goldens** : [`Snapshot::assert_golden`] compare à une image de
//!    référence sur disque (PNG). Absente, elle est créée (à relire et
//!    committer) ; `FRUS_UPDATE_GOLDENS=1` la régénère.
//!
//! Les goldens dépendent du rasteriseur (l'anticrénelage du texte varie d'un
//! GPU à l'autre) : générez-les et comparez-les **dans le même environnement**
//! (ici : llvmpipe sous WSL, déterministe). Sans adaptateur GPU, les fonctions
//! de rendu renvoient `None` — les tests s'ignorent proprement.

use std::path::Path;

use frus_core::{Color, Scene, Size};
use frus_widgets::{build_ui, Runtime, Theme, Widget};

/// Une frame rendue : octets RGBA sRGB, origine en haut-gauche.
pub struct Snapshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Rend une [`Scene`] hors écran sur fond `clear`. `None` sans GPU.
pub fn render_scene(scene: &Scene, width: u32, height: u32, clear: Color) -> Option<Snapshot> {
    let frame = frus_gpu::render_offscreen(scene, width, height, clear)?;
    Some(Snapshot {
        width: frame.width,
        height: frame.height,
        rgba: frame.rgba,
    })
}

/// Construit et rend un **arbre de widgets** comme le ferait le shell : layout
/// (taffy) + peinture (thème fourni) sur fond `theme.background`, dans une
/// fenêtre virtuelle `width`×`height`. État retenu neutre (pas de survol, pas
/// de focus, défilement à zéro). `None` sans GPU.
pub fn render_widget<Msg: Clone + 'static>(
    root: &dyn Widget<Msg>,
    width: u32,
    height: u32,
    theme: &Theme,
) -> Option<Snapshot> {
    let runtime = Runtime::default();
    let ui = build_ui(root, Size::new(width as f32, height as f32), &runtime, theme);
    render_scene(ui.scene(), width, height, theme.background)
}

impl Snapshot {
    /// Le pixel `(x, y)` en RGBA.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "pixel hors cadre");
        let i = ((y * self.width + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// Nombre de pixels dont un canal RGB au moins dépasse `threshold` —
    /// « quelque chose est dessiné ici ».
    pub fn lit_pixels(&self, threshold: u8) -> usize {
        self.rgba
            .chunks_exact(4)
            .filter(|px| px[0] > threshold || px[1] > threshold || px[2] > threshold)
            .count()
    }

    /// Nombre de pixels différant de `other` au-delà de `channel_tolerance`
    /// sur un canal RGBA au moins. Panique si les tailles diffèrent.
    pub fn diff_count(&self, other: &Snapshot, channel_tolerance: u8) -> usize {
        assert_eq!(
            (self.width, self.height),
            (other.width, other.height),
            "tailles de snapshots différentes"
        );
        self.rgba
            .chunks_exact(4)
            .zip(other.rgba.chunks_exact(4))
            .filter(|(a, b)| {
                a.iter()
                    .zip(b.iter())
                    .any(|(ca, cb)| ca.abs_diff(*cb) > channel_tolerance)
            })
            .count()
    }

    /// Compare au golden `path` (PNG, chemin typiquement construit avec
    /// `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/…")`).
    ///
    /// - golden absent → il est **créé** (à relire puis committer) ;
    /// - `FRUS_UPDATE_GOLDENS=1` → il est réécrit ;
    /// - écart au-delà de (`channel_tolerance` par canal, `max_diff_pixels`
    ///   pixels) → panique, en écrivant `<nom>.actual.png` à côté pour
    ///   comparaison visuelle.
    pub fn assert_golden(&self, path: impl AsRef<Path>) {
        self.assert_golden_with(path, 2, 0);
    }

    /// Variante paramétrée de [`Snapshot::assert_golden`].
    pub fn assert_golden_with(
        &self,
        path: impl AsRef<Path>,
        channel_tolerance: u8,
        max_diff_pixels: usize,
    ) {
        let path = path.as_ref();
        let update = std::env::var_os("FRUS_UPDATE_GOLDENS").is_some_and(|v| v == "1");
        if update || !path.exists() {
            write_png(path, self);
            if !update {
                eprintln!("golden créé : {} — à relire puis committer", path.display());
            }
            return;
        }

        let golden = read_png(path);
        if (golden.width, golden.height) != (self.width, self.height) {
            let actual = actual_path(path);
            write_png(&actual, self);
            panic!(
                "golden {} : taille {}×{} ≠ rendu {}×{} (rendu écrit dans {})",
                path.display(),
                golden.width,
                golden.height,
                self.width,
                self.height,
                actual.display()
            );
        }
        let diff = self.diff_count(&golden, channel_tolerance);
        if diff > max_diff_pixels {
            let actual = actual_path(path);
            write_png(&actual, self);
            panic!(
                "golden {} : {diff} pixel(s) diffèrent (tolérance {max_diff_pixels}) — rendu \
                 écrit dans {} ; relancer avec FRUS_UPDATE_GOLDENS=1 pour accepter",
                path.display(),
                actual.display()
            );
        }
    }
}

/// `<dossier>/<nom>.actual.png` à côté du golden.
fn actual_path(golden: &Path) -> std::path::PathBuf {
    golden.with_extension("actual.png")
}

fn write_png(path: &Path, snapshot: &Snapshot) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("création du dossier des goldens");
    }
    let file = std::fs::File::create(path)
        .unwrap_or_else(|e| panic!("création de {} : {e}", path.display()));
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), snapshot.width, snapshot.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("en-tête PNG");
    writer.write_image_data(&snapshot.rgba).expect("écriture PNG");
}

fn read_png(path: &Path) -> Snapshot {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("lecture de {} : {e}", path.display()));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("info PNG");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("frame PNG");
    assert_eq!(info.color_type, png::ColorType::Rgba, "golden non RGBA");
    buf.truncate(info.buffer_size());
    Snapshot {
        width: info.width,
        height: info.height,
        rgba: buf,
    }
}
