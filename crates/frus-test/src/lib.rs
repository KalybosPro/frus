//! `frus-test` — the framework's test tooling (§13 of the design notes).
//!
//! Three levels, from the least integrated to the most:
//!
//! 1. **Pure `update`**: nothing to supply — the Elm architecture makes state
//!    testable with neither GPU nor window (`assert_eq!` on the struct after a
//!    message). This crate plays no part.
//! 2. **Scene and widget snapshots**: [`render_scene`] and [`render_widget`]
//!    render offscreen, through the same pipeline as the window, and return a
//!    [`Snapshot`] that can be queried pixel by pixel.
//! 3. **Goldens**: [`Snapshot::assert_golden`] compares against a reference PNG on
//!    disk. When it is missing, it is created — read it over, then commit it;
//!    `FRUS_UPDATE_GOLDENS=1` regenerates it.
//! 4. **A frame loop**: [`Stage`] holds the retained state and steps it the way the
//!    shell does, for the widgets whose picture is a gesture in flight — a swipe
//!    half-done, a pull past the top, a page between two pages — rather than a
//!    function of their arguments.
//!
//! Goldens depend on the rasteriser, text antialiasing varying from one GPU to
//! another, so generate and compare them **in the same environment** — here,
//! llvmpipe under WSL, which is deterministic. With no GPU adapter the render
//! functions return `None` and the tests skip cleanly.

use std::path::Path;

use frus_core::{Color, Scene, Size};
use frus_widgets::{build_ui, Runtime, ScrollPhysics, Theme, Ui, Widget};

/// A rendered frame: sRGB RGBA bytes, origin at the top left.
pub struct Snapshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Renders a [`Scene`] offscreen on a `clear` background. `None` with no GPU.
pub fn render_scene(scene: &Scene, width: u32, height: u32, clear: Color) -> Option<Snapshot> {
    let frame = frus_gpu::render_offscreen(scene, width, height, clear)?;
    Some(Snapshot {
        width: frame.width,
        height: frame.height,
        rgba: frame.rgba,
    })
}

/// Builds and renders a **widget tree** the way the shell would: layout through
/// taffy, then painting with the given theme, on a `theme.background` background, in
/// a virtual `width`×`height` window. The retained state is neutral: no hover, no
/// focus, scroll at zero. `None` with no GPU.
pub fn render_widget<Msg: Clone + 'static>(
    root: &dyn Widget<Msg>,
    width: u32,
    height: u32,
    theme: &Theme,
) -> Option<Snapshot> {
    let mut stage = Stage::new(width, height).theme(*theme);
    stage.settle(root);
    stage.render(root)
}

/// A frame-by-frame harness: the retained state the shell holds, plus a way to step
/// it forward the way the shell's loop does.
///
/// [`render_widget`] renders one settled frame, which is the right picture for a
/// widget whose appearance follows from its arguments. It is the wrong picture — or
/// no picture at all — for the ones whose appearance follows from **a gesture in
/// flight**: a swipe half-done, a pull past the top edge, a glow where a list hit its
/// end. None of that is in the widget; it is in the [`Runtime`], and the shell is
/// what puts it there.
///
/// A `Stage` puts it there instead, through the same entry points the shell uses
/// (`refresh_pull`, `dismiss_drag`, `glow_pull`, …), so a widget can be caught
/// mid-motion and photographed:
///
/// ```ignore
/// let mut stage = Stage::new(300, 200);
/// let id = stage.build(&root).scroll_regions()[0].id;
/// stage.runtime.glow_pull(id, GlowEdge::Top, 60.0, 200.0);
/// stage.advance(&root, 1.0 / 60.0);
/// stage.render(&root).unwrap().assert_golden(path);
/// ```
pub struct Stage {
    /// The retained state. Every field on it is public, so a test that needs a state
    /// no method here reaches can set it by hand — that is the escape hatch, not the
    /// first resort.
    pub runtime: Runtime,
    /// The theme every frame is built and painted with.
    pub theme: Theme,
    width: u32,
    height: u32,
}

impl Stage {
    /// A stage for a `width`×`height` window, on the dark theme.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            runtime: Runtime::default(),
            theme: Theme::dark(),
            width,
            height,
        }
    }

    /// Paints on `theme` instead.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// The window size, in logical pixels.
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// Builds the tree against the current retained state — the way to reach a
    /// widget's identity (`scroll_regions`, `dismissables`, `refresh_areas`) before
    /// putting a gesture on it.
    pub fn build<'a, Msg: Clone + 'static>(&'a self, root: &'a dyn Widget<Msg>) -> Ui<Msg> {
        build_ui(root, self.size(), &self.runtime, &self.theme)
    }

    /// One turn of the shell's loop: builds the tree, then advances every family of
    /// retained motion by `dt`. Returns whether anything is still moving, which is
    /// what the shell asks to decide if it needs another frame.
    ///
    /// `dt` of zero is not a no-op — it is the **first frame**, where a widget seen
    /// for the first time adopts its target rather than sliding in from zero. See
    /// [`Stage::settle`].
    pub fn advance<Msg: Clone + 'static>(&mut self, root: &dyn Widget<Msg>, dt: f32) -> bool {
        let (regions, refresh_areas, dismissables, interactive) = {
            let ui = self.build(root);
            (
                ui.scroll_regions().to_vec(),
                ui.refresh_areas().to_vec(),
                ui.dismissables().to_vec(),
                ui.interactive_bounds(),
            )
        };
        self.runtime.time += dt;
        self.runtime.sync_pages(&regions);
        let (dismiss_moving, _dismissed) = self.runtime.advance_dismiss(&dismissables, dt);
        // `|` and not `||`: every family must be stepped, whatever an earlier one
        // answered. This is the shell's own list, in the shell's own order.
        self.runtime.advance(dt)
            | self.runtime.advance_leaving(dt)
            | self.runtime.advance_values(root, dt)
            | self.runtime.advance_colors(root, dt)
            | self.runtime.advance_sizes(root, dt)
            | self.runtime.advance_radii(root, dt)
            | self.runtime.advance_paddings(root, dt)
            | self
                .runtime
                .advance_scroll(&regions, ScrollPhysics::default(), dt)
            | self.runtime.advance_glow(dt)
            | self.runtime.advance_refresh(&refresh_areas, dt)
            | self.runtime.advance_interactive(&interactive, dt)
            | dismiss_moving
    }

    /// The shell's **first** frame: every implicit animation adopts its target with
    /// no transition. A switch that is on is drawn on, a drawer declared open is
    /// drawn open.
    pub fn settle<Msg: Clone + 'static>(&mut self, root: &dyn Widget<Msg>) {
        self.advance(root, 0.0);
    }

    /// Advances by `dt` `steps` times — a gesture playing out rather than a jump to
    /// its end, which is the point of having a frame loop at all.
    pub fn advance_by<Msg: Clone + 'static>(
        &mut self,
        root: &dyn Widget<Msg>,
        dt: f32,
        steps: usize,
    ) {
        for _ in 0..steps {
            self.advance(root, dt);
        }
    }

    /// Renders the current frame. `None` with no GPU adapter, so a test can skip.
    pub fn render<Msg: Clone + 'static>(&self, root: &dyn Widget<Msg>) -> Option<Snapshot> {
        let ui = self.build(root);
        render_scene(ui.scene(), self.width, self.height, self.theme.background)
    }
}

impl Snapshot {
    /// The pixel at `(x, y)`, in RGBA.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "pixel out of frame");
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// How many pixels have at least one RGB channel above `threshold` — that is,
    /// "something is drawn here".
    pub fn lit_pixels(&self, threshold: u8) -> usize {
        self.rgba
            .chunks_exact(4)
            .filter(|px| px[0] > threshold || px[1] > threshold || px[2] > threshold)
            .count()
    }

    /// How many pixels differ from `other` by more than `channel_tolerance` on at
    /// least one RGBA channel. Panics when the sizes differ.
    pub fn diff_count(&self, other: &Snapshot, channel_tolerance: u8) -> usize {
        assert_eq!(
            (self.width, self.height),
            (other.width, other.height),
            "the snapshots have different sizes"
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

    /// Writes the frame to `path` as a PNG, creating the folder if it is missing.
    ///
    /// The goldens use this internally; it is public because a rendered frame is
    /// useful outside a test too — the pictures in the README are made this way, so
    /// that they can be regenerated rather than re-photographed.
    pub fn write_png(&self, path: impl AsRef<Path>) {
        write_png(path.as_ref(), self);
    }

    /// Compares against the golden at `path`, a PNG whose path is typically built
    /// with `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/…")`.
    ///
    /// - a missing golden is **created** — read it over, then commit it;
    /// - `FRUS_UPDATE_GOLDENS=1` rewrites it;
    /// - a difference beyond (`channel_tolerance` per channel, `max_diff_pixels`
    ///   pixels) panics, writing `<name>.actual.png` alongside for visual
    ///   comparison.
    pub fn assert_golden(&self, path: impl AsRef<Path>) {
        self.assert_golden_with(path, 2, 0);
    }

    /// The parameterised variant of [`Snapshot::assert_golden`].
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
                eprintln!(
                    "golden created: {} — read it over, then commit it",
                    path.display()
                );
            }
            return;
        }

        let golden = read_png(path);
        if (golden.width, golden.height) != (self.width, self.height) {
            let actual = actual_path(path);
            write_png(&actual, self);
            panic!(
                "golden {}: size {}×{} ≠ rendered {}×{} (the rendering was written to {})",
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
                "golden {}: {diff} pixel(s) differ (tolerance {max_diff_pixels}) — the \
                 rendering was written to {}; rerun with FRUS_UPDATE_GOLDENS=1 to accept it",
                path.display(),
                actual.display()
            );
        }
    }
}

/// `<folder>/<name>.actual.png`, alongside the golden.
fn actual_path(golden: &Path) -> std::path::PathBuf {
    golden.with_extension("actual.png")
}

fn write_png(path: &Path, snapshot: &Snapshot) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("creating the goldens folder");
    }
    let file =
        std::fs::File::create(path).unwrap_or_else(|e| panic!("creating {}: {e}", path.display()));
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        snapshot.width,
        snapshot.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header");
    writer
        .write_image_data(&snapshot.rgba)
        .expect("writing the PNG");
}

fn read_png(path: &Path) -> Snapshot {
    let file =
        std::fs::File::open(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("PNG info");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("PNG frame");
    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "the golden is not RGBA"
    );
    buf.truncate(info.buffer_size());
    Snapshot {
        width: info.width,
        height: info.height,
        rgba: buf,
    }
}

#[cfg(test)]
mod tests {
    use frus_core::Size;
    use frus_widgets::{
        build_ui, button, row, text, Align, Card, Checkbox, Container, Flex, Icon, Icons, Runtime,
        Theme, Widget,
    };

    /// Batching has to survive contact with a real widget tree, or the fix for
    /// milestone 291 would have traded a rendering bug for a frame-time one.
    ///
    /// This is a screen shaped like the ones frus draws: a card, rows, buttons with
    /// icons — rectangles and paths alternating all the way down. If the planner is
    /// sound, near enough all of them share two draw calls, because a button's
    /// background does not cover the previous button's icon. If it ever needs one per
    /// widget, this test says so long before a phone does.
    #[test]
    fn a_real_screen_still_batches() {
        let mut rows: Vec<Box<dyn Widget<()>>> = Vec::new();
        for i in 0..12 {
            rows.push(Box::new(
                row![
                    Checkbox::new(i % 2 == 0),
                    text(format!("Task number {i}")).size(15.0),
                    Container::new().flex(1.0),
                    Icon::new(Icons::Close).size(16.0),
                    button("Open", ()),
                ]
                .gap(12.0)
                .align(Align::Center)
                .height(48.0),
            ));
        }
        let mut list = Flex::column().gap(8.0);
        for r in rows {
            list = list.child_boxed(r);
        }
        let screen = Container::new()
            .width(400.0)
            .height(800.0)
            .padding(16.0)
            .child(Card::new().padding(12.0).child(list));

        let ui = build_ui(
            &screen,
            Size::new(400.0, 800.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let calls = frus_gpu::draw_calls(ui.scene());
        let primitives = ui.scene().primitives().len();
        // Three today — the card's rectangles, the ticks and icons above them, and
        // the buttons above those. The bound has a little room so that an honest
        // extra level does not fail the build, and none at all for one call a widget.
        assert!(
            calls <= 4,
            "{calls} draw calls for {primitives} primitives: batching broke down"
        );
    }
}
