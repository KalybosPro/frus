//! Building an application, for each platform, with the tools it already uses: cargo,
//! cargo-apk and wasm-bindgen. Their own output is shown as it comes.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use crate::project::{json_string, Project};
use crate::setup;

/// Debug or release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Debug,
    Release,
}

impl Mode {
    /// The directory cargo writes this mode's build to.
    fn dir(self) -> &'static str {
        match self {
            Mode::Debug => "debug",
            Mode::Release => "release",
        }
    }
}

/// The desktop program, built: the path of the executable.
pub(crate) fn build_desktop(project: &Project, mode: Mode) -> Result<PathBuf, String> {
    let mut command = Command::new("cargo");
    command
        .args(["build", "--message-format=json-render-diagnostics"])
        .current_dir(&project.root);
    if mode == Mode::Release {
        command.arg("--release");
    }
    // The messages are JSON on stdout; cargo's progress and the compiler's diagnostics,
    // rendered for people, go to stderr and straight through.
    let mut child = command
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    let mut executable = None;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(path) = executable_of(&line) {
                executable = Some(PathBuf::from(path));
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("the build failed".into());
    }
    executable.ok_or_else(|| {
        format!(
            "{} has no binary to run on the desktop: add `src/bin/{}.rs` (see `frus create`)",
            project.package, project.package
        )
    })
}

/// The executable a `compiler-artifact` message names, if it names one.
pub(crate) fn executable_of(message: &str) -> Option<String> {
    if !message.contains("\"reason\":\"compiler-artifact\"") {
        return None;
    }
    json_string(message, "executable")
}

/// The web build: the library compiled to wasm, and wasm-bindgen's JavaScript written into
/// `web/pkg/`. Returns the directory to serve.
pub(crate) fn build_web(project: &Project, mode: Mode) -> Result<PathBuf, String> {
    let web = project.web_dir();
    if !web.join("index.html").is_file() {
        return Err(format!(
            "{} has no web/index.html: the page the application runs in (`frus create` writes one)",
            project.package
        ));
    }
    if !project.has_lib {
        return Err(format!(
            "{} has no library to compile for the web",
            project.package
        ));
    }
    // What the web build needs, installed if it is missing: the target, and the wasm-bindgen
    // program at exactly the version the lockfile holds.
    setup::rust_target("wasm32-unknown-unknown")?;
    let wasm_bindgen = setup::cargo_tool(
        "wasm-bindgen",
        "--version",
        "wasm-bindgen-cli",
        project.locked_wasm_bindgen().as_deref(),
    )?;

    // Release builds use the size-first profile `frus create` declares, when there is one.
    let profile = match mode {
        Mode::Debug => "dev",
        Mode::Release if project.has_web_release => "web-release",
        Mode::Release => "release",
    };
    let profile_dir = match profile {
        "dev" => "debug",
        other => other,
    };
    run(Command::new("cargo")
        .args([
            "build",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
            "--profile",
            profile,
        ])
        .current_dir(&project.root))?;

    let wasm = project
        .target_dir()?
        .join("wasm32-unknown-unknown")
        .join(profile_dir)
        .join(format!("{}.wasm", project.lib));
    run(Command::new(wasm_bindgen)
        .args(["--target", "web", "--no-typescript", "--out-dir"])
        .arg(web.join("pkg"))
        .arg(&wasm))?;
    Ok(web)
}

/// The Android package, built with cargo-apk: the path of the `.apk`.
pub(crate) fn build_apk(project: &Project, mode: Mode) -> Result<PathBuf, String> {
    if !project.has_lib {
        return Err(format!(
            "{} has no library: Android loads the application's library (`[lib]` with \
             crate-type \"cdylib\")",
            project.package
        ));
    }
    // What building for Android needs, installed if it is missing: an SDK with its NDK, the
    // Rust targets the manifest lists, and cargo-apk.
    setup::android_sdk(project.android_target_sdk)?;
    for target in &project.android_targets {
        setup::rust_target(target)?;
    }
    setup::cargo_tool("cargo-apk", "--help", "cargo-apk", None)?;

    let started = SystemTime::now();
    let mut command = Command::new("cargo");
    // `--lib`: the package has a library (Android's) and a binary (the desktop's), and asked
    // for both cargo-apk stops with a panic after writing the APK.
    command
        .args(["apk", "build", "--lib"])
        .current_dir(&project.root);
    if mode == Mode::Release {
        command.arg("--release");
    }
    run(&mut command)?;
    // Only an APK written by this build: a failed one must never install yesterday's.
    newest_apk(&project.target_dir()?.join(mode.dir()).join("apk"), started).ok_or_else(|| {
        "cargo-apk finished but wrote no new .apk in the target directory".to_string()
    })
}

/// The `.apk` in `dir` written after `since`, the newest if several.
fn newest_apk(dir: &Path, since: SystemTime) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "apk"))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .filter(|(modified, _)| *modified >= since)
        .max()
        .map(|(_, path)| path)
}

/// Runs a command with its output shown, and fails if it does.
pub(crate) fn run(command: &mut Command) -> Result<(), String> {
    let program = command.get_program().to_string_lossy().into_owned();
    // An editor reads standard output as JSON: what the tools print goes to standard error.
    if crate::report::is_machine() {
        command.stdout(std::io::stderr());
    }
    let status = command
        .status()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed ({status})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_executable_comes_from_the_artifact_message() {
        let bin = r#"{"reason":"compiler-artifact","package_id":"x","target":{"kind":["bin"]},"executable":"D:\\app\\target\\debug\\app.exe","fresh":false}"#;
        let lib = r#"{"reason":"compiler-artifact","package_id":"x","target":{"kind":["lib"]},"executable":null,"fresh":true}"#;
        let other = r#"{"reason":"build-finished","success":true}"#;
        assert_eq!(
            executable_of(bin).as_deref(),
            Some(r"D:\app\target\debug\app.exe")
        );
        assert_eq!(executable_of(lib), None);
        assert_eq!(executable_of(other), None);
    }

    #[test]
    fn only_a_fresh_apk_counts() {
        let dir = std::env::temp_dir().join(format!("frus-apk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("old.apk"), b"x").unwrap();
        let after_old = SystemTime::now() + std::time::Duration::from_secs(3600);
        assert_eq!(newest_apk(&dir, after_old), None);
        assert_eq!(
            newest_apk(&dir, SystemTime::UNIX_EPOCH),
            Some(dir.join("old.apk"))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
