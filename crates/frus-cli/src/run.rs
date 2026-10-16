//! `frus run`: build the application for a device, start it there, and show what it logs —
//! the desktop's terminal output, Android's logcat, the browser's console — in this terminal
//! until it ends.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use crate::cargo::{self, Mode};
use crate::devices::{self, Device, Kind};
use crate::project::Project;
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
}

pub(crate) const DEFAULT_PORT: u16 = 8080;

pub(crate) fn run(options: &RunOptions) -> Result<ExitCode, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = Project::find(&here)?;
    let devices = devices::discover();
    let device = devices::select(&devices, options.device.as_deref())?;
    if options.device.is_none() && devices.iter().any(|d| d.platform != device.platform) {
        println!(
            "Running on {}. To run elsewhere: `frus run -d <device>` (`frus devices` lists them).",
            device.name
        );
    }
    match &device.kind {
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
    }
}

fn run_desktop(project: &Project, options: &RunOptions) -> Result<ExitCode, String> {
    let exe = cargo::build_desktop(project, options.mode)?;
    println!("Running {}…", exe.display());
    let status = Command::new(&exe)
        .args(&options.args)
        .current_dir(&project.root)
        .status()
        .map_err(|e| format!("could not start {}: {e}", exe.display()))?;
    Ok(exit_code(status.code()))
}

fn run_android(
    project: &Project,
    device: &Device,
    serial: &str,
    mode: Mode,
) -> Result<ExitCode, String> {
    let adb = devices::adb().ok_or("adb was not found: install Android's platform-tools")?;
    let apk = cargo::build_apk(project, mode)?;
    let adb_on = |args: &[&str]| {
        let mut command = Command::new(&adb);
        command.arg("-s").arg(serial).args(args);
        command
    };

    println!("Installing {} on {}…", file_name(&apk), device.name);
    cargo::run(adb_on(&["install", "-r"]).arg(&apk))?;
    let package = &project.android_package;
    let _ = adb_on(&["shell", "am", "force-stop", package]).output();

    // The log from this moment on, read before the application starts so its first lines are
    // there: its own tag (the library's name), and Rust's panics.
    // adb hands its arguments to the device's shell joined by spaces, so a value holding a
    // space is quoted for that shell.
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
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                print_logcat(&line);
            }
        });
    }

    cargo::run(&mut adb_on(&[
        "shell",
        "am",
        "start",
        "-n",
        &format!("{package}/android.app.NativeActivity"),
    ]))?;
    println!(
        "{} is running on {}. Its log follows; Ctrl+C stops reading it.",
        project.package, device.name
    );

    // Until the application's process is gone: closed, or crashed.
    let alive = || {
        adb_on(&["shell", "pidof", package])
            .output()
            .is_ok_and(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
    };
    let mut seen = false;
    let mut gone = 0;
    loop {
        std::thread::sleep(Duration::from_secs(1));
        if alive() {
            seen = true;
            gone = 0;
        } else if seen || gone > 10 {
            gone += 1;
            if gone >= 2 {
                break;
            }
        } else {
            gone += 1;
        }
    }
    let _ = logcat.kill();
    println!("{} has stopped.", project.package);
    Ok(ExitCode::SUCCESS)
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
    println!("Starting the {avd} emulator…");
    Command::new(&emulator)
        .args(["-avd", avd])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the emulator: {e}"))?;

    let started = std::time::Instant::now();
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
    match logcat_line(line) {
        Some((true, text)) => eprintln!("{text}"),
        Some((false, text)) => println!("{text}"),
        // `--------- beginning of main` and the like.
        None => {}
    }
}

fn run_web(
    project: &Project,
    browser: Option<&Path>,
    options: &RunOptions,
) -> Result<ExitCode, String> {
    let site = cargo::build_web(project, options.mode)?;
    let server = serve::start(site, options.port)?;
    let url = &server.url;
    let Some(browser) = browser else {
        println!("{} is served at {url}: open it in a browser with WebGPU. Its console shows here; Ctrl+C stops.", project.package);
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    };

    // A profile of its own: the browser is a fresh instance this run owns and can close, not a
    // tab in the developer's own windows.
    let profile = std::env::temp_dir().join(format!("frus-run-{}", std::process::id()));
    println!("Opening {url}… The page's console shows here; closing the browser ends the run.");
    let status = Command::new(browser)
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
        .status()
        .map_err(|e| format!("could not open {}: {e}", browser.display()))?;
    let _ = std::fs::remove_dir_all(&profile);
    Ok(exit_code(status.code()))
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
}
