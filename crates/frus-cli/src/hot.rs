//! `frus run --hot`: **true hot reload** on the desktop (milestone 670). The application keeps
//! running; each saved change is compiled into a patch and applied to it in place, with every
//! state where it was.
//!
//! The patching is subsecond's, driven by Dioxus's CLI (`dx serve --hot-patch`). Three things make
//! it work for a frus application without changing it:
//!
//! - **The engine** is `dx` 0.7.10 with the fix for its Windows stubs (DioxusLabs/dioxus#5966),
//!   installed once into `~/.frus/tools` until a release has the fix.
//! - **A shadow package** in `target/frus-hot/`: a patch replaces the program's own crate, and a
//!   frus application keeps its code in its library. The shadow is a package whose program *is*
//!   `src/lib.rs`, with links to the application's folders, so that `dx` watches the
//!   application's sources as its own. The application's manifest is never touched.
//! - **The `hot-reload` feature** of frus (milestone 669), turned on for that build only.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::project::Project;

/// Where the fixed `dx` comes from: the fix on top of the 0.7.10 release, which matches the
/// patch protocol of frus's `hot-reload` feature.
const ENGINE_REPO: &str = "https://github.com/KalybosPro/dioxus";
const ENGINE_REV: &str = "7d966a3053ba4eec8c6366718227a4d953670e97";

/// The patch engine, installed first if it is missing (a few minutes, once).
pub(crate) fn engine() -> Result<PathBuf, String> {
    let root = crate::setup::frus_home()
        .join("tools")
        .join(format!("dx-{}", &ENGINE_REV[..7]));
    let exe = root
        .join("bin")
        .join(if cfg!(windows) { "dx.exe" } else { "dx" });
    if exe.is_file() {
        return Ok(exe);
    }
    crate::report::say(
        "Installing the hot-reload engine (dx 0.7.10 with a fix; cargo install, a few minutes, once)…",
    );
    crate::cargo::run(
        Command::new("cargo")
            .args([
                "install",
                "--locked",
                "--git",
                ENGINE_REPO,
                "--rev",
                ENGINE_REV,
            ])
            .arg("--root")
            .arg(&root)
            .arg("dioxus-cli"),
    )?;
    if exe.is_file() {
        Ok(exe)
    } else {
        Err(format!(
            "the hot-reload engine was installed but {} is not there",
            exe.display()
        ))
    }
}

/// What the shadow package does not link: the manifest and lock it has its own versions of, the
/// build output, and the version-control and editor folders.
fn is_kept_out(name: &str) -> bool {
    matches!(name, "Cargo.toml" | "Cargo.lock" | "target") || name.starts_with('.')
}

/// Makes the shadow package for `project` and gives its directory.
pub(crate) fn shadow(project: &Project) -> Result<PathBuf, String> {
    let dir = project
        .target_dir()?
        .join("frus-hot")
        .join(&project.package);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let entries =
        std::fs::read_dir(&project.root).map_err(|e| format!("{}: {e}", project.root.display()))?;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_kept_out(&name) {
            continue;
        }
        let link = dir.join(&name);
        if link.exists() || link.symlink_metadata().is_ok() {
            continue;
        }
        let made = if entry.path().is_dir() {
            link_dir(&entry.path(), &link)
        } else {
            // A file: hard-linked where it can be, so an edit is seen; copied otherwise.
            std::fs::hard_link(entry.path(), &link)
                .or_else(|_| std::fs::copy(entry.path(), &link).map(|_| ()))
        };
        made.map_err(|e| format!("could not link {}: {e}", link.display()))?;
    }
    let manifest = std::fs::read_to_string(project.root.join("Cargo.toml"))
        .map_err(|e| format!("Cargo.toml: {e}"))?;
    write_if_changed(
        &dir.join("Cargo.toml"),
        &shadow_manifest(&manifest, &project.root, &project.package),
    )?;
    if let Ok(lock) = std::fs::read(project.root.join("Cargo.lock")) {
        let _ = std::fs::write(dir.join("Cargo.lock"), lock);
    }
    Ok(dir)
}

fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A link to the directory `target` at `link`: a junction on Windows, which needs no privilege,
/// a symbolic link elsewhere.
fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other("mklink /J failed"))
        }
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

/// The shadow package's manifest, from the application's: its `[lib]` and binaries replaced by one
/// program built from `src/lib.rs`, and its relative paths made absolute from `root`.
pub(crate) fn shadow_manifest(manifest: &str, root: &Path, package: &str) -> String {
    let mut out = String::from(
        "# Written by `frus run --hot` (milestone 670) from the application's own manifest:\n\
         # its library compiled as the program a hot-reload patch replaces.\n",
    );
    let mut skipping = false;
    let mut has_workspace = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            skipping = trimmed == "[lib]" || trimmed == "[[bin]]";
            has_workspace |= trimmed == "[workspace]";
        }
        if skipping {
            continue;
        }
        out.push_str(&absolute_paths(line, root));
        out.push('\n');
    }
    if !has_workspace {
        out.push_str("\n[workspace]\n");
    }
    out.push_str(&format!(
        "\n[[bin]]\nname = \"{package}\"\npath = \"src/lib.rs\"\n"
    ));
    out
}

/// `line` with each relative `path = "…"` made absolute from `root`.
fn absolute_paths(line: &str, root: &Path) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("path") {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let after_key = &after[4..];
        let value = after_key.trim_start();
        let Some(value) = value.strip_prefix('=') else {
            out.push_str("path");
            rest = after_key;
            continue;
        };
        let value = value.trim_start();
        let Some(quoted) = value.strip_prefix('"') else {
            out.push_str("path");
            rest = after_key;
            continue;
        };
        let Some(end) = quoted.find('"') else {
            out.push_str("path");
            rest = after_key;
            continue;
        };
        let path = &quoted[..end];
        let absolute = if Path::new(path).is_absolute() {
            path.to_string()
        } else {
            root.join(path).display().to_string().replace('\\', "/")
        };
        out.push_str(&format!("path = \"{absolute}\""));
        rest = &quoted[end + 1..];
    }
    out.push_str(rest);
    out
}

/// What one line of `dx`'s output says.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DxLine {
    /// A line of the application's own log, and whether it is an error.
    App(String, bool),
    /// The application was built and is starting.
    Launched,
    /// A patch was built for `file`, in that many milliseconds.
    Patched { file: String, millis: u64 },
    /// The application is gone.
    Exited,
    /// Something failed: the build, a patch.
    Failed(String),
    /// Nothing the developer needs.
    Noise,
}

/// `line` without the terminal's colour codes.
pub(crate) fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Reads one line of `dx serve`'s output. It prints its own messages and the application's
/// output, each behind the time since it started and a level: the application's standard error
/// as `ERROR` whatever it says, which is where a frus application's log goes.
pub(crate) fn dx_line(line: &str) -> DxLine {
    let line = plain(line);
    let text = line.trim();
    // `  63.23s ERROR <the application's line>`, `302.18s  INFO <dx's message>`.
    let body = text
        .split_once("s ")
        .filter(|(time, _)| time.parse::<f32>().is_ok())
        .map(|(_, rest)| rest.trim_start())
        .unwrap_or(text);
    if let Some(app) = body.strip_prefix("ERROR ") {
        if app.starts_with("Application [") && app.contains("exited") {
            return DxLine::Exited;
        }
        if app.starts_with("Build failed") || app.starts_with("Hot-patching failed") {
            return DxLine::Failed(app.to_string());
        }
        let error = app.contains(" ERROR ") || app.contains(" WARN ") || app.contains("panicked");
        return DxLine::App(app.to_string(), error);
    }
    if let Some(info) = body.strip_prefix("INFO ") {
        if info.contains("launching app") {
            return DxLine::Launched;
        }
        if let Some(rest) = info.strip_prefix("Hot-patching: ") {
            let (file, took) = rest.split_once(" took ").unwrap_or((rest, ""));
            let millis = took.trim_end_matches("ms").trim().parse().unwrap_or(0);
            return DxLine::Patched {
                file: file.trim().to_string(),
                millis,
            };
        }
        if info.starts_with("Application [") && info.contains("exited") {
            return DxLine::Exited;
        }
    }
    if body.contains("Build failed") || body.starts_with("error") {
        return DxLine::Failed(body.to_string());
    }
    DxLine::Noise
}

/// Stops `dx` and the application it started.
pub(crate) fn stop(dx: &mut Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &dx.id().to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = dx.kill();
    let _ = dx.wait();
}

/// The platform flag `dx serve` takes for this computer.
pub(crate) fn platform_flag() -> &'static str {
    if cfg!(windows) {
        "--windows"
    } else if cfg!(target_os = "macos") {
        "--macos"
    } else {
        "--linux"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shadow_manifest_builds_the_library_as_the_program() {
        let manifest = r#"[package]
name = "shop"
version = "0.1.0"

[workspace]

[lib]
crate-type = ["rlib", "cdylib"]

[[bin]]
name = "shop"
path = "src/bin/shop.rs"

[dependencies]
frus = { path = "../../crates/frus", default-features = false }
helper = { path = "/abs/helper" }
serde = "1"

[package.metadata.android]
resources = "res"
"#;
        let out = shadow_manifest(manifest, Path::new("/apps/shop"), "shop");
        assert!(!out.contains("[lib]"), "{out}");
        assert!(!out.contains("src/bin/shop.rs"), "{out}");
        assert!(!out.contains("crate-type"), "{out}");
        assert!(
            out.contains("[[bin]]\nname = \"shop\"\npath = \"src/lib.rs\"\n"),
            "{out}"
        );
        assert!(
            out.contains(
                r#"frus = { path = "/apps/shop/../../crates/frus", default-features = false }"#
            ),
            "{out}"
        );
        assert!(
            out.contains(r#"helper = { path = "/abs/helper" }"#),
            "{out}"
        );
        assert!(out.contains("serde = \"1\""), "{out}");
        assert!(out.contains("resources = \"res\""), "{out}");
        assert_eq!(out.matches("[workspace]").count(), 1, "{out}");
    }

    #[test]
    fn a_manifest_without_a_workspace_gets_one() {
        let out = shadow_manifest(
            "[package]\nname = \"a\"\n\n[dependencies]\n",
            Path::new("/a"),
            "a",
        );
        assert!(out.contains("[workspace]"), "{out}");
    }

    #[test]
    fn dx_says_what_happens() {
        assert_eq!(
            dx_line("\u{1b}[0m 302.18s  INFO Build completed successfully in \u{1b}[33m285.20s\u{1b}[0m, launching app! 💫"),
            DxLine::Launched
        );
        assert_eq!(
            dx_line(" 39.18s  INFO Hot-patching: \u{1b}[32msrc\\features\\counter\\widgets\\count_display.rs\u{1b}[0m took \u{1b}[33m2325ms\u{1b}[0m"),
            DxLine::Patched {
                file: r"src\features\counter\widgets\count_display.rs".into(),
                millis: 2325
            }
        );
        assert_eq!(
            dx_line(" 39.51s ERROR [2026-10-18T06:12:00Z INFO  frus_shell::hot] hot reload: patch applied"),
            DxLine::App(
                "[2026-10-18T06:12:00Z INFO  frus_shell::hot] hot reload: patch applied".into(),
                false
            )
        );
        assert_eq!(
            dx_line(" 39.52s ERROR thread 'main' (12040) panicked at src/lib.rs:3:5:"),
            DxLine::App(
                "thread 'main' (12040) panicked at src/lib.rs:3:5:".into(),
                true
            )
        );
        assert_eq!(
            dx_line(" 39.97s ERROR Application [windows] exited with error: exit code: 0xc000041d"),
            DxLine::Exited
        );
        assert!(matches!(
            dx_line("106.20s ERROR \u{1b}[31mBuild failed\u{1b}[0m: os error 1224"),
            DxLine::Failed(_)
        ));
        assert_eq!(
            dx_line("\u{1b}[33mwarning\u{1b}[0m: (Try 3) Taking a while..."),
            DxLine::Noise
        );
    }
}
