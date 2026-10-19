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
//!   `src/lib.rs`. Its `src` is a copy of the application's, kept in step while the run lasts
//!   ([`Mirror`]): `dx` watches it as its own, which it does not do through a link. The other
//!   folders are linked. The application's manifest is never touched.
//! - **The `hot-reload` feature** of frus (milestone 669), turned on for that build only.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::SystemTime;

use crate::project::Project;

/// Where the fixed `dx` comes from: the fix on top of the 0.7.10 release, which matches the
/// patch protocol of frus's `hot-reload` feature.
const ENGINE_REPO: &str = "https://github.com/KalybosPro/dioxus";
const ENGINE_REV: &str = "7d966a3053ba4eec8c6366718227a4d953670e97";

/// What building the engine needs and this machine lacks, as what to do about it: on Linux it
/// links OpenSSL, whose headers a package manager installs, which frus cannot do without the
/// administrator's password.
pub(crate) fn missing_build_requirement() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let found = Command::new("pkg-config")
        .args(["--exists", "openssl"])
        .status()
        .is_ok_and(|s| s.success());
    (!found).then(|| {
        "building the hot-reload engine needs OpenSSL's headers and pkg-config: \
         `sudo apt install pkg-config libssl-dev` (Fedora: `sudo dnf install pkgconf \
         openssl-devel`), then run again"
            .to_string()
    })
}

/// Where the patch engine is installed: `~/.frus/tools/dx-<commit>/`.
fn engine_root() -> PathBuf {
    crate::setup::frus_home()
        .join("tools")
        .join(format!("dx-{}", &ENGINE_REV[..7]))
}

/// The patch engine's program, installed or not.
pub(crate) fn engine_path() -> PathBuf {
    engine_root()
        .join("bin")
        .join(if cfg!(windows) { "dx.exe" } else { "dx" })
}

/// The patch engine, installed first if it is missing (a few minutes, once).
pub(crate) fn engine() -> Result<PathBuf, String> {
    let root = engine_root();
    let exe = engine_path();
    if exe.is_file() {
        return Ok(exe);
    }
    if let Some(missing) = missing_build_requirement() {
        return Err(missing);
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
            .arg("dioxus-cli")
            // Through the `git` program: the repository is large, and cargo's own client gives
            // up on a connection that drops for a moment.
            .env("CARGO_NET_GIT_FETCH_WITH_CLI", "true"),
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
        // The sources are copied, not linked (see `Mirror`). A link left by an earlier version
        // is taken away — the link alone, never what it points to.
        if name == "src" {
            if link
                .symlink_metadata()
                .is_ok_and(|meta| meta.file_type().is_symlink() || is_junction(&link))
            {
                std::fs::remove_dir(&link)
                    .map_err(|e| format!("could not unlink {}: {e}", link.display()))?;
            }
            continue;
        }
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

/// Whether `path` is a directory junction (Windows): a link `is_symlink` does not always report.
fn is_junction(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        path.symlink_metadata()
            .is_ok_and(|meta| meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

/// The application's sources, copied into the shadow package and kept in step: each file whose
/// size or time changed is copied again, each one gone is removed. `dx` sees the copy change as
/// the developer saves, and patches.
pub(crate) struct Mirror {
    from: PathBuf,
    to: PathBuf,
    /// What each source file was when it was last copied.
    copied: HashMap<PathBuf, (u64, Option<SystemTime>)>,
}

impl Mirror {
    pub(crate) fn new(from: PathBuf, to: PathBuf) -> Self {
        Self {
            from,
            to,
            copied: HashMap::new(),
        }
    }

    /// Writes again, unchanged, every copied file changed since `since`: the engine starts watching
    /// a moment after the application is up, and an edit saved in that moment would otherwise
    /// never reach it. Returns how many it wrote.
    pub(crate) fn retouch_since(&self, since: SystemTime) -> std::io::Result<usize> {
        let mut held = HashMap::new();
        if self.to.is_dir() {
            list(&self.to, Path::new(""), &mut held)?;
        }
        let mut touched = 0;
        for (relative, (_, modified)) in held {
            if modified.is_some_and(|m| m > since) {
                let copy = self.to.join(&relative);
                let bytes = std::fs::read(&copy)?;
                std::fs::write(&copy, bytes)?;
                touched += 1;
            }
        }
        Ok(touched)
    }

    /// Brings the copy in step. Returns how many files it changed.
    pub(crate) fn sync(&mut self) -> std::io::Result<usize> {
        let mut found = HashMap::new();
        list(&self.from, Path::new(""), &mut found)?;
        let mut changed = 0;
        for (relative, stamp) in &found {
            if self.copied.get(relative) == Some(stamp) {
                continue;
            }
            let source = self.from.join(relative);
            let copy = self.to.join(relative);
            // The first time, a copy with the same bytes is kept: no change for `dx` to see.
            let same = std::fs::read(&copy).ok() == Some(std::fs::read(&source)?);
            if !same {
                if let Some(parent) = copy.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&source, &copy)?;
                changed += 1;
            }
        }
        // Files the copy has and the sources no longer do.
        let mut held = HashMap::new();
        if self.to.is_dir() {
            list(&self.to, Path::new(""), &mut held)?;
        }
        for relative in held.keys().filter(|r| !found.contains_key(*r)) {
            std::fs::remove_file(self.to.join(relative))?;
            changed += 1;
        }
        self.copied = found;
        Ok(changed)
    }
}

/// Every file under `dir`, by its path relative to the starting directory, with its size and
/// time.
fn list(
    dir: &Path,
    relative: &Path,
    out: &mut HashMap<PathBuf, (u64, Option<SystemTime>)>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir.join(relative))?.filter_map(Result::ok) {
        let path = relative.join(entry.file_name());
        let meta = entry.metadata()?;
        if meta.is_dir() {
            list(dir, &path, out)?;
        } else {
            out.insert(path, (meta.len(), meta.modified().ok()));
        }
    }
    Ok(())
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

/// Removes the archives `dx` links the program's dependencies into (`libdeps-*.a`), wherever
/// they are under `target`. One being written when `dx` was stopped is left half-written, and the
/// next build fails on it ("unknown file type"); `dx` writes it again in seconds.
pub(crate) fn clear_dependency_archives(target: &Path) {
    fn walk(dir: &Path, depth: u32) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if depth < 3 {
                    walk(&path, depth + 1);
                }
            } else if name.starts_with("libdeps-") && name.ends_with(".a") {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    walk(target, 0);
}

/// One located diagnostic of `cargo check --message-format=short`, with its path made the
/// application's: `src/x.rs:40:28: error[E0425]: …` → `<root>/src/x.rs:40:28: error[E0425]: …`.
/// `None` for any other line, and for warnings.
pub(crate) fn located_error(line: &str, shadow: &Path, root: &Path) -> Option<String> {
    let at = line.find(": error")?;
    let (place, rest) = line.split_at(at);
    // `path:line:column`, the path possibly with a drive's colon in it.
    let mut parts = place.rsplitn(3, ':');
    let column = parts.next()?;
    let row = parts.next()?;
    let path = parts.next()?;
    if column.parse::<u32>().is_err() || row.parse::<u32>().is_err() {
        return None;
    }
    let path = Path::new(path);
    let in_app = match path.strip_prefix(shadow) {
        Ok(relative) => root.join(relative),
        Err(_) if path.is_relative() => root.join(path),
        Err(_) => path.to_path_buf(),
    };
    Some(format!("{}:{row}:{column}{rest}", in_app.display()))
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
    // Elsewhere `dx` leads a process group of its own (see `run_hot`): the group goes, the
    // application with it.
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{}", dx.id())])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
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
    fn the_mirror_follows_the_sources() {
        let base = std::env::temp_dir().join(format!("frus-hot-mirror-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (from, to) = (base.join("src"), base.join("copy"));
        std::fs::create_dir_all(from.join("features")).unwrap();
        std::fs::write(from.join("lib.rs"), "mod features;").unwrap();
        std::fs::write(from.join("features").join("a.rs"), "// a").unwrap();
        let mut mirror = Mirror::new(from.clone(), to.clone());
        assert_eq!(mirror.sync().unwrap(), 2);
        assert_eq!(mirror.sync().unwrap(), 0, "nothing changed");
        assert_eq!(
            std::fs::read_to_string(to.join("features").join("a.rs")).unwrap(),
            "// a"
        );

        std::fs::write(from.join("features").join("a.rs"), "// a, changed").unwrap();
        std::fs::write(from.join("features").join("b.rs"), "// b").unwrap();
        std::fs::remove_file(from.join("lib.rs")).unwrap();
        assert_eq!(mirror.sync().unwrap(), 3);
        assert_eq!(
            std::fs::read_to_string(to.join("features").join("a.rs")).unwrap(),
            "// a, changed"
        );
        assert!(to.join("features").join("b.rs").is_file());
        assert!(!to.join("lib.rs").exists());

        // A fresh run over a copy already in step changes nothing.
        let mut again = Mirror::new(from, to);
        assert_eq!(again.sync().unwrap(), 0);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn errors_point_at_the_applications_files() {
        let shadow = Path::new("/apps/shop/target/frus-hot/shop");
        let root = Path::new("/apps/shop");
        assert_eq!(
            located_error(
                "src/features/a.rs:40:28: error[E0425]: cannot find value `x` in this scope",
                shadow,
                root
            )
            .as_deref(),
            Some(
                format!(
                    "{}:40:28: error[E0425]: cannot find value `x` in this scope",
                    root.join("src/features/a.rs").display()
                )
                .as_str()
            )
        );
        assert_eq!(
            located_error(
                "/apps/shop/target/frus-hot/shop/src/lib.rs:3:1: error: expected item",
                shadow,
                root
            )
            .as_deref(),
            Some(
                format!(
                    "{}:3:1: error: expected item",
                    root.join("src/lib.rs").display()
                )
                .as_str()
            )
        );
        // A path outside the shadow is left as it is: an absolute one, on each system.
        let elsewhere = if cfg!(windows) {
            r"C:\a\src\lib.rs"
        } else {
            "/elsewhere/src/lib.rs"
        };
        assert_eq!(
            located_error(&format!("{elsewhere}:3:1: error: x"), shadow, root),
            Some(format!("{elsewhere}:3:1: error: x"))
        );
        assert_eq!(
            located_error("src/a.rs:1:2: warning: unused", shadow, root),
            None
        );
        assert_eq!(
            located_error("error: could not compile `shop`", shadow, root),
            None
        );
    }

    #[test]
    fn an_edit_in_the_engines_blind_moment_is_written_again() {
        let base = std::env::temp_dir().join(format!("frus-hot-retouch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (from, to) = (base.join("src"), base.join("copy"));
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("a.rs"), "// a").unwrap();
        std::fs::write(from.join("b.rs"), "// b").unwrap();
        let mut mirror = Mirror::new(from.clone(), to.clone());
        mirror.sync().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let launched = SystemTime::now();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(from.join("a.rs"), "// a, edited").unwrap();
        mirror.sync().unwrap();
        assert_eq!(mirror.retouch_since(launched).unwrap(), 1);
        assert_eq!(
            std::fs::read_to_string(to.join("a.rs")).unwrap(),
            "// a, edited"
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn half_written_archives_are_cleared() {
        let target = std::env::temp_dir().join(format!("frus-hot-archives-{}", std::process::id()));
        let dev = target.join("x86_64-pc-windows-msvc").join("desktop-dev");
        std::fs::create_dir_all(&dev).unwrap();
        std::fs::write(dev.join("libdeps-064f36d8.a"), b"half").unwrap();
        std::fs::write(dev.join("counter.exe"), b"keep").unwrap();
        clear_dependency_archives(&target);
        assert!(!dev.join("libdeps-064f36d8.a").exists());
        assert!(dev.join("counter.exe").exists());
        std::fs::remove_dir_all(&target).unwrap();
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
