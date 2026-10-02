//! **How smooth the frames are, measured where they are drawn** (milestone 609).
//!
//! A phone's own frame statistics do not always reach a native surface — the test device's
//! compositor kept none for it — so the shell keeps its own. Each frame records the time
//! since the one before it, and how long it spent building the tree, laying it out and
//! painting it into a scene, and rendering that scene, which includes waiting for the GPU
//! and the display. Every two seconds of frames drawn without a pause, one line goes to the
//! log under the target `frus::frames`:
//!
//! ```text
//! 36 frames in 1.0 s (37 fps): interval 16.5/57.9/58.7 ms, 12 over budget; no rebuild,
//! layout+paint 6.4/13.2 ms, render 10.2/49.4 ms (waiting for an image 0.5/0.8, drawing
//! 2.3/5.5) (median/95th or max); 2 layers, up to 3; next frame asked by runtime 35 (moving:
//! scroll 34, glow 22, anims 3, ink 1)
//! ```
//!
//! The intervals say how smooth it was; the stages say where a slow frame went. Rendering
//! is split once more, because the wait for a surface image and the drawing are the CPU's,
//! and what is left of the render is the CPU held back by a GPU still on earlier frames.
//! The last part says what kept the frames coming: a screen that never stops drawing names
//! the motion that never settles.
//!
//! It is off unless asked for, and costs nothing then: `FRUS_FRAME_STATS=1` in the
//! environment, or on Android `adb shell setprop debug.frus.frames 1` before the
//! application starts.

use std::time::Duration;
// The browser has no `std` clock: the same one the shell keeps time with.
use web_time::Instant;

/// A pause longer than this ends a run of frames: what follows is not the same motion.
const PAUSE: Duration = Duration::from_millis(250);
/// How long a run is summarised over.
const WINDOW: Duration = Duration::from_secs(2);
/// A frame later than this past the one before it was missed by the display: 1.5 frames of
/// 60 Hz.
const LATE_MS: f32 = 25.0;

/// What asked for the next frame: the reasons a frame was followed by another, so that a
/// screen that never stops drawing says why.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Why(u8);

impl Why {
    pub(crate) const NONE: Why = Why(0);
    /// The application's own `tick`, or the theme crossing.
    pub(crate) const APP: Why = Why(1);
    /// A transition the runtime keeps: a value, a colour, a size, a scroll in flight…
    pub(crate) const RUNTIME: Why = Why(2);
    /// A widget that draws continuously: a spinner, a skeleton, a chart easing in.
    pub(crate) const WIDGET: Why = Why(4);
    /// An image or a task still on its way.
    pub(crate) const LOADING: Why = Why(8);
    /// The pointer moved onto something new, or the keyboard revealed a field.
    pub(crate) const POINTER: Why = Why(16);

    const NAMES: [(Why, &'static str); 5] = [
        (Why::APP, "app"),
        (Why::RUNTIME, "runtime"),
        (Why::WIDGET, "widget"),
        (Why::LOADING, "loading"),
        (Why::POINTER, "pointer"),
    ];

    fn has(self, other: Why) -> bool {
        self.0 & other.0 != 0
    }
}

impl std::ops::BitOr for Why {
    type Output = Why;
    fn bitor(self, other: Why) -> Why {
        Why(self.0 | other.0)
    }
}

/// One frame's costs, in milliseconds.
#[derive(Clone, Copy, Default)]
pub(crate) struct FrameCosts {
    /// Building the tree; `None` when the frame reused the last one.
    pub(crate) build: Option<f32>,
    /// Laying the tree out and painting it into a scene.
    pub(crate) layout_paint: f32,
    /// Rendering the scene: encoding, submitting, and waiting for the GPU and the display.
    pub(crate) render: f32,
    /// Of that, waiting for a surface image to draw into.
    pub(crate) acquire: f32,
    /// Of that, tessellating, encoding and submitting.
    pub(crate) draw: f32,
    /// What the GPU took, by its own clock, where it has one (milestone 612).
    pub(crate) gpu: Option<f32>,
    /// How many layers the scene holds, nested ones included: each is a pass of its own
    /// on the GPU, into a texture the size of the surface.
    pub(crate) layers: u32,
    /// What asked for the frame after it.
    pub(crate) why: Why,
}

/// The frames of the current run.
pub(crate) struct FrameStats {
    enabled: bool,
    run_start: Option<Instant>,
    last: Option<Instant>,
    intervals: Vec<f32>,
    costs: Vec<FrameCosts>,
    /// For each family of retained motion, how many frames of the run it was still moving in.
    moving: Vec<(&'static str, usize)>,
}

impl FrameStats {
    /// Off unless the platform's switch is on.
    pub(crate) fn new() -> Self {
        Self::with(switched_on())
    }

    pub(crate) fn with(enabled: bool) -> Self {
        Self {
            enabled,
            run_start: None,
            last: None,
            intervals: Vec::new(),
            costs: Vec::new(),
            moving: Vec::new(),
        }
    }

    /// Which families of retained motion are still moving this frame, by name: how a
    /// screen that never stops drawing says which one keeps it going.
    pub(crate) fn note_moving(&mut self, families: &[(&'static str, bool)]) {
        if !self.enabled {
            return;
        }
        for (name, on) in families.iter().filter(|(_, on)| *on) {
            let _ = on;
            match self.moving.iter_mut().find(|(n, _)| n == name) {
                Some((_, count)) => *count += 1,
                None => self.moving.push((name, 1)),
            }
        }
    }

    /// Whether frames are being measured: a frame need not time itself otherwise.
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    /// A frame that began at `start` and cost `costs`. Returns the line logged, if this
    /// frame closed a run's window.
    pub(crate) fn frame(&mut self, start: Instant, costs: FrameCosts) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let mut report = None;
        if let Some(last) = self.last {
            let gap = start.saturating_duration_since(last);
            if gap > PAUSE {
                report = self.flush(last);
            } else {
                self.intervals.push(gap.as_secs_f32() * 1000.0);
            }
        }
        if self.run_start.is_none() {
            self.run_start = Some(start);
        }
        self.costs.push(costs);
        self.last = Some(start);
        if self
            .run_start
            .is_some_and(|run| start.saturating_duration_since(run) >= WINDOW)
        {
            report = self.flush(start).or(report);
        }
        if let Some(line) = &report {
            log::info!(target: "frus::frames", "{line}");
        }
        report
    }

    /// Summarises the run that ended at `end`, and starts the next.
    fn flush(&mut self, end: Instant) -> Option<String> {
        let run = self.run_start.take()?;
        let costs = std::mem::take(&mut self.costs);
        let intervals = std::mem::take(&mut self.intervals);
        let mut moving = std::mem::take(&mut self.moving);
        // A run of one or two frames says nothing about smoothness: a click's redraw.
        if intervals.len() < 2 {
            return None;
        }
        let seconds = end.saturating_duration_since(run).as_secs_f32().max(1e-3);
        let late = intervals.iter().filter(|&&ms| ms > LATE_MS).count();
        let builds: Vec<f32> = costs.iter().filter_map(|c| c.build).collect();
        let layout: Vec<f32> = costs.iter().map(|c| c.layout_paint).collect();
        let render: Vec<f32> = costs.iter().map(|c| c.render).collect();
        let (i50, i95, imax) = spread(&intervals);
        let (l50, l95, _) = spread(&layout);
        let (r50, r95, _) = spread(&render);
        let (a50, a95, _) = spread(&costs.iter().map(|c| c.acquire).collect::<Vec<_>>());
        let (d50, d95, _) = spread(&costs.iter().map(|c| c.draw).collect::<Vec<_>>());
        let gpu: Vec<f32> = costs.iter().filter_map(|c| c.gpu).collect();
        let gpu = if gpu.is_empty() {
            String::new()
        } else {
            let (g50, g95, gmax) = spread(&gpu);
            format!(", on the GPU {g50:.1}/{g95:.1}/{gmax:.1}")
        };
        let (y50, _, ymax) = spread(&costs.iter().map(|c| c.layers as f32).collect::<Vec<_>>());
        // Which reasons kept the frames coming, and in how many of them.
        let asked: Vec<String> = Why::NAMES
            .iter()
            .filter_map(|(why, name)| {
                let n = costs.iter().filter(|c| c.why.has(*why)).count();
                (n > 0).then(|| format!("{name} {n}"))
            })
            .collect();
        let mut asked = if asked.is_empty() {
            "nothing".to_string()
        } else {
            asked.join(", ")
        };
        if !moving.is_empty() {
            moving.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            let named: Vec<String> = moving.iter().map(|(n, c)| format!("{n} {c}")).collect();
            asked = format!("{asked} (moving: {})", named.join(", "));
        }
        let build = if builds.is_empty() {
            "no rebuild".to_string()
        } else {
            let (b50, b95, _) = spread(&builds);
            format!(
                "build {b50:.1}/{b95:.1} ms ({} of {})",
                builds.len(),
                costs.len()
            )
        };
        Some(format!(
            "{} frames in {seconds:.1} s ({:.0} fps): interval {i50:.1}/{i95:.1}/{imax:.1} ms, \
             {late} over budget; {build}, layout+paint {l50:.1}/{l95:.1} ms, render \
             {r50:.1}/{r95:.1} ms (waiting for an image {a50:.1}/{a95:.1}, drawing \
             {d50:.1}/{d95:.1}{gpu}) (median/95th or max); {y50:.0} layers, up to {ymax:.0}; next \
             frame asked by {asked}",
            costs.len(),
            intervals.len() as f32 / seconds,
        ))
    }
}

/// The median, the 95th percentile and the largest of `values`.
fn spread(values: &[f32]) -> (f32, f32, f32) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    let at = |p: f32| sorted[((sorted.len() - 1) as f32 * p).round() as usize];
    (at(0.5), at(0.95), sorted[sorted.len() - 1])
}

/// Whether the platform's switch is on.
fn switched_on() -> bool {
    if std::env::var("FRUS_FRAME_STATS").is_ok_and(|v| !v.is_empty() && v != "0") {
        return true;
    }
    // An Android application has no environment to be given: a system property, set with
    // `adb shell setprop debug.frus.frames 1`, read once at start.
    #[cfg(target_os = "android")]
    {
        let read = std::process::Command::new("/system/bin/getprop")
            .arg("debug.frus.frames")
            .output();
        if let Ok(out) = read {
            let value = String::from_utf8_lossy(&out.stdout);
            let value = value.trim();
            return !value.is_empty() && value != "0";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn costs() -> FrameCosts {
        FrameCosts {
            build: Some(1.0),
            layout_paint: 2.0,
            render: 3.0,
            acquire: 0.5,
            draw: 2.0,
            gpu: None,
            layers: 0,
            why: Why::WIDGET,
        }
    }

    /// **Two seconds of frames make one line**: how many, how often, how many were late,
    /// and what each stage cost.
    #[test]
    fn a_run_of_frames_is_summarised_every_two_seconds() {
        let mut stats = FrameStats::with(true);
        let t0 = Instant::now();
        let mut line = None;
        // 121 frames 1/60 s apart, one of them 50 ms late.
        let mut at = t0;
        for i in 0..121 {
            let step = if i == 60 { 50.0 } else { 1000.0 / 60.0 };
            at += Duration::from_secs_f32(step / 1000.0);
            if let Some(l) = stats.frame(at, costs()) {
                line = Some(l);
            }
        }
        let line = line.expect("a line after two seconds");
        assert!(line.contains("1 over budget"), "{line}");
        assert!(line.contains("build 1.0/1.0 ms"), "{line}");
        assert!(line.contains("render 3.0/3.0 ms"), "{line}");
        assert!(line.contains("interval 16.7/"), "{line}");
        assert!(line.ends_with("asked by widget 120"), "{line}");
    }

    /// **A pause ends a run**: frames on either side of it are not one motion, and the
    /// pause is not counted as a late frame.
    #[test]
    fn a_pause_ends_a_run() {
        let mut stats = FrameStats::with(true);
        let t0 = Instant::now();
        for i in 0..10 {
            stats.frame(t0 + Duration::from_millis(16 * i), costs());
        }
        let line = stats
            .frame(t0 + Duration::from_secs(5), costs())
            .expect("the pause closes the run");
        assert!(line.starts_with("10 frames"), "{line}");
        assert!(line.contains("0 over budget"), "{line}");
    }

    /// **The families still moving are named**, most frames first.
    #[test]
    fn the_families_still_moving_are_named() {
        let mut stats = FrameStats::with(true);
        let t0 = Instant::now();
        let mut line = None;
        for i in 0..130u64 {
            stats.note_moving(&[("scroll", i % 2 == 0), ("anims", true), ("ink", false)]);
            if let Some(l) = stats.frame(t0 + Duration::from_millis(16 * i), costs()) {
                line = Some(l);
            }
        }
        let line = line.expect("a line");
        assert!(line.contains("(moving: anims 1"), "{line}");
        assert!(line.contains(", scroll "), "{line}");
        assert!(!line.contains("ink"), "{line}");
    }

    /// **Off, it records nothing.**
    #[test]
    fn off_it_records_nothing() {
        let mut stats = FrameStats::with(false);
        let t0 = Instant::now();
        for i in 0..200 {
            assert!(stats
                .frame(t0 + Duration::from_millis(16 * i), costs())
                .is_none());
        }
        assert!(stats.costs.is_empty());
    }
}
