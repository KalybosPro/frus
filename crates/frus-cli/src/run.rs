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
    /// `--hot`: true hot reload, the code patched into the running application (desktop, web).
    pub hot: bool,
    /// The device profile the desktop window stands in for, when `-d` named a simulated device
    /// (milestone 723). Not parsed: `run` sets it once the device is known.
    pub simulate: Option<String>,
    /// `--web`: a simulated device in a browser instead of a desktop window (milestone 724).
    pub web: bool,
}

pub(crate) const DEFAULT_PORT: u16 = 8080;

/// What tells the shell which device the window stands in for (`frus-shell`'s `simulated`).
const SIMULATE_ENV: &str = "FRUS_DEVICE";

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
        'T' => Action::App("text-scale"),
        'l' => Action::App("rotate"),
        'K' => Action::App("keyboard"),
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
  T  text size: 100%, 130%, 200%, 85%, as a reader's settings would
  l  turn a simulated device on its side and back (frus run -d iphone-15)
  K  hold a simulated device's keyboard up, or let it go
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
    let hot_here = matches!(
        device.kind,
        Kind::Host
            | Kind::Browser { .. }
            | Kind::WebServer
            | Kind::Android { .. }
            | Kind::Emulator { .. }
            | Kind::Simulated { .. }
    );
    if options.hot && !hot_here {
        return Err(format!(
            "--hot patches the code of a running application on the desktop and the web only, \
             for now; on {} use `r`, which reloads with the state kept",
            device.name
        ));
    }
    if options.web && !matches!(device.kind, Kind::Simulated { .. }) {
        return Err(format!(
            "--web shows a simulated device in a browser (`frus run -d iphone-15 --web`); \
             {} is not one: to run in a browser, `frus run -d web`",
            device.name
        ));
    }
    let result = match &device.kind {
        Kind::Host if options.hot => run_hot(&project, options),
        Kind::Host => run_desktop(&project, options),
        // A simulation is a debug build's: a released application never stands in for another
        // device, and the shell ignores `FRUS_DEVICE` there.
        Kind::Simulated { .. } if options.mode == Mode::Release => Err(format!(
            "{} is simulated by a debug build: run it without --release",
            device.name
        )),
        Kind::Simulated { profile } => {
            let options = RunOptions {
                simulate: Some(profile.clone()),
                ..options.clone()
            };
            // In a browser: the first one found, or served for any browser if there is none.
            let browser = devices.iter().find_map(|d| match &d.kind {
                Kind::Browser { exe } => Some(exe.as_path()),
                _ => None,
            });
            match (options.web, options.hot) {
                (true, true) => run_web_hot(&project, browser, &options),
                (true, false) => run_web(&project, browser, &options),
                (false, true) => run_hot(&project, &options),
                (false, false) => run_desktop(&project, &options),
            }
        }
        Kind::Android { serial } if options.hot => {
            run_android_hot(&project, &device, serial, options)
        }
        Kind::Android { serial } => run_android(&project, &device, serial, options.mode),
        Kind::Emulator { avd } => {
            let serial = start_emulator(avd)?;
            if options.hot {
                run_android_hot(&project, &device, &serial, options)
            } else {
                run_android(&project, &device, &serial, options.mode)
            }
        }
        Kind::IosSimulator { .. } => Err(format!(
            "{} is an iOS simulator, and frus cannot run on iOS yet: the iOS shell does not \
             exist (ROADMAP, issue #19). `frus run` will start iOS applications once it does.",
            device.name
        )),
        Kind::Browser { exe } if options.hot => run_web_hot(&project, Some(exe), options),
        Kind::Browser { exe } => run_web(&project, Some(exe), options),
        Kind::WebServer if options.hot => run_web_hot(&project, None, options),
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
    // The previous copies, which are no longer running (the states file is the new one's;
    // DXC's DLLs stay, for the next copy).
    if let Ok(entries) = std::fs::read_dir(runs) {
        for entry in entries.filter_map(Result::ok) {
            let dll = entry.path().extension().is_some_and(|e| e == "dll");
            if Some(entry.path().as_path()) != states && !dll {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    // DirectX 12 compiles the program's shaders with DXC when it is beside the program: a
    // start about half a second shorter (milestone 684).
    if cfg!(windows) {
        crate::setup::place_dxc(runs);
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
    if let Some(profile) = &options.simulate {
        command.env(SIMULATE_ENV, profile);
    }
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

// --- Desktop, hot ---

/// `frus run --hot`: `dx serve --hot-patch` on the shadow package (see `hot`), its output read
/// and turned into the run's own — the application's log, and a line for each patch.
fn run_hot(project: &Project, options: &RunOptions) -> Result<ExitCode, String> {
    let engine = crate::hot::engine()?;
    let shadow = crate::hot::shadow(project)?;
    let mirror = mirror_sources(project, &shadow)?;
    let link = DevLink::open()?;
    // A build of its own: the program a patch replaces is linked with everything kept.
    let target = project.target_dir()?.join("frus-hot").join("target");
    let (lines_in, lines) = std::sync::mpsc::channel::<crate::hot::DxLine>();
    let start = |link: &DevLink| -> Result<Child, String> {
        crate::hot::clear_dependency_archives(&target);
        let mut command = Command::new(&engine);
        command
            .args([
                "serve",
                crate::hot::platform_flag(),
                "--hot-patch",
                "--bin",
                &project.lib,
                "--features",
                "frus/hot-reload",
                "--interactive",
                "false",
                "--open",
                "true",
            ])
            .args(options.mode.eq(&Mode::Release).then_some("--release"))
            .current_dir(&shadow)
            .env("CARGO_TARGET_DIR", &target)
            // The application dx starts inherits it, and reports to this run as under `r`.
            .env("FRUS_DEV", link.address());
        // And the device it stands in for, the same way.
        if let Some(profile) = &options.simulate {
            command.env(SIMULATE_ENV, profile);
        }
        spawn_engine(&mut command, &lines_in)
    };

    report::say("Building for hot reload (the first build takes longer)…");
    let mut dx = start(&link)?;
    // When the application came up, and whether the edits made just after have been handed on
    // again (see `Mirror::retouch_since`).
    let mut launched_at: Option<std::time::SystemTime> = None;
    let mut retouched = false;
    let mut keys: Option<Receiver<char>> = None;
    let mut restarting: Option<Instant> = None;
    loop {
        while let Ok(line) = lines.try_recv() {
            use crate::hot::DxLine;
            match line {
                DxLine::App(text, error) => report::app_log(&text, error),
                DxLine::Launched => {
                    launched_at.get_or_insert_with(std::time::SystemTime::now);
                    on_launch(&mut keys, project, &mut restarting);
                }
                DxLine::Patched { file, millis } => {
                    let seconds = millis as f64 / 1000.0;
                    if report::is_machine() {
                        report::event(
                            "app.restarted",
                            object([("fullRestart", false.into()), ("seconds", seconds.into())]),
                        );
                    } else {
                        report::say(&format!("Patched {file} in {seconds:.1} s."));
                    }
                }
                DxLine::Failed(why) => {
                    report::warn(&why);
                    // The engine says what is wrong but not where: the compiler, asked again, does.
                    locate_errors(project, &shadow, &target);
                }
                DxLine::Exited if restarting.is_none() => {
                    report::say(&format!("{} has stopped.", project.package));
                    crate::hot::stop(&mut dx);
                    return Ok(ExitCode::SUCCESS);
                }
                DxLine::Exited | DxLine::Noise => {}
            }
        }
        if let Some(at) = launched_at.filter(|_| !retouched) {
            if at.elapsed().unwrap_or_default() > Duration::from_secs(8) {
                retouched = true;
                if let Ok(mirror) = mirror.lock() {
                    let _ = mirror.retouch_since(at);
                }
            }
        }
        if let Some(status) = dx.try_wait().map_err(|e| e.to_string())? {
            if restarting.is_none() {
                return Err(format!("the hot-reload engine stopped ({status})"));
            }
        }
        let Some(keys) = keys.as_ref() else {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        };
        match next_action(keys, Duration::from_millis(100)) {
            Some(Action::Quit) => {
                crate::hot::stop(&mut dx);
                return Ok(ExitCode::SUCCESS);
            }
            Some(Action::Help) => report::say(KEYS),
            Some(Action::Reload) => report::say(
                "With --hot, saving a file patches it in: there is nothing to press. `R` starts \
                 the application again, every state afresh.",
            ),
            Some(Action::Restart) => {
                restarting = Some(Instant::now());
                report::say("Restarting: building…");
                crate::hot::stop(&mut dx);
                link.disconnect();
                dx = start(&link)?;
            }
            Some(Action::Detach) => {
                // dx alone: the application it started goes on.
                let _ = dx.kill();
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
            None => {}
        }
    }
}

/// The application's sources, copied into the shadow package and kept in step while the run
/// lasts (see `hot::Mirror`).
fn mirror_sources(
    project: &Project,
    shadow: &Path,
) -> Result<std::sync::Arc<std::sync::Mutex<crate::hot::Mirror>>, String> {
    let mut mirror = crate::hot::Mirror::new(project.root.join("src"), shadow.join("src"));
    mirror
        .sync()
        .map_err(|e| format!("could not copy the sources: {e}"))?;
    let mirror = std::sync::Arc::new(std::sync::Mutex::new(mirror));
    {
        let mirror = mirror.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));
            if let Ok(mut mirror) = mirror.lock() {
                if let Err(error) = mirror.sync() {
                    report::warn(&format!("could not copy a changed source: {error}"));
                }
            }
        });
    }
    Ok(mirror)
}

/// Starts the engine, each line it prints read into `lines`.
fn spawn_engine(
    command: &mut Command,
    lines: &std::sync::mpsc::Sender<crate::hot::DxLine>,
) -> Result<Child, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A process group of its own, so that stopping it stops the application it started too
    // (see `hot::stop`).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut dx = command
        .spawn()
        .map_err(|e| format!("could not start the hot-reload engine: {e}"))?;
    for stream in [
        dx.stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        dx.stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let send = lines.clone();
        // `FRUS_HOT_TRACE=1`: every line the engine prints, as it prints it, on standard error.
        let trace = std::env::var_os("FRUS_HOT_TRACE").is_some();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if trace {
                    eprintln!("[dx] {line}");
                }
                if send.send(crate::hot::dx_line(&line)).is_err() {
                    break;
                }
            }
        });
    }
    Ok(dx)
}

/// The application is up: after a restart, how long it took; the first time, the keys.
fn on_launch(
    keys: &mut Option<Receiver<char>>,
    project: &Project,
    restarting: &mut Option<Instant>,
) {
    match restarting.take() {
        Some(started) => restarted(Action::Restart, started, ""),
        None if keys.is_none() => {
            *keys = Some(attach(&format!(
                "{} is running with hot reload: save a file and the change is patched into it, \
                 its state kept",
                project.package
            )));
        }
        None => {}
    }
}

/// After a failed patch: `cargo check` on the shadow package, and each error it finds said with
/// its place in the application's own files — the engine names the error, not where it is.
fn locate_errors(project: &Project, shadow: &Path, target: &Path) {
    let output = Command::new("cargo")
        .args([
            "check",
            "--message-format=short",
            "--bin",
            &project.lib,
            "--features",
            "frus/hot-reload",
        ])
        .current_dir(shadow)
        .env("CARGO_TARGET_DIR", target)
        .stdin(Stdio::null())
        .output();
    let Ok(output) = output else {
        return;
    };
    let text = String::from_utf8_lossy(&output.stderr);
    let located: Vec<String> = text
        .lines()
        .filter_map(|line| crate::hot::located_error(line, shadow, &project.root))
        .collect();
    for line in &located {
        report::warn(line);
    }
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
    // The device's processor: the package is built for it (see `cargo::build_apk`).
    let device_target = adb_on(&["shell", "getprop", "ro.product.cpu.abi"])
        .output()
        .ok()
        .and_then(|o| android_rust_target(String::from_utf8_lossy(&o.stdout).trim()));

    // What was installed last: a build that wrote the same package again — nothing in the code
    // changed — is not sent over the cable again, which is most of a restart's time.
    let installed = std::cell::Cell::new(None::<u64>);
    let install = |location: Option<&str>| -> Result<(), String> {
        let apk = cargo::build_apk(project, mode, device_target.as_deref())?;
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

    let mut logcat = follow_android_log(&adb, serial, project, &link)?;

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
            Some(Action::App(command)) => send_android_command(&adb, serial, package, command),
            Some(Action::Screenshot) => android_screenshot(&adb, serial, project),
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

/// The Rust target for an Android ABI (`getprop ro.product.cpu.abi`).
pub(crate) fn android_rust_target(abi: &str) -> Option<String> {
    let target = match abi {
        "arm64-v8a" => "aarch64-linux-android",
        "armeabi-v7a" => "armv7-linux-androideabi",
        "x86_64" => "x86_64-linux-android",
        "x86" => "i686-linux-android",
        _ => return None,
    };
    Some(target.to_string())
}

/// The device's log from this moment on, read before the application starts so its first lines
/// are there: its own tag (the library's name), and Rust's panics. It goes on across restarts.
fn follow_android_log(
    adb: &Path,
    serial: &str,
    project: &Project,
    link: &DevLink,
) -> Result<Child, String> {
    let since = Command::new(adb)
        .args(["-s", serial, "shell", "date +'%m-%d %H:%M:%S.000'"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|date| !date.is_empty());
    let mut logcat = Command::new(adb);
    logcat.args(["-s", serial, "logcat", "-v", "brief"]);
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
    Ok(logcat)
}

// --- Android, hot ---

/// `frus run --hot` on an Android device (milestone 680): the engine builds the application
/// into an Android project of its own, installs and starts it, and pushes a patch library to the
/// device for each saved change, which the running application loads, its state kept. The
/// application's log, the keys and screenshots go as under `frus run` without `--hot`.
fn run_android_hot(
    project: &Project,
    device: &Device,
    serial: &str,
    options: &RunOptions,
) -> Result<ExitCode, String> {
    let adb = devices::adb().ok_or("adb was not found: install Android's platform-tools")?;
    let adb_on = |args: &[&str]| {
        let mut command = Command::new(&adb);
        command.arg("-s").arg(serial).args(args);
        command
    };
    let package = &project.android_package;
    // What the engine's build needs and the machine may lack: the Rust targets, the SDK and the
    // NDK, and a JDK for its Gradle project.
    for target in &project.android_targets {
        crate::setup::rust_target(target)?;
    }
    let sdk = crate::setup::android_sdk(project.android_target_sdk)?;
    let java = crate::setup::java()?;
    let engine = crate::hot::engine()?;
    let shadow = crate::hot::shadow(project)?;
    crate::hot::android_project_files(&shadow, package, &project.root.join("res"))?;
    let mirror = mirror_sources(project, &shadow)?;
    let link = DevLink::open()?;
    let target = project.target_dir()?.join("frus-hot").join("target");
    // The engine's server, which the application reaches through `adb reverse` on this port.
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|e| format!("could not find a free port for the hot-reload engine: {e}"))?;
    let (lines_in, lines) = std::sync::mpsc::channel::<crate::hot::DxLine>();
    let port_text = port.to_string();
    let start = || -> Result<Child, String> {
        crate::hot::clear_dependency_archives(&target);
        let mut command = Command::new(&engine);
        command
            .args([
                "serve",
                "--android",
                "--hot-patch",
                "--bin",
                &project.lib,
                "--features",
                "frus/hot-reload",
                "--interactive",
                "false",
                "--open",
                "true",
                "--device",
                serial,
                "--port",
                &port_text,
            ])
            .args(options.mode.eq(&Mode::Release).then_some("--release"))
            .current_dir(&shadow)
            .env("CARGO_TARGET_DIR", &target)
            .env("ANDROID_HOME", &sdk)
            .env("JAVA_HOME", &java);
        if let Some(ndk) = std::env::var_os("ANDROID_NDK_ROOT") {
            command.env("NDK_HOME", ndk);
        }
        spawn_engine(&mut command, &lines_in)
    };

    // The engine's package says version 1; Android refuses it over the one `frus run` installs
    // without `--hot`, whose version comes from the manifest's. That one goes first.
    let dumped = adb_on(&["shell", "dumpsys", "package", package])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    if installed_version_code(&dumped).is_some_and(|code| code > 1) {
        report::say(&format!(
            "Removing {package} from {}: the hot-reload build cannot replace the one installed \
             without --hot (its data on the device goes with it).",
            device.name
        ));
        let _ = adb_on(&["uninstall", package]).output();
    }
    report::say(&format!(
        "Building for hot reload on {} (the first build takes long: the whole application, and \
         an Android project)…",
        device.name
    ));
    let mut logcat = follow_android_log(&adb, serial, project, &link)?;
    let mut dx = start()?;
    let alive = || {
        adb_on(&["shell", "pidof", package])
            .output()
            .is_ok_and(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
    };
    let mut keys: Option<Receiver<char>> = None;
    let mut launched_at: Option<std::time::SystemTime> = None;
    let mut retouched = false;
    // After `R`: when the engine was started again, until the application runs again.
    let mut restarting: Option<Instant> = None;
    let mut watch = Watch::default();
    let mut next_check = Instant::now();
    let end =
        loop {
            while let Ok(line) = lines.try_recv() {
                use crate::hot::DxLine;
                match line {
                    // The engine installs and starts the package after this; the keys come once
                    // the application runs.
                    DxLine::Launched => {
                        launched_at.get_or_insert_with(std::time::SystemTime::now);
                    }
                    DxLine::Patched { file, millis } => {
                        let seconds = millis as f64 / 1000.0;
                        if report::is_machine() {
                            report::event(
                                "app.restarted",
                                object([
                                    ("fullRestart", false.into()),
                                    ("seconds", seconds.into()),
                                ]),
                            );
                        } else {
                            report::say(&format!("Patched {file} in {seconds:.1} s."));
                        }
                    }
                    DxLine::Failed(why) => {
                        report::warn(&why);
                        locate_errors(project, &shadow, &target);
                    }
                    // The application's log is read from the device (above), not from the engine.
                    DxLine::App(..) | DxLine::Exited | DxLine::Noise => {}
                }
            }
            if let Some(at) = launched_at.filter(|_| !retouched) {
                if at.elapsed().unwrap_or_default() > Duration::from_secs(8) {
                    retouched = true;
                    if let Ok(mirror) = mirror.lock() {
                        let _ = mirror.retouch_since(at);
                    }
                }
            }
            if let Some(status) = dx.try_wait().map_err(|e| e.to_string())? {
                report::warn(&format!("the hot-reload engine stopped ({status})"));
                break ExitCode::FAILURE;
            }
            if launched_at.is_some() && Instant::now() >= next_check {
                next_check = Instant::now() + Duration::from_secs(1);
                let running = alive();
                if running && keys.is_none() {
                    keys = Some(attach(&format!(
                        "{} is running on {} with hot reload: save a file and the change is \
                     patched into it, its state kept",
                        project.package, device.name
                    )));
                }
                if running {
                    if let Some(started) = restarting.take() {
                        restarted(Action::Restart, started, "");
                    }
                }
                // While the engine installs the new build, the application is not there.
                if keys.is_some() && restarting.is_none() && watch.gone(running) {
                    report::say(&format!("{} has stopped.", project.package));
                    break ExitCode::SUCCESS;
                }
            }
            let Some(keys) = keys.as_ref() else {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };
            match next_action(keys, Duration::from_millis(100)) {
            Some(Action::Quit) => {
                let _ = adb_on(&["shell", "am", "force-stop", package]).output();
                break ExitCode::SUCCESS;
            }
            Some(Action::Help) => report::say(KEYS),
            Some(Action::Reload) => report::say(
                "With --hot, saving a file patches it in: there is nothing to press. `R` starts \
                 the application again, every state afresh.",
            ),
            // A patch is linked against the process the engine started: a process started
            // again by hand cannot take one (it crashed). So `R` starts the engine again, which
            // builds what is saved, installs it and starts it, as on the desktop.
            Some(Action::Restart) => {
                report::say("Restarting: building again…");
                crate::hot::stop(&mut dx);
                let _ = adb_on(&["shell", "run-as", package, "rm", "-f", "files/frus-run-states"])
                    .output();
                let _ = adb_on(&["shell", "am", "force-stop", package]).output();
                launched_at = None;
                retouched = false;
                watch = Watch::default();
                restarting = Some(Instant::now());
                dx = start()?;
            }
            Some(Action::Detach) => {
                // The engine alone: the application goes on, without patches.
                let _ = dx.kill();
                report::say(&format!("Detached: {} goes on running.", project.package));
                break ExitCode::SUCCESS;
            }
            Some(Action::Clear) => clear_screen(),
            Some(Action::Missing(why)) => report::say(why),
            Some(Action::App(command)) => send_android_command(&adb, serial, package, command),
            Some(Action::Screenshot) => android_screenshot(&adb, serial, project),
            None => {}
        }
        };
    crate::hot::stop(&mut dx);
    let _ = logcat.kill();
    Ok(end)
}

/// The version code of the package `dumpsys package` describes, if one is installed.
pub(crate) fn installed_version_code(dumpsys: &str) -> Option<u64> {
    dumpsys
        .split_whitespace()
        .find_map(|word| word.strip_prefix("versionCode="))
        .and_then(|code| code.parse().ok())
}

/// A key's command for an Android application under `frus run`: nothing reaches a running
/// application from outside but its own storage, which a debuggable package lets `run-as` write.
fn send_android_command(adb: &Path, serial: &str, package: &str, command: &str) {
    let write = format!(
        "run-as {package} sh -c {}",
        shell_quote(&format!(
            "echo {} >> files/frus-run-commands",
            shell_quote(command)
        ))
    );
    let done = Command::new(adb)
        .args(["-s", serial, "shell", &write])
        .output();
    let refused = done.as_ref().map_or(true, |out| {
        !out.status.success() || !String::from_utf8_lossy(&out.stderr).trim().is_empty()
    });
    if refused {
        report::warn(
            "the application could not be told: only a debug build can be (run without \
             --release)",
        );
    }
}

/// The device's screen, saved beside the application (`s`).
fn android_screenshot(adb: &Path, serial: &str, project: &Project) {
    let path = screenshot_path(&project.root);
    match Command::new(adb)
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .output()
    {
        Ok(out) if out.status.success() && out.stdout.starts_with(b"\x89PNG") => {
            match std::fs::write(&path, &out.stdout) {
                Ok(()) => report_screenshot(&path, Duration::from_secs(1)),
                Err(error) => report::warn(&format!("{}: {error}", path.display())),
            }
        }
        _ => report::warn("the device took no screenshot"),
    }
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
    let server = serve::start_site(
        serve::Site {
            dir: site,
            device: options.simulate.clone(),
            ..serve::Site::default()
        },
        options.port,
    )?;
    let url = server.url.clone();
    let (mut browser_process, profile) = open_browser(browser, &url)?;
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
            // The application reads its canvas on its next frame and the page sends it here.
            Some(Action::Screenshot) => {
                let path = screenshot_path(&project.root);
                server.expect_screenshot(path.clone());
                server.command("screenshot page");
                report_screenshot(&path, Duration::from_secs(10));
            }
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

/// The browser opened on `url`, with a profile of its own: a fresh instance this run owns and
/// can close, not a tab in the developer's own windows. Without a browser (`web-server`),
/// nothing is opened.
fn open_browser(
    browser: Option<&Path>,
    url: &str,
) -> Result<(Option<Child>, std::path::PathBuf), String> {
    let profile = std::env::temp_dir().join(format!("frus-run-{}", std::process::id()));
    let process = match browser {
        Some(browser) => Some(
            Command::new(browser)
                .arg(format!("--user-data-dir={}", profile.display()))
                .args([
                    "--no-first-run",
                    "--no-default-browser-check",
                    "--new-window",
                ])
                .arg(url)
                // The browser's own diagnostics are not the application's log.
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("could not open {}: {e}", browser.display()))?,
        ),
        None => None,
    };
    Ok((process, profile))
}

// --- Web, hot ---

/// `frus run --hot -d web` (milestone 679): the engine builds the page's module and a patch for
/// each saved change; this run's own server serves the page with the engine's build in it and
/// hands each patch to the page, which links it into the running module, its state
/// kept. A page that loads or reloads is given the latest patch as it starts.
fn run_web_hot(
    project: &Project,
    browser: Option<&Path>,
    options: &RunOptions,
) -> Result<ExitCode, String> {
    let web = project.web_dir();
    if !web.join("index.html").is_file() {
        return Err(format!(
            "{} has no web/index.html: the page the application runs in (`frus create` writes one)",
            project.package
        ));
    }
    crate::setup::rust_target("wasm32-unknown-unknown")?;
    let engine = crate::hot::engine()?;
    let shadow = crate::hot::shadow(project)?;
    let mirror = mirror_sources(project, &shadow)?;
    let target = project.target_dir()?.join("frus-hot").join("target");
    // The engine's own server: only its socket is used, by this run.
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|e| format!("could not find a free port for the hot-reload engine: {e}"))?;
    let (lines_in, lines) = std::sync::mpsc::channel::<crate::hot::DxLine>();
    let port_text = port.to_string();
    let start = || -> Result<Child, String> {
        crate::hot::clear_dependency_archives(&target);
        let mut command = Command::new(&engine);
        command
            .args([
                "serve",
                "--web",
                "--hot-patch",
                "--bin",
                &project.lib,
                "--features",
                "frus/hot-reload",
                "--interactive",
                "false",
                "--open",
                "false",
                "--port",
                &port_text,
            ])
            .args(options.mode.eq(&Mode::Release).then_some("--release"))
            .current_dir(&shadow)
            .env("CARGO_TARGET_DIR", &target);
        spawn_engine(&mut command, &lines_in)
    };

    report::say("Building for hot reload on the web (the first build takes longer)…");
    let mut dx = start()?;
    // The first build: the page is served once there is a module to serve.
    loop {
        if let Some(status) = dx.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("the hot-reload engine stopped ({status})"));
        }
        match lines.recv_timeout(Duration::from_millis(200)) {
            Ok(crate::hot::DxLine::Launched) => break,
            Ok(crate::hot::DxLine::Failed(why)) => {
                report::warn(&format!("{why}: save a fix and it is built again"));
                locate_errors(project, &shadow, &target);
            }
            _ => {}
        }
    }
    let launched_at = std::time::SystemTime::now();
    let Some(built) = crate::hot::web_build_dir(&target, &project.lib) else {
        crate::hot::stop(&mut dx);
        return Err(format!(
            "the hot-reload engine built no {}.js under {}",
            project.lib,
            target.join("dx").display()
        ));
    };
    let server = serve::start_site(
        serve::Site {
            dir: web,
            // The page loads `pkg/<library>.js`; a patch is fetched from `/wasm/<patch>.wasm`.
            mounts: vec![("pkg".into(), built.clone()), ("wasm".into(), built)],
            hot: true,
            device: options.simulate.clone(),
        },
        options.port,
    )?;
    // The engine's socket, read by this run: each patch is kept for the pages, which ask for it
    // (see `serve::PatchSlot`). The engine sends a patch to each client connected when it is
    // made, and the latest one to a client that connects — but not reliably to every page that
    // connects later, as a reloaded one does.
    {
        let patches = server.patches.clone();
        let address = format!("ws://127.0.0.1:{port}/_dioxus?build_id=0");
        std::thread::spawn(move || loop {
            let _ = crate::socket::read_messages(&address, |message| {
                if message.starts_with("{\"HotReload\"") && message.contains("\"jump_table\":{") {
                    patches.push(message);
                }
            });
            std::thread::sleep(Duration::from_secs(1));
        });
    }
    let url = server.url.clone();
    let (mut browser_process, profile) = open_browser(browser, &url)?;
    let keys = attach(&match browser_process {
        Some(_) => format!(
            "{url} is open with hot reload: save a file and the change is patched into the page, \
             its state kept; its console shows here"
        ),
        None => format!(
            "{} is served at {url} with hot reload: open it in a browser with WebGPU; save a \
             file and the change is patched into the page",
            project.package
        ),
    });

    let mut retouched = false;
    let end = loop {
        while let Ok(line) = lines.try_recv() {
            use crate::hot::DxLine;
            match line {
                DxLine::Patched { file, millis } => {
                    let seconds = millis as f64 / 1000.0;
                    if report::is_machine() {
                        report::event(
                            "app.restarted",
                            object([("fullRestart", false.into()), ("seconds", seconds.into())]),
                        );
                    } else {
                        report::say(&format!("Patched {file} in {seconds:.1} s."));
                    }
                }
                DxLine::Failed(why) => {
                    report::warn(&why);
                    locate_errors(project, &shadow, &target);
                }
                // A change no patch can carry was built in full: the page reloads onto it, its
                // states kept.
                DxLine::Launched => {
                    server.rebuilt(false);
                    report::say("Rebuilt in full: the page reloads, its state kept.");
                }
                DxLine::App(text, true) => report::warn(&text),
                DxLine::App(..) | DxLine::Exited | DxLine::Noise => {}
            }
        }
        // The engine watches the files a few seconds after it starts: edits made before are
        // handed on again (see `Mirror::retouch_since`).
        if !retouched && launched_at.elapsed().unwrap_or_default() > Duration::from_secs(8) {
            retouched = true;
            if let Ok(mirror) = mirror.lock() {
                let _ = mirror.retouch_since(launched_at);
            }
        }
        if let Some(status) = dx.try_wait().map_err(|e| e.to_string())? {
            report::warn(&format!("the hot-reload engine stopped ({status})"));
            break ExitCode::FAILURE;
        }
        if let Some(process) = browser_process.as_mut() {
            if process.try_wait().map_err(|e| e.to_string())?.is_some() {
                break ExitCode::SUCCESS;
            }
        }
        match next_action(&keys, Duration::from_millis(100)) {
            Some(Action::Quit) => {
                if let Some(process) = browser_process.as_mut() {
                    let _ = process.kill();
                }
                break ExitCode::SUCCESS;
            }
            Some(Action::Help) => report::say(KEYS),
            Some(Action::Reload) => report::say(
                "With --hot, saving a file patches it in: there is nothing to press. `R` starts \
                 the application again, every state afresh.",
            ),
            // The page reloads afresh; the engine hands it the latest patch as it connects.
            Some(Action::Restart) => {
                server.rebuilt(true);
                report::say("Restarting: the page reloads, every state afresh.");
            }
            Some(Action::Detach) => report::say(
                "The page is served by frus run: it cannot go on without it. `q` ends both.",
            ),
            Some(Action::Clear) => clear_screen(),
            Some(Action::Missing(why)) => report::say(why),
            Some(Action::App(command)) => server.command(command),
            Some(Action::Screenshot) => {
                let path = screenshot_path(&project.root);
                server.expect_screenshot(path.clone());
                server.command("screenshot page");
                report_screenshot(&path, Duration::from_secs(10));
            }
            None => {}
        }
    };
    crate::hot::stop(&mut dx);
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
    fn a_device_is_built_for_its_own_processor() {
        assert_eq!(
            android_rust_target("x86_64").as_deref(),
            Some("x86_64-linux-android")
        );
        assert_eq!(
            android_rust_target("arm64-v8a").as_deref(),
            Some("aarch64-linux-android")
        );
        assert_eq!(android_rust_target("mips"), None);
    }

    #[test]
    fn the_installed_version_code_is_read() {
        let dumped =
            "Packages:\n  Package [com.example.counter] (8e1c):\n    versionCode=16777472 \
                      minSdk=24 targetSdk=34\n    versionName=0.1.0\n";
        assert_eq!(installed_version_code(dumped), Some(16777472));
        assert_eq!(installed_version_code("Unable to find package: x"), None);
    }

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
        assert_eq!(action('T'), Some(Action::App("text-scale")));
        assert_eq!(action('l'), Some(Action::App("rotate")));
        assert_eq!(action('K'), Some(Action::App("keyboard")));
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
