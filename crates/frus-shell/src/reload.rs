//! **State-preserving live reload** (§13, development only).
//!
//! The principle: Elm state is **one single struct**, so serialising it is enough to
//! survive a binary being replaced. Under `FRUS_WATCH=1`, in debug builds, the shell
//! watches the mtime of its own executable; when `cargo build` — or `cargo watch` —
//! replaces it, the shell captures [`Application::save_state`], relaunches the **new**
//! binary with the snapshot's path in the environment, and steps aside. The new
//! process rehydrates through [`Application::restore_state`] before `init`.
//!
//! Pure `cargo`: no ABI to stabilise, no proprietary tooling — `FRUS_WATCH=1 cargo
//! run` in one terminal, `cargo watch -x build` in another, and the edit-run loop
//! keeps its state across a code change.
//!
//! [`Application::save_state`]: crate::Application::save_state
//! [`Application::restore_state`]: crate::Application::restore_state

// `Path` only serves `restore_from_env`, which is desktop-only; `PathBuf` serves
// everywhere the module compiles, through `state_file_path`.
#[cfg(desktop)]
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use web_time::Instant;

/// The environment variable that **turns on** watching, in development.
const WATCH_ENV: &str = "FRUS_WATCH";
/// The environment variable carrying the path of the snapshot to rehydrate.
const STATE_ENV: &str = "FRUS_HOT_STATE";
/// How often the executable's mtime is checked.
const POLL_INTERVAL: Duration = Duration::from_millis(700);

/// Watches the current binary and orchestrates the relaunch.
pub(crate) struct ReloadWatcher {
    /// The executable's path, **captured at startup**: after a replacement,
    /// `/proc/self/exe` would point at the deleted inode ("(deleted)"), whereas the
    /// original path names the freshly compiled binary.
    exe: PathBuf,
    mtime: SystemTime,
    next_poll: Instant,
}

impl ReloadWatcher {
    /// `Some` only in a debug build, under `FRUS_WATCH=1`, and only when the
    /// executable can be observed.
    pub(crate) fn new() -> Option<Self> {
        if !cfg!(debug_assertions) || std::env::var_os(WATCH_ENV).map_or(true, |v| v != "1") {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let mtime = std::fs::metadata(&exe).ok()?.modified().ok()?;
        Some(Self {
            exe,
            mtime,
            next_poll: Instant::now() + POLL_INTERVAL,
        })
    }

    /// The next check's deadline, for `ControlFlow::WaitUntil`.
    pub(crate) fn deadline(&self) -> Instant {
        self.next_poll
    }

    /// Checks whether the binary was replaced, at most once per interval.
    pub(crate) fn binary_changed(&mut self) -> bool {
        let now = Instant::now();
        if now < self.next_poll {
            return false;
        }
        self.next_poll = now + POLL_INTERVAL;
        match std::fs::metadata(&self.exe).and_then(|m| m.modified()) {
            Ok(mtime) if mtime != self.mtime => {
                self.mtime = mtime;
                true
            }
            _ => false,
        }
    }

    /// Relaunches the recompiled binary and steps aside: writes the snapshot when
    /// there is one, passes it along through the environment, spawns the new process
    /// and ends this one. With no snapshot the relaunch starts from a fresh state.
    pub(crate) fn handoff(&self, state: Option<Vec<u8>>) -> ! {
        let mut command = std::process::Command::new(&self.exe);
        if let Some(bytes) = state {
            let path = state_file_path();
            if std::fs::write(&path, bytes).is_ok() {
                command.env(STATE_ENV, &path);
            }
        }
        match command.spawn() {
            Ok(_) => eprintln!(
                "[frus] binary recompiled: relaunching ({})",
                self.exe.display()
            ),
            Err(err) => eprintln!("[frus] cannot relaunch: {err}"),
        }
        std::process::exit(0);
    }
}

/// The state snapshot's path, unique per parent process.
fn state_file_path() -> PathBuf {
    std::env::temp_dir().join(format!("frus-hot-{}.state", std::process::id()))
}

/// At boot: rehydrates the state from the snapshot the previous binary left behind,
/// if there is one; the file is consumed. Call this before `init`. Desktop only —
/// live reload makes no sense on Android or the Web, where `run`, its only caller,
/// does not exist.
#[cfg(desktop)]
pub(crate) fn restore_from_env<A: crate::Application>(app: &mut A) {
    let Some(path) = std::env::var_os(STATE_ENV) else {
        return;
    };
    let path = Path::new(&path);
    if let Ok(bytes) = std::fs::read(path) {
        app.restore_state(&bytes);
        eprintln!("[frus] state rehydrated ({} bytes)", bytes.len());
    }
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::{Theme, Widget};

    struct Probe {
        restored: Option<Vec<u8>>,
    }

    impl crate::Application for Probe {
        type Message = ();
        fn update(&mut self, _: ()) -> crate::Command<()> {
            crate::Command::none()
        }
        fn view(&self, _: &Theme, _: f32, _: f32) -> Box<dyn Widget<()>> {
            unimplemented!("never rendered in this test")
        }
        fn restore_state(&mut self, bytes: &[u8]) {
            self.restored = Some(bytes.to_vec());
        }
    }

    /// The snapshot the environment points at is rehydrated then consumed; with no
    /// variable set, nothing happens.
    #[test]
    fn restore_reads_and_consumes_the_snapshot() {
        let path = std::env::temp_dir().join("frus-hot-test.state");
        std::fs::write(&path, b"snapshot").unwrap();
        std::env::set_var(STATE_ENV, &path);
        let mut app = Probe { restored: None };
        restore_from_env(&mut app);
        std::env::remove_var(STATE_ENV);
        assert_eq!(app.restored.as_deref(), Some(&b"snapshot"[..]));
        assert!(!path.exists(), "the snapshot is consumed");

        let mut fresh = Probe { restored: None };
        restore_from_env(&mut fresh);
        assert!(fresh.restored.is_none(), "no variable: a fresh start");
    }

    /// The watcher exists only under `FRUS_WATCH=1`, and reports a change only after
    /// a genuine mtime jump, once the interval has elapsed.
    #[test]
    fn watcher_requires_opt_in_and_tracks_mtime() {
        std::env::remove_var(WATCH_ENV);
        assert!(ReloadWatcher::new().is_none(), "opting in is mandatory");

        std::env::set_var(WATCH_ENV, "1");
        let watcher = ReloadWatcher::new();
        std::env::remove_var(WATCH_ENV);
        // In debug the test binary can be observed → Some.
        let Some(mut watcher) = watcher else {
            panic!("a watcher was expected in debug under FRUS_WATCH=1");
        };
        // The interval holds the first check back.
        assert!(!watcher.binary_changed());
        // Once the deadline has passed, with no recompilation: still nothing.
        watcher.next_poll = Instant::now() - Duration::from_millis(1);
        assert!(!watcher.binary_changed(), "mtime unchanged → no relaunch");
    }
}
