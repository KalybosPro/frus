//! **How long the application takes to show its first frame** (milestone 683, #211).
//!
//! A toolkit that draws itself pays at launch: a GPU to set up, pipelines to build, fonts to
//! read. Measured from the moment the process started where the system says when that was —
//! Linux and Android (`/proc`), Windows (the process's creation time), the web (the
//! navigation) — and from the shell's own start elsewhere. When the first frame is presented
//! one line is logged, which `frus run` shows on every platform:
//!
//! ```text
//! first frame at 312 ms after launch — window at 35 ms; GPU ready at 245 ms (adapter 40 ms,
//! device 60 ms, pipelines 20 ms, warm-up 90 ms); fonts 80 ms (2 loads); laid out at 290 ms
//! ```

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// What was done when, as time since the launch.
struct Marks {
    window: Option<Duration>,
    gpu: Option<(Duration, frus_gpu::StartupTimes)>,
    laid_out: Option<Duration>,
    reported: bool,
}

static MARKS: Mutex<Marks> = Mutex::new(Marks {
    window: None,
    gpu: None,
    laid_out: None,
    reported: false,
});

/// The shell's own start, for a platform that does not say when the process started.
static BEGUN: OnceLock<web_time::Instant> = OnceLock::new();

/// The shell starts: the clock for a platform that cannot tell the process's start.
pub(crate) fn begin() {
    BEGUN.get_or_init(web_time::Instant::now);
}

/// The window exists.
pub(crate) fn window_made() {
    if let Ok(mut marks) = MARKS.lock() {
        marks.window.get_or_insert_with(since_launch);
    }
}

/// The renderer is ready, with what making it took.
pub(crate) fn renderer_made(times: frus_gpu::StartupTimes) {
    if let Ok(mut marks) = MARKS.lock() {
        marks.gpu.get_or_insert_with(|| (since_launch(), times));
    }
}

/// The interface is built and laid out.
pub(crate) fn laid_out() {
    if let Ok(mut marks) = MARKS.lock() {
        marks.laid_out.get_or_insert_with(since_launch);
    }
}

/// A frame was presented: the first one is reported.
pub(crate) fn presented() {
    let Ok(mut marks) = MARKS.lock() else {
        return;
    };
    if marks.reported {
        return;
    }
    marks.reported = true;
    let now = since_launch();
    log::info!("{}", report(now, &marks, frus_text::font_loading()));
}

fn report(at: Duration, marks: &Marks, fonts: (Duration, u32)) -> String {
    let ms = |d: Duration| d.as_millis();
    let mut parts = Vec::new();
    if let Some(window) = marks.window {
        parts.push(format!("window at {} ms", ms(window)));
    }
    if let Some((ready, times)) = marks.gpu {
        parts.push(format!(
            "GPU ready at {} ms (adapter {} ms, device {} ms, pipelines {} ms, warm-up {} ms)",
            ms(ready),
            ms(times.adapter),
            ms(times.device),
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
    if let Some(laid_out) = marks.laid_out {
        parts.push(format!("laid out at {} ms", ms(laid_out)));
    }
    format!(
        "first frame at {} ms after launch — {}",
        ms(at),
        parts.join("; ")
    )
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

    #[test]
    fn the_report_names_each_step() {
        let marks = Marks {
            window: Some(Duration::from_millis(35)),
            gpu: Some((
                Duration::from_millis(245),
                frus_gpu::StartupTimes {
                    adapter: Duration::from_millis(40),
                    device: Duration::from_millis(60),
                    pipelines: Duration::from_millis(20),
                    warm_up: Duration::from_millis(90),
                },
            )),
            laid_out: Some(Duration::from_millis(290)),
            reported: false,
        };
        assert_eq!(
            report(
                Duration::from_millis(312),
                &marks,
                (Duration::from_millis(80), 2)
            ),
            "first frame at 312 ms after launch — window at 35 ms; GPU ready at 245 ms \
             (adapter 40 ms, device 60 ms, pipelines 20 ms, warm-up 90 ms); fonts 80 ms \
             (2 loads); laid out at 290 ms"
        );
    }
}
