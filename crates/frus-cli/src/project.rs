//! The application `frus run` and `frus build` work on: found from the current directory, and
//! read from its manifest.
//!
//! The manifest is read line by line, not parsed as TOML: the tool has no dependencies, and the
//! handful of keys it needs are always written one per line, the way `frus create` writes them.

use std::path::{Path, PathBuf};
use std::process::Command;

/// An application's package, as far as running and building it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Project {
    /// The directory holding its `Cargo.toml`.
    pub root: PathBuf,
    /// `[package] name`.
    pub package: String,
    /// The library crate's name, which is what the `.wasm`, the `.so` and the logcat tag are
    /// called: `[lib] name`, or the package's with `-` made `_`.
    pub lib: String,
    /// Whether the package has a library at all — Android and the web load the library.
    pub has_lib: bool,
    /// The Android application id: `[package.metadata.android] package`, or cargo-apk's default.
    pub android_package: String,
    /// Whether the manifest declares the `web-release` profile `frus create` writes.
    pub has_web_release: bool,
    /// The Rust targets the APK is built for: `[package.metadata.android] build_targets`.
    pub android_targets: Vec<String>,
    /// The Android API the APK targets: `[package.metadata.android.sdk] target_sdk_version`.
    pub android_target_sdk: u32,
}

impl Project {
    /// The application whose manifest is in `start` or the nearest directory above it.
    pub(crate) fn find(start: &Path) -> Result<Self, String> {
        let mut dir = Some(start);
        while let Some(here) = dir {
            let manifest = here.join("Cargo.toml");
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if let Some(project) = Self::from_manifest(here, &text) {
                    return Ok(project);
                }
            }
            dir = here.parent();
        }
        Err(format!(
            "no application here: there is no Cargo.toml with a [package] in {} or above it",
            start.display()
        ))
    }

    /// The application `text` describes, or `None` when it has no `[package]` (a workspace root).
    pub(crate) fn from_manifest(root: &Path, text: &str) -> Option<Self> {
        let mut section = String::new();
        let mut package = None;
        let mut lib_name = None;
        let mut has_lib_section = false;
        let mut android_package = None;
        let mut has_web_release = false;
        let mut android_targets = None;
        let mut android_target_sdk = None;
        for line in text.lines() {
            let line = line.trim();
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = header.trim().to_string();
                match section.as_str() {
                    "lib" => has_lib_section = true,
                    "profile.web-release" => has_web_release = true,
                    _ => {}
                }
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            match (section.as_str(), key) {
                ("package.metadata.android", "build_targets") => {
                    android_targets = Some(string_list(value));
                }
                ("package.metadata.android.sdk", "target_sdk_version") => {
                    android_target_sdk = value.split('#').next().unwrap_or("").trim().parse().ok();
                }
                _ => {}
            }
            let value = string_value(value);
            match (section.as_str(), key, value) {
                ("package", "name", Some(value)) => package = Some(value),
                ("lib", "name", Some(value)) => lib_name = Some(value),
                ("package.metadata.android", "package", Some(value)) => {
                    android_package = Some(value)
                }
                _ => {}
            }
        }
        let package = package?;
        let lib = lib_name.unwrap_or_else(|| package.replace('-', "_"));
        Some(Project {
            root: root.to_path_buf(),
            has_lib: has_lib_section || root.join("src").join("lib.rs").is_file(),
            // cargo-apk's own default when the manifest names none.
            android_package: android_package.unwrap_or_else(|| format!("rust.{lib}")),
            package,
            lib,
            has_web_release,
            // cargo-apk's defaults, which `frus create` writes out explicitly anyway.
            android_targets: android_targets
                .filter(|targets: &Vec<String>| !targets.is_empty())
                .unwrap_or_else(|| vec!["aarch64-linux-android".to_string()]),
            android_target_sdk: android_target_sdk.unwrap_or(34),
        })
    }

    /// The directory the web build is served from: `web/`, with its `index.html`.
    pub(crate) fn web_dir(&self) -> PathBuf {
        self.root.join("web")
    }

    /// Where cargo writes this package's builds, as `cargo metadata` says — a workspace's
    /// `target/`, or wherever `CARGO_TARGET_DIR` points.
    pub(crate) fn target_dir(&self) -> Result<PathBuf, String> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(&self.root)
            .output()
            .map_err(|e| format!("could not run cargo: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let json = String::from_utf8_lossy(&output.stdout);
        json_string(&json, "target_directory")
            .map(PathBuf::from)
            .ok_or_else(|| "cargo metadata did not say where the target directory is".into())
    }

    /// The version of `wasm-bindgen` the lockfile holds, which the `wasm-bindgen` program must
    /// match exactly.
    pub(crate) fn locked_wasm_bindgen(&self) -> Option<String> {
        let mut dir = Some(self.root.as_path());
        while let Some(here) = dir {
            if let Ok(lock) = std::fs::read_to_string(here.join("Cargo.lock")) {
                return locked_version(&lock, "wasm-bindgen");
            }
            dir = here.parent();
        }
        None
    }
}

/// `["a", "b"]` → `[a, b]`, on one line.
fn string_list(raw: &str) -> Vec<String> {
    raw.trim()
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or("")
        .split(',')
        .filter_map(string_value)
        .collect()
}

/// `"value"` → `value`; anything else (a table, a number) is not a string.
fn string_value(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw.split(" #").next().unwrap_or(raw).trim();
    raw.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .map(str::to_string)
}

/// The version a `Cargo.lock` holds for `name`.
pub(crate) fn locked_version(lock: &str, name: &str) -> Option<String> {
    let wanted = format!("name = \"{name}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == wanted {
            return lines.next().and_then(|next| {
                next.trim()
                    .strip_prefix("version = ")
                    .and_then(string_value)
            });
        }
    }
    None
}

/// The string value of the first `"key":"..."` in a JSON text, unescaped. Enough for cargo's
/// machine output, whose strings are paths and names.
pub(crate) fn json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let mut out = String::new();
    let mut chars = json[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
[package]
name = "my-app"
version = "0.1.0"

[lib]
crate-type = ["rlib", "cdylib"]

[dependencies]
frus = "0.3"
name = "not the package"

[package.metadata.android]
package = "com.acme.my_app"  # settle it before the first release
build_targets = ["aarch64-linux-android", "x86_64-linux-android"]

[package.metadata.android.sdk]
min_sdk_version = 24
target_sdk_version = 33  # the store's minimum

[profile.web-release]
inherits = "release"
"#;

    #[test]
    fn the_manifest_names_what_running_needs() {
        let project = Project::from_manifest(Path::new("app"), MANIFEST).unwrap();
        assert_eq!(project.package, "my-app");
        assert_eq!(project.lib, "my_app");
        assert!(project.has_lib);
        assert_eq!(project.android_package, "com.acme.my_app");
        assert!(project.has_web_release);
        assert_eq!(
            project.android_targets,
            ["aarch64-linux-android", "x86_64-linux-android"]
        );
        assert_eq!(project.android_target_sdk, 33);
    }

    #[test]
    fn defaults_are_cargos() {
        let text = "[package]\nname = \"hello\"\n\n[lib]\nname = \"hello_lib\"\n";
        let project = Project::from_manifest(Path::new("."), text).unwrap();
        assert_eq!(project.lib, "hello_lib");
        assert_eq!(project.android_package, "rust.hello_lib");
        assert_eq!(project.android_targets, ["aarch64-linux-android"]);
        assert_eq!(project.android_target_sdk, 34);
        assert!(!project.has_web_release);
    }

    #[test]
    fn a_workspace_root_is_not_an_application() {
        assert_eq!(
            Project::from_manifest(Path::new("."), "[workspace]\nmembers = [\"a\"]\n"),
            None
        );
    }

    #[test]
    fn the_lockfile_says_which_wasm_bindgen() {
        let lock =
            "[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.127\"\nsource = \"x\"\n\n\
                    [[package]]\nname = \"wasm-bindgen-futures\"\nversion = \"0.4.1\"\n";
        assert_eq!(
            locked_version(lock, "wasm-bindgen").as_deref(),
            Some("0.2.127")
        );
        assert_eq!(locked_version(lock, "nothing"), None);
    }

    #[test]
    fn json_strings_come_back_unescaped() {
        let json = r#"{"packages":[],"target_directory":"D:\\src\\app\\target","x":"\"q\""}"#;
        assert_eq!(
            json_string(json, "target_directory").as_deref(),
            Some(r"D:\src\app\target")
        );
        assert_eq!(json_string(json, "x").as_deref(), Some("\"q\""));
        assert_eq!(json_string(json, "missing"), None);
    }
}
