//! `frus run`: build the application for a device, start it there, and show what it logs —
//! the desktop's terminal output, Android's logcat, the browser's console — in this terminal
//! until it ends.
//!
//! While it runs, keys drive it. `r` **reloads** it: it is built again, and the new build
//! opens where the old one was — the same page, the window in the same place — with the state
//! of every component that keeps one (`State::save`). `R` **restarts** it the same way but with
//! every state afresh. `q` quits. A build that fails leaves the running application as it was.
//!
//! The other keys (milestone 668) look into the running application or change how it shows: the
//! inspector, debug paint, the widget, focus and semantics trees, another platform, the other
//! brightness, a performance panel, a screenshot. The application is told through the same
//! channel it reports on (see `devlink`), and carries them out on its next frame.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::cargo::{self, Mode};
use crate::devices::{self, Device, Kind};
use crate::devlink::{DevLink, Report};
use crate::json::object;
use crate::keys;
use crate::project::Project;
use crate::report;
use crate::serve;

/// What `frus run` was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunOptions {
    /// `-d`: a device's id, name or platform. None: the desktop.
    pub device: Option<String>,
    pub mode: Mode,
    /// The port the web build is served on, when it is free.
    pub port: u16,
    /// Everything after `--`, handed to the application (desktop only).
    pub args: Vec<String>,
    /// `--machine`: JSON lines for an editor instead of text for a person (see `report`).
    pub machine: bool,
}

pub(crate) const DEFAULT_PORT: u16 = 8080;

/// What a key asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// A new build, where the old one was, every state afresh.
    Restart,
    /// A new build, where the old one was, with the states that are kept.
    Reload,
    Quit,
    Help,
    /// `frus run` ends and the application goes on running.
    Detach,
    /// The terminal is cleared.
    Clear,
    /// The screen, as a PNG beside the application: `frus_01.png`, `frus_02.png`…
    Screenshot,
    /// Something the application does itself, by the command it is sent (`frus-shell`'s
    /// `dev_commands`).
    App(&'static str),
    /// A key of the reference's tool that has nothing to do here, and why.
    Missing(&'static str),
}

pub(crate) fn action(key: char) -> Option<Action> {
    Some(match key {
        'R' => Action::Restart,
        'r' => Action::Reload,
        'q' | 'Q' => Action::Quit,
        'h' | 'H' | '?' => Action::Help,
        'd' => Action::Detach,
        'c' => Action::Clear,
        's' => Action::Screenshot,
        'w' => Action::App("widgets"),
        'f' => Action::App("focus"),
        'S' => Action::App("semantics"),
        'i' => Action::App("inspector"),
        'p' => Action::App("outlines"),
        'o' => Action::App("platform"),
        'b' => Action::App("brightness"),
        'P' => Action::App("performance"),
        't' => Action::Missing("frus has a single tree: `w` prints it, every widget with its box"),
        'L' => Action::Missing("frus has no layer tree: a frame is one scene, painted in order"),
        'U' => Action::Missing("frus hit-tests one way only: there is no order to invert"),
        'I' => Action::Missing("frus does not mark oversized images yet"),
        'a' => Action::Missing("frus records no timeline yet: `P` shows what each frame costs"),
        'M' => Action::Missing("frus compiles no shaders at run time: there is nothing to write"),
        'g' => Action::Missing("frus has no code generators to run"),
        'v' => Action::Missing("frus has no DevTools yet: `i` is its inspector"),
        _ => return None,
    })
}

// No line continuation after the opening quote: it would eat this line's indentation too.
const KEYS: &str = "  r  reload: build again, start where it was, with the states it keeps
  R  restart: build again, start where it was, every state afresh
  h  this list
  d  detach: frus run ends, the application goes on
  c  clear the screen
  q  quit
  s  save a screenshot to frus_01.png (02, 03…)
  w  print the widget tree, every widget with its box
  f  print the focus tree, in tab order
  S  print the semantics tree, in reading order
  i  inspector: outlines, and a card for the widget under the pointer
  p  debug paint: outlines around every widget
  o  switch the platform the widgets behave as (Android, iOS, Windows…)
  b  switch between light and dark
  P  performance panel: what each frame costs
";

/// Clears the terminal, its scrollback too, and puts the cursor at the top.
fn clear_screen() {
    if !report::is_machine() {
        print!("\x1b[2J\x1b[3J\x1b[H");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

/// Where the next screenshot goes: the first `frus_NN.png` not taken in `dir`.
pub(crate) fn screenshot_path(dir: &Path) -> std::path::PathBuf {
    (1..)
        .map(|n| dir.join(format!("frus_{n:02}.png")))
        .find(|path| !path.exists())
        .unwrap_or_else(|| dir.join("frus.png"))
}

/// Says where a screenshot went, once it is there: the application writes it on its next frame.
fn report_screenshot(path: &Path, wait: Duration) {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > 0 {
                report::say(&format!(
                    "Screenshot written to {} ({} kB).",
                    file_name(path),
                    meta.len().div_ceil(1024)
                ));
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    report::warn("the screenshot did not arrive: is the application still drawing?");
}

pub(crate) fn run(options: &RunOptions) -> Result<ExitCode, String> {
    // For an editor, from the very start: even a run that cannot begin ends with `app.stop`.
    if options.machine {
        report::machine(format!("frus-run-{}", std::process::id()));
    }
    let result = run_session(options);
    keys::restore();
    report::event(
        "app.stop",
        object([("error", result.as_ref().err().map(String::as_str).into())]),
    );
    result
}

fn run_session(options: &RunOptions) -> Result<ExitCode, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = Project::find(&here)?;
    let devices = devices::discover();
    let device = devices::select(&devices, options.device.as_deref())?;
    if options.machine {
        report::event(
            "app.start",
            object([
                ("deviceId", device.id.as_str().into()),
                ("deviceName", device.name.as_str().into()),
                ("directory", project.root.display().to_string().into()),
                ("supportsRestart", true.into()),
            ]),
        );
    }
    if options.device.is_none() && devices.iter().any(|d| d.platform != device.platform) {
        report::say(&format!(
            "Running on {}. To run elsewhere: `frus run -d <device>` (`frus devices` lists them).",
            device.name
        ));
    }
    let result = match &device.kind {
        Kind::Host => run_desktop(&project, options),
        Kind::Android { serial } => run_android(&project, &device, serial, options.mode),
        Kind::Emulator { avd } => {
            let serial = start_emulator(avd)?;
            run_android(&project, &device, &serial, options.mode)
        }
        Kind::IosSimulator { .. } => Err(format!(
            "{} is an iOS simulator, and frus cannot run on iOS yet: the iOS shell does not \
             exist (ROADMAP, issue #19). `frus run` will start iOS applications once it does.",
            device.name
        )),
        Kind::Browser { exe } => run_web(&project, Some(exe), options),
        Kind::WebServer => run_web(&project, None, options),
    };
    result
}

/// Says a restart is done, and how long it took.
fn restarted(action: Action, started: Instant, more: &str) {
    let seconds = started.elapsed().as_secs_f32();
    if report::is_machine() {
        report::event(
            "app.restarted",
            object([
                ("fullRestart", (action == Action::Restart).into()),
                ("seconds", f64::from(seconds).into()),
            ]),
        );
    } else {
        report::say(&format!("Restarted in {seconds:.1} s{more}."));
    }
}

/// The keys, from the moment the application is up.
fn attach(what: &str) -> Receiver<char> {
    if report::is_machine() {
        report::event("app.started", object([]));
    } else {
        report::say(&format!("\n{what}. Keys:\n{KEYS}"));
    }
    keys::listen()
}

/// The next key's action, waiting at most `wait`. Keys that mean nothing are skipped.
fn next_action(keys: &Receiver<char>, wait: Duration) -> Option<Action> {
    match keys.recv_timeout(wait) {
        Ok(key) => action(key),
        Err(RecvTimeoutError::Timeout) => None,
        // No more input (it was a pipe that closed): only the application can end the run now.
        Err(RecvTimeoutError::Disconnected) => {
            std::thread::sleep(wait);
            None
        }
    }
}

fn restarting(action: Action, location: Option<&str>) {
    let what = match action {
        Action::Reload => "Reloading",
        _ => "Restarting",
    };
    match location {
        Some(location) => report::say(&format!("{what} at {location}: building…")),
        None => report::say(&format!("{what}: building…")),
    }
}

// --- Desktop ---

fn run_desktop(project: &Project, options: &RunOptions) -> Result<ExitCode, String> {
    let link = DevLink::open()?;
    let runs = project.target_dir()?.join("frus-run");
    let mut generation = 0;
    let built = cargo::build_desktop(project, options.mode)?;
    let mut child = start_desktop(
        project,
        options,
        &built,
        &runs,
        &mut generation,
        &link,
        None,
    )?;
    let keys = attach(&format!("{} is running", project.package));
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            report::say(&format!("{} has stopped.", project.package));
            return Ok(exit_code(status.code()));
        }
        match next_action(&keys, Duration::from_millis(100)) {
            Some(Action::Quit) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(ExitCode::SUCCESS);
            }
            Some(Action::Help) => report::say(KEYS),
            Some(Action::Detach) => {
                report::say(&format!("Detached: {} goes on running.", project.package));
                return Ok(ExitCode::SUCCESS);
            }
            Some(Action::Clear) => clear_screen(),
            Some(Action::Missing(why)) => report::say(why),
            Some(Action::App(command)) => {
                if !link.send(&format!("do {command}")) {
                    report::warn("the application is not connected to frus run");
                }
            }
            Some(Action::Screenshot) => {
                let path = screenshot_path(&project.root);
                if link.send(&format!("do screenshot {}", path.display())) {
                    report_screenshot(&path, Duration::from_secs(10));
                } else {
                    report::warn("the application is not connected to frus run");
                }
            }
            Some(action @ (Action::Restart | Action::Reload)) => {
                restarting(action, link.report().location.as_deref());
                let started = Instant::now();
                // The running application stays up while the new one builds: it runs from a
                // copy, so the build may write its executable.
                match cargo::build_desktop(project, options.mode) {
                    Ok(built) => {
                        let states = (action == Action::Reload).then(|| runs.join("states.txt"));
                        let states = states.filter(|file| link.save_states(file));
                        let _ = child.kill();
                        let _ = child.wait();
                        link.disconnect();
                        child = start_desktop(
                            project,
                            options,
                            &built,
                            &runs,
                            &mut generation,
                            &link,
                            states.as_deref(),
                        )?;
                        restarted(action, started, "");
                    }
                    Err(error) => report::warn(&format!(
                        "{error}: the running application is left as it was."
                    )),
                }
            }
            None => {}
        }
    }
}

/// Starts a copy of `built`, told where to report and, after a restart, where to open.
///
/// A copy, because Windows does not let a running program's file be written: started from where
/// cargo built it, the application would make the next build fail.
fn start_desktop(
    project: &Project,
    options: &RunOptions,
    built: &Path,
    runs: &Path,
    generation: &mut u32,
    link: &DevLink,
    states: Option<&Path>,
) -> Result<Child, String> {
    std::fs::create_dir_all(runs).map_err(|e| e.to_string())?;
    // The previous copies, which are no longer running (the states file is the new one's).
    if let Ok(entries) = std::fs::read_dir(runs) {
        for entry in entries.filter_map(Result::ok) {
            if Some(entry.path().as_path()) != states {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    *generation += 1;
    let name = built
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| project.package.clone());
    let copy = runs.join(match built.extension() {
        Some(ext) => format!("{name}-{generation}.{}", ext.to_string_lossy()),
        None => format!("{name}-{generation}"),
    });
    std::fs::copy(built, &copy).map_err(|e| format!("could not copy the program to run: {e}"))?;

    let mut command = Command::new(&copy);
    command
        .args(&options.args)
        .current_dir(&project.root)
        .env("FRUS_DEV", link.address());
    let Report { location, window } = link.report();
    if let Some(location) = location {
        command.env("FRUS_RESTORE_LOCATION", location);
    }
    if let Some(window) = window {
        command.env("FRUS_RESTORE_WINDOW", window);
    }
    if let Some(states) = states {
        command.env("FRUS_RESTORE_STATES", states);
    }
    if !report::is_machine() {
        return command
            .spawn()
            .map_err(|e| format!("could not start {}: {e}", copy.display()));
    }
    // For an editor, the application's output is carried in `app.log` events: it must not
    // reach standard output, which holds JSON only.
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", copy.display()))?;
    if let Some(out) = child.stdout.take() {
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                report::app_log(&line, false);
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        std::thread::spawn(move || {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                report::app_log(&line, is_error_line(&line));
            }
        });
    }
    Ok(child)
}

/// Whether a line of a desktop program's standard error is an error. The log goes there whole,
/// information included, so its level says — and a panic.
pub(crate) fn is_error_line(line: &str) -> bool {
    line.contains(" ERROR ")
        || line.contains(" WARN ")
        || line.starts_with("thread '") && line.contains("panicked")
        || line.starts_with("error")
}

// --- Android ---

fn run_android(
    project: &Project,
    device: &Device,
    serial: &str,
    mode: Mode,
) -> Result<ExitCode, String> {
    let adb = devices::adb().ok_or("adb was not found: install Android's platform-tools")?;
    let adb_on = |args: &[&str]| {
        let mut command = Command::new(&adb);
        command.arg("-s").arg(serial).args(args);
        command
    };
    // The application reports where it is through its log (see `devlink`): no socket, so it
    // needs no network permission to be restarted.
    let link = DevLink::open()?;
    let package = &project.android_package;

    // What was installed last: a build that wrote the same package again — nothing in the code
    // changed — is not sent over the cable again, which is most of a restart's time.
    let installed = std::cell::Cell::new(None::<u64>);
    let install = |location: Option<&str>| -> Result<(), String> {
        let apk = cargo::build_apk(project, mode)?;
        let digest = std::fs::read(&apk).ok().map(|bytes| digest(&bytes));
        if digest.is_some() && digest == installed.get() {
            report::say("The package has not changed: starting it again.");
        } else {
            report::say(&format!(
                "Installing {} on {}…",
                file_name(&apk),
                device.name
            ));
            cargo::run(adb_on(&["install", "-r"]).arg(&apk).stdout(Stdio::null()))?;
            installed.set(digest);
        }
        let _ = adb_on(&["shell", "am", "force-stop", package]).output();
        let activity = format!("{package}/android.app.NativeActivity");
        let mut start = adb_on(&[
            "shell", "am", "start", "-n", &activity, "--es", "frus_dev", "log",
        ]);
        // After a restart, where the application was, as a link it reads like any other. Quoted
        // for the device's shell, which `adb shell` hands its arguments to joined by spaces —
        // a location's `?` and `&` would mean something there.
        if let Some(location) = location {
            start.args([
                "-d",
                &shell_quote(&format!("frus:///{}", location.trim_start_matches('/'))),
            ]);
        }
        cargo::run(start.stdout(Stdio::null()))
    };

    // The log from this moment on, read before the application starts so its first lines are
    // there: its own tag (the library's name), and Rust's panics. It goes on across restarts.
    let since = adb_on(&["shell", "date +'%m-%d %H:%M:%S.000'"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|date| !date.is_empty());
    let mut logcat = adb_on(&["logcat", "-v", "brief"]);
    if let Some(since) = &since {
        // `adb logcat` passes its arguments through as they are, unlike `adb shell`.
        logcat.args(["-T", since]);
    }
    let mut logcat = logcat
        .args([
            &format!("{}:V", project.lib),
            "RustPanic:V",
            "RustStdoutStderr:V",
            "*:S",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not read the device's log: {e}"))?;
    if let Some(stdout) = logcat.stdout.take() {
        let report = link.shared();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if !DevLink::from_log(&report, &line) {
                    print_logcat(&line);
                }
            }
        });
    }

    install(None)?;
    let keys = attach(&format!(
        "{} is running on {}",
        project.package, device.name
    ));

    // Until the application's process is gone — closed, or crashed — or `q`.
    let alive = || {
        adb_on(&["shell", "pidof", package])
            .output()
            .is_ok_and(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
    };
    let mut watch = Watch::default();
    let mut next_check = Instant::now();
    let end = loop {
        if Instant::now() >= next_check {
            next_check = Instant::now() + Duration::from_secs(1);
            if watch.gone(alive()) {
                report::say(&format!("{} has stopped.", project.package));
                break ExitCode::SUCCESS;
            }
        }
        match next_action(&keys, Duration::from_millis(200)) {
            Some(Action::Quit) => {
                let _ = adb_on(&["shell", "am", "force-stop", package]).output();
                break ExitCode::SUCCESS;
            }
            Some(Action::Help) => report::say(KEYS),
            Some(Action::Detach) => {
                report::say(&format!("Detached: {} goes on running.", project.package));
                break ExitCode::SUCCESS;
            }
            Some(Action::Clear) => clear_screen(),
            Some(Action::Missing(why)) => report::say(why),
            // Nothing reaches a running application from outside but its own storage, which a
            // debuggable package lets `run-as` write.
            Some(Action::App(command)) => {
                let write = format!(
                    "run-as {package} sh -c {}",
                    shell_quote(&format!(
                        "echo {} >> files/frus-run-commands",
                        shell_quote(command)
                    ))
                );
                let done = adb_on(&["shell", &write]).output();
                let refused = done.as_ref().map_or(true, |out| {
                    !out.status.success() || !String::from_utf8_lossy(&out.stderr).trim().is_empty()
                });
                if refused {
                    report::warn(
                        "the application could not be told: only a debug build can be (run \
                         without --release)",
                    );
                }
            }
            Some(Action::Screenshot) => {
                let path = screenshot_path(&project.root);
                match adb_on(&["exec-out", "screencap", "-p"]).output() {
                    Ok(out) if out.status.success() && out.stdout.starts_with(b"\x89PNG") => {
                        match std::fs::write(&path, &out.stdout) {
                            Ok(()) => report_screenshot(&path, Duration::from_secs(1)),
                            Err(error) => report::warn(&format!("{}: {error}", path.display())),
                        }
                    }
                    _ => report::warn("the device took no screenshot"),
                }
            }
            Some(action @ (Action::Restart | Action::Reload)) => {
                restarting(action, link.report().location.as_deref());
                let started = Instant::now();
                // On a reload, the application is sent to the background first: going there
                // under `frus run`, it keeps its states in its own storage, which the
                // reinstall leaves alone, and the next launch reads them back.
                if action == Action::Reload {
                    let _ = adb_on(&["shell", "input", "keyevent", "KEYCODE_HOME"]).output();
                    std::thread::sleep(Duration::from_millis(1500));
                } else {
                    let _ = adb_on(&[
                        "shell",
                        "run-as",
                        package,
                        "rm",
                        "-f",
                        "files/frus-run-states",
                    ])
                    .output();
                }
                match install(link.report().location.as_deref()) {
                    Ok(()) => {
                        watch = Watch::default();
                        restarted(action, started, "");
                    }
                    Err(error) => report::warn(&format!(
                        "{error}: the running application is left as it was."
                    )),
                }
            }
            None => {}
        }
    };
    let _ = logcat.kill();
    Ok(end)
}

/// Whether an application on a device has gone, from one look a second: it has to have been
/// seen first (or be missing for ten seconds), then be missing twice in a row.
#[derive(Debug, Default)]
pub(crate) struct Watch {
    seen: bool,
    missing: u32,
}

impl Watch {
    pub(crate) fn gone(&mut self, alive: bool) -> bool {
        if alive {
            self.seen = true;
            self.missing = 0;
            return false;
        }
        self.missing += 1;
        if self.seen {
            self.missing >= 2
        } else {
            self.missing > 10
        }
    }
}

/// `text` as one word for a POSIX shell.
pub(crate) fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Starts the emulator `avd`, waits until Android has booted in it, and gives its adb serial.
///
/// The emulator is left running when the run ends, as it would be after starting it by hand:
/// booting one takes a minute, and the next run should not pay for it again.
fn start_emulator(avd: &str) -> Result<String, String> {
    let adb = devices::adb().ok_or("adb was not found: install Android's platform-tools")?;
    let emulator = devices::emulator().ok_or("the Android emulator was not found in the SDK")?;
    let serials = |adb: &Path| -> Vec<String> {
        Command::new(adb)
            .args(["devices"])
            .output()
            .map(|o| {
                devices::parse_adb_devices(&String::from_utf8_lossy(&o.stdout))
                    .into_iter()
                    .map(|d| d.id)
                    .filter(|id| id.starts_with("emulator-"))
                    .collect()
            })
            .unwrap_or_default()
    };
    let before = serials(&adb);
    report::say(&format!("Starting the {avd} emulator…"));
    Command::new(&emulator)
        .args(["-avd", avd])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the emulator: {e}"))?;

    let started = Instant::now();
    let mut serial = None;
    while started.elapsed() < Duration::from_secs(300) {
        std::thread::sleep(Duration::from_secs(2));
        if serial.is_none() {
            serial = serials(&adb).into_iter().find(|s| !before.contains(s));
        }
        if let Some(serial) = &serial {
            let booted = Command::new(&adb)
                .args(["-s", serial, "shell", "getprop", "sys.boot_completed"])
                .output()
                .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1");
            if booted {
                return Ok(serial.clone());
            }
        }
    }
    Err(format!(
        "the {avd} emulator did not finish starting within five minutes"
    ))
}

/// A logcat line, `I/tag( 1234): text`, as `I/tag: text` — errors and warnings on stderr.
pub(crate) fn logcat_line(line: &str) -> Option<(bool, String)> {
    let (head, text) = line.split_once("): ")?;
    let tag = head.split('(').next()?.trim_end();
    let urgent = tag.starts_with("E/") || tag.starts_with("W/") || tag.starts_with("F/");
    Some((urgent, format!("{tag}: {text}")))
}

fn print_logcat(line: &str) {
    // Lines that are not the application's (`--------- beginning of main`) are left out.
    if let Some((urgent, text)) = logcat_line(line) {
        report::app_log(&text, urgent);
    }
}

// --- Web ---

fn run_web(
    project: &Project,
    browser: Option<&Path>,
    options: &RunOptions,
) -> Result<ExitCode, String> {
    let site = cargo::build_web(project, options.mode)?;
    let server = serve::start(site, options.port)?;
    let url = server.url.clone();

    // A profile of its own: the browser is a fresh instance this run owns and can close, not a
    // tab in the developer's own windows.
    let profile = std::env::temp_dir().join(format!("frus-run-{}", std::process::id()));
    let mut browser_process = match browser {
        Some(browser) => Some(
            Command::new(browser)
                .arg(format!("--user-data-dir={}", profile.display()))
                .args([
                    "--no-first-run",
                    "--no-default-browser-check",
                    "--new-window",
                ])
                .arg(&url)
                // The browser's own diagnostics are not the application's log.
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("could not open {}: {e}", browser.display()))?,
        ),
        None => None,
    };
    let keys = attach(&match browser_process {
        Some(_) => {
            format!("{url} is open; its console shows here, and closing the browser ends the run")
        }
        None => format!(
            "{} is served at {url}: open it in a browser with WebGPU; its console shows here",
            project.package
        ),
    });

    let end = loop {
        if let Some(process) = browser_process.as_mut() {
            if process.try_wait().map_err(|e| e.to_string())?.is_some() {
                break ExitCode::SUCCESS;
            }
        }
        match next_action(&keys, Duration::from_millis(200)) {
            Some(Action::Quit) => {
                if let Some(process) = browser_process.as_mut() {
                    let _ = process.kill();
                }
                break ExitCode::SUCCESS;
            }
            Some(Action::Help) => report::say(KEYS),
            Some(Action::Detach) => report::say(
                "The page is served by frus run: it cannot go on without it. `q` ends both.",
            ),
            Some(Action::Clear) => clear_screen(),
            Some(Action::Missing(why)) => report::say(why),
            // The page asks for them, as it asks which build is served.
            Some(Action::App(command)) => server.command(command),
            Some(Action::Screenshot) => report::say(
                "A page cannot hand its picture over yet: the browser's own tools take one \
                 (its developer tools, then Capture screenshot).",
            ),
            Some(action @ (Action::Restart | Action::Reload)) => {
                restarting(action, None);
                let started = Instant::now();
                match cargo::build_web(project, options.mode) {
                    // The page asks which build is served and reloads itself; its address,
                    // the application's location, survives the reload.
                    Ok(_) => {
                        server.rebuilt(action == Action::Restart);
                        restarted(action, started, ": the page reloads");
                    }
                    Err(error) => report::warn(&format!(
                        "{error}: the running application is left as it was."
                    )),
                }
            }
            None => {}
        }
    };
    let _ = std::fs::remove_dir_all(&profile);
    Ok(end)
}

/// A digest of a package's bytes, to tell whether a build wrote the same one again.
fn digest(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn exit_code(code: Option<i32>) -> ExitCode {
    match code {
        Some(0) => ExitCode::SUCCESS,
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::FAILURE,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logcat_lines_lose_the_pid() {
        assert_eq!(
            logcat_line("I/hello_world( 4321): signed in as ada"),
            Some((false, "I/hello_world: signed in as ada".into()))
        );
        assert_eq!(
            logcat_line("E/RustPanic(  77): panicked at src/lib.rs:3"),
            Some((true, "E/RustPanic: panicked at src/lib.rs:3".into()))
        );
        assert_eq!(logcat_line("--------- beginning of main"), None);
    }

    #[test]
    fn errors_are_told_from_information() {
        assert!(!is_error_line("[2026-10-17T10:00:00Z INFO  app] started"));
        assert!(is_error_line("[2026-10-17T10:00:00Z WARN  app] slow"));
        assert!(is_error_line("[2026-10-17T10:00:00Z ERROR app] failed"));
        assert!(is_error_line("thread 'main' panicked at src/lib.rs:3:5:"));
    }

    #[test]
    fn keys_mean_what_they_say() {
        assert_eq!(action('R'), Some(Action::Restart));
        assert_eq!(action('r'), Some(Action::Reload));
        assert_eq!(action('q'), Some(Action::Quit));
        assert_eq!(action('h'), Some(Action::Help));
        assert_eq!(action('x'), None);
        assert_eq!(action('d'), Some(Action::Detach));
        assert_eq!(action('c'), Some(Action::Clear));
        assert_eq!(action('s'), Some(Action::Screenshot));
        assert_eq!(action('p'), Some(Action::App("outlines")));
        assert_eq!(action('P'), Some(Action::App("performance")));
        assert!(matches!(action('v'), Some(Action::Missing(_))));
    }

    /// Every key the help lists does something, and every key that does something is listed
    /// (the reference's keys frus has nothing for say why instead, and are not).
    #[test]
    fn the_help_lists_the_keys() {
        let listed: Vec<char> = KEYS
            .lines()
            .filter_map(|line| line.trim_start().chars().next())
            .collect();
        for key in &listed {
            assert!(action(*key).is_some(), "{key} is listed and does nothing");
        }
        for key in ('!'..='~').filter(|k| !matches!(k, 'Q' | 'H' | '?')) {
            match action(key) {
                None | Some(Action::Missing(_)) => {}
                Some(_) => assert!(listed.contains(&key), "{key} is not listed"),
            }
        }
    }

    #[test]
    fn screenshots_take_the_next_free_number() {
        let dir = std::env::temp_dir().join(format!("frus-shots-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(screenshot_path(&dir), dir.join("frus_01.png"));
        std::fs::write(dir.join("frus_01.png"), b"x").unwrap();
        assert_eq!(screenshot_path(&dir), dir.join("frus_02.png"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn locations_are_quoted_for_the_device_shell() {
        assert_eq!(
            shell_quote("frus:///search?q=shoes&sort=price"),
            "'frus:///search?q=shoes&sort=price'"
        );
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn an_application_is_gone_after_two_misses() {
        let mut watch = Watch::default();
        // Not up yet: patience, for a while.
        assert!(!watch.gone(false));
        assert!(!watch.gone(true));
        assert!(!watch.gone(false));
        assert!(watch.gone(false));
        // Never seen: given up after ten seconds.
        let mut never = Watch::default();
        assert!((0..10).all(|_| !never.gone(false)));
        assert!(never.gone(false));
    }
}
