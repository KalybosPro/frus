//! `frus doctor`: what this machine has for building frus applications, and what it lacks
//! (milestone 672).
//!
//! Each part of the toolchain is looked at — Rust and its targets, the Android SDK and what
//! building for Android needs, the web build's tools, the hot-reload engine, VS Code, the devices —
//! and said in one line, `[✓]` ready, `[!]` usable but incomplete, `[✗]` missing. `-v` says
//! everything that was found; without it, only what is wrong. `--fix` installs what is missing,
//! the way the other commands do when they need it. `--machine` answers in JSON, for an editor.
//!
//! Nothing is installed or changed without `--fix`: the doctor only looks.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use crate::devices::{self, Kind, Platform};
use crate::json::{object, Value};
use crate::setup;

/// How a part of the toolchain stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    /// Ready.
    Ready,
    /// Usable, with something missing or worth knowing.
    Partial,
    /// Missing: what needs it does not work.
    Missing,
}

impl Status {
    fn mark(self) -> &'static str {
        match self {
            Status::Ready => "[✓]",
            Status::Partial => "[!]",
            Status::Missing => "[✗]",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Status::Ready => "ready",
            Status::Partial => "partial",
            Status::Missing => "missing",
        }
    }
}

/// One line under a part: what was found, or what is wrong and how to put it right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// `None` for information, `Some(true)` for something present, `Some(false)` for a problem.
    pub ok: Option<bool>,
    pub text: String,
}

impl Finding {
    fn info(text: impl Into<String>) -> Self {
        Self {
            ok: None,
            text: text.into(),
        }
    }
    fn found(text: impl Into<String>) -> Self {
        Self {
            ok: Some(true),
            text: text.into(),
        }
    }
    fn problem(text: impl Into<String>) -> Self {
        Self {
            ok: Some(false),
            text: text.into(),
        }
    }
}

/// One part of the toolchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    pub name: String,
    pub status: Status,
    /// What follows the name on the summary line: a version, a count.
    pub summary: String,
    pub findings: Vec<Finding>,
    /// Whether `--fix` can put it right: iOS, or an editor's extension, it cannot.
    pub fixable: bool,
}

impl Check {
    /// A part, ready unless a finding is a problem; `required` problems make it missing rather
    /// than partial.
    fn from(name: &str, summary: String, findings: Vec<Finding>, required: bool) -> Self {
        let problems = findings.iter().any(|f| f.ok == Some(false));
        let status = match (problems, required) {
            (false, _) => Status::Ready,
            (true, true) => Status::Missing,
            (true, false) => Status::Partial,
        };
        Self {
            name: name.to_string(),
            status,
            summary,
            findings,
            fixable: true,
        }
    }

    fn unfixable(mut self) -> Self {
        self.fixable = false;
        self
    }
}

/// What `frus doctor` was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Options {
    pub verbose: bool,
    pub fix: bool,
    pub machine: bool,
}

pub(crate) fn doctor(options: Options) -> ExitCode {
    if options.fix {
        fix();
    }
    let checks = checks();
    if options.machine {
        println!(
            "{}",
            Value::Array(checks.iter().map(to_json).collect()).to_json()
        );
    } else {
        print!("{}", render(&checks, options.verbose));
    }
    ExitCode::SUCCESS
}

/// Every part, in the order a developer meets them.
fn checks() -> Vec<Check> {
    let rust = rust();
    let targets = installed_targets();
    vec![
        frus(),
        rust,
        desktop(),
        android(&targets),
        web(&targets),
        hot_reload(),
        ios(),
        vscode(),
        connected(),
    ]
}

/// The summary, and with `verbose` every finding — without it, only the problems.
pub(crate) fn render(checks: &[Check], verbose: bool) -> String {
    let mut out = String::from(if verbose {
        "Doctor:\n"
    } else {
        "Doctor summary (to see everything that was found, run frus doctor -v):\n"
    });
    for check in checks {
        if check.summary.is_empty() {
            out.push_str(&format!("{} {}\n", check.status.mark(), check.name));
        } else {
            out.push_str(&format!(
                "{} {} ({})\n",
                check.status.mark(),
                check.name,
                check.summary
            ));
        }
        for finding in &check.findings {
            let show = verbose || finding.ok == Some(false);
            if !show {
                continue;
            }
            let bullet = match finding.ok {
                None => "•",
                Some(true) => "✓",
                Some(false) => "✗",
            };
            out.push_str(&format!("    {bullet} {}\n", finding.text));
        }
    }
    let issues = checks
        .iter()
        .filter(|c| c.status != Status::Ready && c.fixable)
        .count();
    if issues == 0 {
        out.push_str("\n• No issues found.\n");
    } else {
        out.push_str(&format!(
            "\n! {issues} part{} with issues: `frus doctor --fix` installs what is missing.\n",
            if issues == 1 { "" } else { "s" }
        ));
    }
    out
}

fn to_json(check: &Check) -> Value {
    object([
        ("name", check.name.as_str().into()),
        ("status", check.status.name().into()),
        ("summary", check.summary.as_str().into()),
        (
            "findings",
            Value::Array(
                check
                    .findings
                    .iter()
                    .map(|f| {
                        object([
                            ("ok", f.ok.map_or(Value::Null, Value::Bool)),
                            ("text", f.text.as_str().into()),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// What a program prints when asked, its first line, if it runs.
fn first_line(program: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    String::from_utf8_lossy(&text)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

// --- The parts ---

fn frus() -> Check {
    let at = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    Check::from(
        "frus",
        env!("CARGO_PKG_VERSION").to_string(),
        vec![Finding::info(format!("at {at}"))],
        true,
    )
}

fn rust() -> Check {
    let mut findings = Vec::new();
    let rustc = first_line("rustc", &["--version"]);
    let host = Command::new("rustc")
        .arg("-vV")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .find_map(|l| l.strip_prefix("host: ").map(str::to_string))
        });
    match &rustc {
        Some(version) => findings.push(Finding::found(version.clone())),
        None => findings.push(Finding::problem(
            "Rust is not installed: install it from https://rustup.rs",
        )),
    }
    match first_line("cargo", &["--version"]) {
        Some(version) => findings.push(Finding::found(version)),
        None => findings.push(Finding::problem(
            "cargo is not installed (it comes with rustup)",
        )),
    }
    if first_line("rustup", &["--version"]).is_none() {
        findings.push(Finding::problem(
            "rustup is not installed: frus adds the Android and web targets through it \
             (https://rustup.rs)",
        ));
    }
    let summary = match (&rustc, &host) {
        (Some(v), Some(h)) => format!("{}, {h}", v.split_whitespace().nth(1).unwrap_or(v.as_str())),
        (Some(v), None) => v.clone(),
        _ => String::new(),
    };
    Check::from("Rust", summary, findings, true)
}

/// The targets rustup has installed.
fn installed_targets() -> Vec<String> {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// What building for this computer needs beyond Rust: on Windows, Microsoft's linker.
fn desktop() -> Check {
    let os = std::env::consts::OS;
    let mut findings = Vec::new();
    if cfg!(windows) {
        let msvc = Command::new("rustc")
            .arg("-vV")
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("msvc"));
        if msvc {
            match visual_studio_tools() {
                Some(at) => findings.push(Finding::found(format!(
                    "Visual Studio's C++ build tools, at {}",
                    at.display()
                ))),
                None => findings.push(Finding::problem(
                    "Visual Studio's C++ build tools were not found: Rust links Windows programs \
                     with them. Install \"Desktop development with C++\" from \
                     https://visualstudio.microsoft.com/visual-cpp-build-tools/",
                )),
            }
        }
    } else if cfg!(target_os = "linux") {
        if first_line("pkg-config", &["--version"]).is_none() {
            findings.push(Finding::problem(
                "pkg-config is not installed: some of the libraries a desktop build links are found \
                 through it (sudo apt install pkg-config)",
            ));
        }
    } else {
        findings.push(Finding::info("the system's own tools are enough"));
    }
    Check::from(&format!("Desktop ({os})"), String::new(), findings, true)
}

/// Where Visual Studio's C++ tools are, through the installer's own `vswhere`.
fn visual_studio_tools() -> Option<PathBuf> {
    let program_files = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    let vswhere = program_files
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    let path = first_line(
        vswhere,
        &[
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ],
    )?;
    Some(PathBuf::from(path))
}

/// The Android targets frus builds for by default.
const ANDROID_TARGETS: &[&str] = &["aarch64-linux-android"];

fn android(targets: &[String]) -> Check {
    let mut findings = Vec::new();
    let Some(sdk) = setup::existing_sdk() else {
        return Check::from(
            "Android toolchain",
            String::new(),
            vec![Finding::problem(
                "no Android SDK: `frus doctor --fix` (or the first `frus build apk`) installs one",
            )],
            true,
        );
    };
    findings.push(Finding::found(format!("Android SDK at {}", sdk.display())));
    let present = |dir: &Path, what: &str, fix: &str| match setup::newest_in(dir) {
        Some(found) => Finding::found(format!(
            "{what} {}",
            found.file_name().unwrap_or_default().to_string_lossy()
        )),
        None => Finding::problem(format!("{what} is not installed: {fix}")),
    };
    findings.push(if sdk.join("platform-tools").is_dir() {
        Finding::found(format!(
            "platform-tools ({})",
            first_line(sdk.join("platform-tools").join(exe("adb")), &["--version"])
                .unwrap_or_else(|| "adb".into())
        ))
    } else {
        Finding::problem("platform-tools (adb) is not installed: `frus doctor --fix`")
    });
    findings.push(present(
        &sdk.join("platforms"),
        "Android platform",
        "`frus doctor --fix`",
    ));
    findings.push(present(
        &sdk.join("build-tools"),
        "build-tools",
        "`frus doctor --fix`",
    ));
    findings.push(present(&sdk.join("ndk"), "NDK", "`frus doctor --fix`"));
    findings.push(match setup::find_java() {
        Some(java) => Finding::found(format!("Java at {}", java.display())),
        None => Finding::problem("no Java (the SDK's tools run on it): `frus doctor --fix`"),
    });
    for target in ANDROID_TARGETS {
        findings.push(if targets.iter().any(|t| t == target) {
            Finding::found(format!("Rust target {target}"))
        } else {
            Finding::problem(format!(
                "the Rust target {target} is not installed: `frus doctor --fix`"
            ))
        });
    }
    findings.push(if cargo_subcommand_exists("cargo-apk") {
        Finding::found("cargo-apk")
    } else {
        Finding::problem("cargo-apk is not installed: `frus doctor --fix`")
    });
    if sdk.join("emulator").is_dir() {
        findings.push(Finding::found("the Android emulator"));
    } else {
        findings.push(Finding::info(
            "no Android emulator (only needed to run without a phone): `frus emulators --create` \
             installs it",
        ));
    }
    let summary = setup::newest_in(&sdk.join("platforms"))
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_default();
    Check::from("Android toolchain", summary, findings, false)
}

fn web(targets: &[String]) -> Check {
    let mut findings = Vec::new();
    findings.push(if targets.iter().any(|t| t == "wasm32-unknown-unknown") {
        Finding::found("Rust target wasm32-unknown-unknown")
    } else {
        Finding::problem(
            "the Rust target wasm32-unknown-unknown is not installed: `frus doctor --fix`",
        )
    });
    let mut bindgen: Vec<String> = Vec::new();
    if let Some(version) = first_line("wasm-bindgen", &["--version"]) {
        bindgen.push(version);
    }
    if let Ok(entries) = std::fs::read_dir(setup::frus_home().join("tools")) {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(version) = name.strip_prefix("wasm-bindgen-cli-") {
                bindgen.push(format!("wasm-bindgen {version} (frus's)"));
            }
        }
    }
    if bindgen.is_empty() {
        findings.push(Finding::info(
            "no wasm-bindgen yet: the first web build installs the version the application locks",
        ));
    } else {
        for found in &bindgen {
            findings.push(Finding::found(found.clone()));
        }
    }
    let browsers: Vec<String> = devices::discover()
        .into_iter()
        .filter(|d| matches!(d.kind, Kind::Browser { .. }))
        .map(|d| d.name)
        .collect();
    if browsers.is_empty() {
        findings.push(Finding::info(
            "no browser frus knows (Edge, Chrome): `frus run -d web-server` serves the application \
             to any browser with WebGPU",
        ));
    } else {
        findings.push(Finding::found(format!("browsers: {}", browsers.join(", "))));
    }
    let summary = browsers.join(", ");
    Check::from("Web", summary, findings, false)
}

fn hot_reload() -> Check {
    let engine = crate::hot::engine_path();
    let finding = if engine.is_file() {
        Finding::found(format!("the hot-reload engine at {}", engine.display()))
    } else {
        Finding::problem(
            "the hot-reload engine is not installed (`frus run --hot` installs it, about 20 minutes \
             once; so does `frus doctor --fix`)",
        )
    };
    Check::from(
        "Hot reload (frus run --hot)",
        String::new(),
        vec![finding],
        false,
    )
}

fn ios() -> Check {
    let mut findings = vec![Finding::problem(
        "frus cannot run on iOS yet: its shell does not exist (ROADMAP, issue #19)",
    )];
    if cfg!(target_os = "macos") {
        if let Some(xcode) = first_line("xcodebuild", &["-version"]) {
            findings.insert(0, Finding::found(xcode));
        }
    }
    // Not something to fix here: said, and not counted as an issue a fix could take away.
    Check::from("iOS", String::new(), findings, false).unfixable()
}

fn vscode() -> Check {
    let code = if cfg!(windows) { "code.cmd" } else { "code" };
    let Some(version) = first_line(code, &["--version"]) else {
        return Check::from(
            "VS Code",
            String::new(),
            vec![Finding::info(
                "not found: any editor works; VS Code has the frus extension (editors/vscode)",
            )],
            false,
        );
    };
    let extensions = Command::new(code)
        .args(["--list-extensions", "--show-versions"])
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let ours = extensions
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("kalybospro.frus@"));
    let finding = match ours {
        Some(line) => Finding::found(format!(
            "the frus extension {}",
            line.split('@').nth(1).unwrap_or("")
        )),
        None => Finding::problem(
            "the frus extension is not installed: device picker, F5, reload on save \
             (editors/vscode)",
        ),
    };
    Check::from("VS Code", version, vec![finding], false).unfixable()
}

fn connected() -> Check {
    let all = devices::discover();
    let available: Vec<_> = all
        .iter()
        .filter(|d| d.unavailable.is_none() && !matches!(d.kind, Kind::Emulator { .. }))
        .collect();
    let mut findings: Vec<Finding> = all
        .iter()
        .map(|d| match &d.unavailable {
            None => Finding::found(format!("{} ({}, {})", d.name, d.id, d.detail)),
            Some(why) => Finding::info(format!("{} ({}): {why}", d.name, d.id)),
        })
        .collect();
    if !all.iter().any(|d| d.platform == Platform::Android) {
        findings.push(Finding::info(
            "no phone connected: plug one in with USB debugging on, or start an emulator",
        ));
    }
    Check::from(
        "Connected devices",
        format!("{} available", available.len()),
        findings,
        false,
    )
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Whether `cargo <name without "cargo-">` exists: its program in cargo's `bin`, or on the `PATH`.
fn cargo_subcommand_exists(program: &str) -> bool {
    let in_cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|h| PathBuf::from(h).join(".cargo"))
        })
        .is_some_and(|home| home.join("bin").join(exe(program)).is_file());
    in_cargo_home || first_line(program, &["--help"]).is_some()
}

/// `--fix`: installs what the checks find missing, with the installers the other commands use.
fn fix() {
    let say = crate::report::say;
    let targets = installed_targets();
    for target in ANDROID_TARGETS
        .iter()
        .copied()
        .chain(["wasm32-unknown-unknown"])
    {
        if !targets.iter().any(|t| t == target) {
            if let Err(error) = setup::rust_target(target) {
                say(&format!("could not add {target}: {error}"));
            }
        }
    }
    // Only when the doctor found something wrong there: a fix never downloads what it found fine.
    // The application's target when the doctor runs in one; otherwise the newest platform
    // already installed.
    let android_ok = android(&targets).status == Status::Ready;
    let level = std::env::current_dir()
        .ok()
        .and_then(|here| crate::project::Project::find(&here).ok())
        .map(|project| project.android_target_sdk)
        .or_else(|| {
            let sdk = setup::existing_sdk()?;
            let newest = setup::newest_in(&sdk.join("platforms"))?;
            let name = newest.file_name()?.to_string_lossy().into_owned();
            name.trim_start_matches("android-")
                .split(['.', '-'])
                .next()?
                .parse()
                .ok()
        })
        .unwrap_or(34);
    if !android_ok {
        if let Err(error) = setup::android_sdk(level) {
            say(&format!("could not complete the Android SDK: {error}"));
        }
    }
    if !cargo_subcommand_exists("cargo-apk") {
        if let Err(error) = setup::cargo_tool("cargo-apk", "--help", "cargo-apk", None) {
            say(&format!("could not install cargo-apk: {error}"));
        }
    }
    if !crate::hot::engine_path().is_file() {
        if let Err(error) = crate::hot::engine() {
            say(&format!("could not install the hot-reload engine: {error}"));
        }
    }
    say("");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(name: &str, findings: Vec<Finding>, required: bool) -> Check {
        Check::from(name, String::new(), findings, required)
    }

    #[test]
    fn a_part_is_as_good_as_its_findings() {
        assert_eq!(
            check("a", vec![Finding::found("x"), Finding::info("y")], true).status,
            Status::Ready
        );
        assert_eq!(
            check("a", vec![Finding::found("x"), Finding::problem("y")], false).status,
            Status::Partial
        );
        assert_eq!(
            check("a", vec![Finding::problem("y")], true).status,
            Status::Missing
        );
    }

    #[test]
    fn the_summary_shows_problems_and_the_details_everything() {
        let checks = vec![
            Check::from(
                "Rust",
                "1.96.1".into(),
                vec![Finding::found("rustc 1.96.1")],
                true,
            ),
            Check::from(
                "Android toolchain",
                String::new(),
                vec![
                    Finding::found("Android SDK"),
                    Finding::problem("cargo-apk is not installed"),
                ],
                false,
            ),
        ];
        let summary = render(&checks, false);
        assert!(summary.contains("[✓] Rust (1.96.1)\n"), "{summary}");
        assert!(
            summary.contains("[!] Android toolchain\n    ✗ cargo-apk is not installed\n"),
            "{summary}"
        );
        assert!(!summary.contains("rustc 1.96.1"), "{summary}");
        assert!(summary.contains("1 part with issues"), "{summary}");
        // What a fix cannot reach is said, and not counted.
        let ios = Check::from(
            "iOS",
            String::new(),
            vec![Finding::problem("no shell")],
            false,
        )
        .unfixable();
        let with_ios = render(&[checks[0].clone(), ios], false);
        assert!(with_ios.contains("[!] iOS\n    ✗ no shell\n"), "{with_ios}");
        assert!(with_ios.contains("No issues found."), "{with_ios}");
        let details = render(&checks, true);
        assert!(details.contains("    ✓ rustc 1.96.1\n"), "{details}");
        assert!(details.contains("    ✓ Android SDK\n"), "{details}");

        let fine = render(&checks[..1], false);
        assert!(fine.contains("No issues found."), "{fine}");
    }

    #[test]
    fn an_editor_gets_json() {
        let json = to_json(&Check::from(
            "Web",
            "Edge".into(),
            vec![Finding::problem("no wasm32 target")],
            false,
        ))
        .to_json();
        assert!(json.contains("\"status\":\"partial\""), "{json}");
        assert!(json.contains("\"ok\":false"), "{json}");
    }
}
