//! **How long the application takes to show its first frame** (milestones 683 and 696, #211).
//!
//! A toolkit that draws itself pays at launch: a GPU to set up, pipelines to build, fonts to
//! read. Measured from the moment the process started where the system says when that was —
//! Linux and Android (`/proc`), Windows (the process's creation time), the web (the
//! navigation) — and from the shell's own start elsewhere. When the first frame is presented
//! one line is logged, which `frus run` shows on every platform:
//!
//! ```text
//! first frame at 312 ms after launch — shell at 8 ms; window at 35 ms; GPU ready at 245 ms
//! (adapter 40 ms, device 60 ms, surface 2 ms, pipelines 20 ms, warm-up 90 ms); fonts 80 ms
//! (2 loads); built in 4 ms; laid out at 290 ms (in 16 ms); presented in 22 ms
//! ```
//!
//! *Shell* is when the framework's code began to run: what is before it is the system
//! loading the program. *Built* is the application's first tree; *laid out* the layout and the
//! paint into a scene that follow it; *presented* the first scene drawn and handed to the
//! display.
//!
//! It is always on, because it costs nothing worth counting: a few clock readings, once. Once
//! the first frame is reported every mark is one atomic load and returns.
//!
//! `FRUS_EXIT_AFTER_FIRST_FRAME=1` ends the program right after the line is logged: what
//! `scripts/first-frame.sh` launches an application with, several times, to give the time a
//! median rather than one launch's luck.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// What was done when, as time since the launch, until the first frame is reported.
struct Startup {
    /// The framework's code began to run.
    shell: Option<Duration>,
    window: Option<Duration>,
    gpu: Option<(Duration, frus_gpu::StartupTimes)>,
    /// How long the first build of the tree took.
    built: Option<Duration>,
    /// When the first layout and paint ended, and how long they took.
    laid_out: Option<(Duration, Option<Duration>)>,
    /// The first frame was reported: nothing is recorded after it.
    done: bool,
}

impl Startup {
    const fn new() -> Self {
        Self {
            shell: None,
            window: None,
            gpu: None,
            built: None,
            laid_out: None,
            done: false,
        }
    }

    fn shell(&mut self, at: Duration) {
        if !self.done {
            self.shell.get_or_insert(at);
        }
    }

    fn window(&mut self, at: Duration) {
        if !self.done {
            self.window.get_or_insert(at);
        }
    }

    fn renderer(&mut self, at: Duration, times: frus_gpu::StartupTimes) {
        if !self.done {
            self.gpu.get_or_insert((at, times));
        }
    }

    fn built(&mut self, took: Duration) {
        if !self.done {
            self.built.get_or_insert(took);
        }
    }

    fn laid_out(&mut self, at: Duration, took: Option<Duration>) {
        if !self.done {
            self.laid_out.get_or_insert((at, took));
        }
    }

    /// A frame was presented at `at`, its drawing having taken `took`: the line for the
    /// first one, nothing for the others.
    fn presented(
        &mut self,
        at: Duration,
        took: Option<Duration>,
        fonts: (Duration, u32),
    ) -> Option<String> {
        if std::mem::replace(&mut self.done, true) {
            return None;
        }
        Some(self.report(at, took, fonts))
    }

    fn report(&self, at: Duration, took: Option<Duration>, fonts: (Duration, u32)) -> String {
        let ms = |d: Duration| d.as_millis();
        let mut parts = Vec::new();
        if let Some(shell) = self.shell {
            parts.push(format!("shell at {} ms", ms(shell)));
        }
        if let Some(window) = self.window {
            parts.push(format!("window at {} ms", ms(window)));
        }
        if let Some((ready, times)) = self.gpu {
            parts.push(format!(
                "GPU ready at {} ms (adapter {} ms, device {} ms, surface {} ms, pipelines {} ms, \
                 warm-up {} ms)",
                ms(ready),
                ms(times.adapter),
                ms(times.device),
                ms(times.surface),
                ms(times.pipelines),
                ms(times.warm_up)
            ));
        }
        if fonts.1 > 0 {
            parts.push(format!(
                "fonts {} ms ({} load{})",
                ms(fonts.0),
                fonts.1,
                if fonts.1 == 1 { "" } else { "s" }
            ));
        }
        if let Some(built) = self.built {
            parts.push(format!("built in {} ms", ms(built)));
        }
        match self.laid_out {
            Some((at, Some(took))) => {
                parts.push(format!("laid out at {} ms (in {} ms)", ms(at), ms(took)))
            }
            Some((at, None)) => parts.push(format!("laid out at {} ms", ms(at))),
            None => {}
        }
        if let Some(took) = took {
            parts.push(format!("presented in {} ms", ms(took)));
        }
        format!(
            "first frame at {} ms after launch — {}",
            ms(at),
            parts.join("; ")
        )
    }
}

static STARTUP: Mutex<Startup> = Mutex::new(Startup::new());

/// Whether the first frame is still to come: the one thing every mark reads, without a lock,
/// once it has been reported.
static PENDING: AtomicBool = AtomicBool::new(true);

/// Whether to end the program once the first frame is reported.
static EXIT: AtomicBool = AtomicBool::new(false);

/// The shell's own start, for a platform that does not say when the process started.
static BEGUN: OnceLock<web_time::Instant> = OnceLock::new();

/// Whether the first frame is still to come: a frame times its stages for it until then.
pub(crate) fn pending() -> bool {
    PENDING.load(Ordering::Relaxed)
}

/// Records a mark, while the first frame is still to come.
fn mark(record: impl FnOnce(&mut Startup)) {
    if !pending() {
        return;
    }
    if let Ok(mut startup) = STARTUP.lock() {
        record(&mut startup);
    }
}

/// The shell starts: the clock for a platform that cannot tell the process's start.
pub(crate) fn begin() {
    BEGUN.get_or_init(web_time::Instant::now);
    #[cfg(not(target_arch = "wasm32"))]
    EXIT.store(
        switched_on(std::env::var("FRUS_EXIT_AFTER_FIRST_FRAME").ok().as_deref()),
        Ordering::Relaxed,
    );
    mark(|s| s.shell(since_launch()));
}

/// The window exists.
pub(crate) fn window_made() {
    mark(|s| s.window(since_launch()));
}

/// The renderer is ready, with what making it took.
pub(crate) fn renderer_made(times: frus_gpu::StartupTimes) {
    mark(|s| s.renderer(since_launch(), times));
}

/// The interface was built, in `took`.
pub(crate) fn built(took: Duration) {
    mark(|s| s.built(took));
}

/// The interface is laid out and painted into a scene, which took `took` where it was timed.
pub(crate) fn laid_out(took: Option<Duration>) {
    mark(|s| s.laid_out(since_launch(), took));
}

/// A frame was presented, its drawing having taken `took` where it was timed: the first one
/// is reported.
pub(crate) fn presented(took: Option<Duration>) {
    if !pending() {
        return;
    }
    let line = match STARTUP.lock() {
        Ok(mut startup) => startup.presented(since_launch(), took, frus_text::font_loading()),
        Err(_) => None,
    };
    PENDING.store(false, Ordering::Relaxed);
    let Some(line) = line else {
        return;
    };
    log::info!("{line}");
    if EXIT.load(Ordering::Relaxed) {
        log::logger().flush();
        std::process::exit(0);
    }
}

/// An environment switch: set, and neither empty nor `0`. A page has no environment.
#[cfg(any(not(target_arch = "wasm32"), test))]
fn switched_on(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v != "0")
}

/// The time since the process started, where the system says when that was; since the
/// shell's start otherwise.
fn since_launch() -> Duration {
    process_age().unwrap_or_else(|| BEGUN.get_or_init(web_time::Instant::now).elapsed())
}

/// The web: the page's clock counts from the navigation.
#[cfg(target_arch = "wasm32")]
fn process_age() -> Option<Duration> {
    let now = web_sys::window()?.performance()?.now();
    Some(Duration::from_secs_f64(now / 1000.0))
}

/// Linux and Android: the process's start, in clock ticks since boot, against the time since
/// boot.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn process_age() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let uptime = std::fs::read_to_string("/proc/uptime").ok()?;
    age_from_proc(&stat, &uptime)
}

/// `/proc/self/stat`'s start time (the 22nd field, counted after the command's closing
/// parenthesis, which may itself hold spaces) against `/proc/uptime`. The clock ticks at
/// 100 Hz on Linux and Android.
#[cfg(any(target_os = "linux", target_os = "android", test))]
fn age_from_proc(stat: &str, uptime: &str) -> Option<Duration> {
    let after_command = &stat[stat.rfind(')')? + 1..];
    // The state is field 3, the first after the command: the start time is 19 further on.
    let started_ticks: f64 = after_command.split_whitespace().nth(19)?.parse().ok()?;
    let up: f64 = uptime.split_whitespace().next()?.parse().ok()?;
    let age = up - started_ticks / 100.0;
    (age >= 0.0).then(|| Duration::from_secs_f64(age))
}

/// Windows: the process's creation time against the clock, both in 100 ns since 1601.
#[cfg(windows)]
fn process_age() -> Option<Duration> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user, mut now) = (zero, zero, zero, zero, zero);
    // SAFETY: the current process's pseudo-handle, and five FILETIMEs written by the calls.
    let found = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    };
    if found == 0 {
        return None;
    }
    // SAFETY: writes one FILETIME.
    unsafe { GetSystemTimeAsFileTime(&mut now) };
    let age = ticks(now).checked_sub(ticks(created))?;
    Some(Duration::from_nanos(age * 100))
}

/// Elsewhere (macOS, iOS): since the shell's start.
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "linux",
    target_os = "android",
    windows
)))]
fn process_age() -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn gpu() -> frus_gpu::StartupTimes {
        frus_gpu::StartupTimes {
            adapter: ms(40),
            device: ms(60),
            surface: ms(2),
            pipelines: ms(20),
            warm_up: ms(90),
        }
    }

    /// Every stage marked, in the order a launch goes through them.
    fn marked() -> Startup {
        let mut s = Startup::new();
        s.shell(ms(8));
        s.window(ms(35));
        s.renderer(ms(245), gpu());
        s.built(ms(4));
        s.laid_out(ms(290), Some(ms(16)));
        s
    }

    #[test]
    fn the_process_age_is_read_from_proc() {
        // A command with a space and a parenthesis in it, as `/proc` gives it.
        let stat = "4242 (my app (x)) S 1 4242 4242 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 3 0 \
                    123456 1000 50";
        // Started at tick 123456 (1234.56 s after boot); the system has been up 1235.06 s.
        let age = age_from_proc(stat, "1235.06 4000.00\n").unwrap();
        assert!((age.as_secs_f64() - 0.5).abs() < 1e-6, "{age:?}");
        assert_eq!(age_from_proc("garbage", "1.0"), None);
    }

    /// **The line names each stage**, in the order a launch goes through them — the format
    /// `scripts/first-frame.sh` reads.
    #[test]
    fn the_report_names_each_stage() {
        let line = marked()
            .presented(ms(312), Some(ms(22)), (ms(80), 2))
            .expect("the first frame is reported");
        assert_eq!(
            line,
            "first frame at 312 ms after launch — shell at 8 ms; window at 35 ms; GPU ready at \
             245 ms (adapter 40 ms, device 60 ms, surface 2 ms, pipelines 20 ms, warm-up 90 ms); \
             fonts 80 ms (2 loads); built in 4 ms; laid out at 290 ms (in 16 ms); presented in \
             22 ms"
        );
    }

    /// **A stage that did not happen is left out**, rather than shown as zero: no fonts read,
    /// a layout that was not timed, a platform where the window is the page.
    #[test]
    fn a_stage_not_marked_is_left_out() {
        let mut s = Startup::new();
        s.laid_out(ms(50), None);
        let line = s.presented(ms(60), None, (ms(0), 0)).unwrap();
        assert_eq!(
            line,
            "first frame at 60 ms after launch — laid out at 50 ms"
        );
        let mut one = Startup::new();
        let line = one.presented(ms(9), None, (ms(3), 1)).unwrap();
        assert!(line.ends_with("fonts 3 ms (1 load)"), "{line}");
    }

    /// **Only the first frame is reported**, and only the first of each stage counts: a
    /// renderer made again after a lost device, or the next frame's layout, change nothing.
    #[test]
    fn only_the_first_of_each_counts() {
        let mut s = marked();
        s.window(ms(1000));
        s.renderer(ms(2000), frus_gpu::StartupTimes::default());
        s.built(ms(500));
        s.laid_out(ms(3000), Some(ms(700)));
        let line = s.presented(ms(312), Some(ms(22)), (ms(80), 2)).unwrap();
        assert!(line.contains("window at 35 ms;"), "{line}");
        assert!(line.contains("GPU ready at 245 ms"), "{line}");
        assert!(line.contains("built in 4 ms;"), "{line}");
        assert!(line.contains("laid out at 290 ms (in 16 ms)"), "{line}");
        assert_eq!(s.presented(ms(400), Some(ms(1)), (ms(80), 2)), None);
    }

    /// **After the first frame, nothing is kept**: the marks of later frames leave it as it
    /// was.
    #[test]
    fn after_the_first_frame_nothing_is_kept() {
        let mut s = Startup::new();
        assert!(s.presented(ms(10), None, (ms(0), 0)).is_some());
        s.shell(ms(1));
        s.window(ms(2));
        s.renderer(ms(3), gpu());
        s.built(ms(4));
        s.laid_out(ms(5), None);
        assert_eq!(s.shell, None);
        assert_eq!(s.window, None);
        assert!(s.gpu.is_none());
        assert_eq!(s.built, None);
        assert_eq!(s.laid_out, None);
    }

    /// The exit switch: on when set to anything but nothing or `0`.
    #[test]
    fn the_exit_switch() {
        assert!(!switched_on(None));
        assert!(!switched_on(Some("")));
        assert!(!switched_on(Some("0")));
        assert!(switched_on(Some("1")));
        assert!(switched_on(Some("yes")));
    }
}
