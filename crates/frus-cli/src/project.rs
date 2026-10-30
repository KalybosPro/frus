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
    /// The devices the project describes, each a table of `[[package.metadata.frus.devices]]`
    /// written as `key=value;…`, the way the shell reads one (milestone 727).
    pub devices: Vec<String>,
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
        let mut message = format!(
            "no application here: {} is not inside a frus application (a Cargo.toml with a \
             [package])",
            start.display()
        );
        let below = applications_below(start);
        if let Some(first) = below.first() {
            message.push_str("\n\nThe applications below it:\n");
            for app in below.iter().take(12) {
                message.push_str(&format!("  {}\n", app.display()));
            }
            if below.len() > 12 {
                message.push_str(&format!("  and {} more\n", below.len() - 12));
            }
            message.push_str(&format!(
                "\nRun `frus run` in one of them: cd {}",
                first.display()
            ));
        }
        Err(message)
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
        let mut devices: Vec<Vec<String>> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = header.trim().to_string();
                // `[[…]]`, an array of tables: each header starts another.
                if section == DEVICES {
                    devices.push(Vec::new());
                }
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
                (DEVICES, _) => {
                    if let (Some(device), Some(value)) = (devices.last_mut(), spec_value(value)) {
                        device.push(format!("{key}={value}"));
                    }
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
            devices: devices.into_iter().map(|pairs| pairs.join(";")).collect(),
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

/// The header of a project's own devices, as `from_manifest` sees `[[package.metadata.frus.devices]]`.
const DEVICES: &str = "[package.metadata.frus.devices]";

/// A device key's value as a description holds it: a string's text, a list's items joined by
/// commas, a number as written. `;` and `=` would end the pair early, so they are left out.
fn spec_value(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let value = if raw.starts_with('"') {
        string_value(raw)?
    } else if let Some(list) = raw.strip_prefix('[') {
        let list = list.split(']').next()?;
        list.split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(",")
    } else {
        raw.split('#').next()?.trim().to_string()
    };
    Some(value.replace([';', '='], " "))
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

/// The frus applications in the directories below `start`, a few levels down, as paths relative
/// to it: what to suggest to someone who ran `frus run` at the top of a workspace.
pub(crate) fn applications_below(start: &Path) -> Vec<PathBuf> {
    const SKIP: [&str; 8] = [
        "target",
        "node_modules",
        ".git",
        "src",
        "web",
        "res",
        "assets",
        "docs",
    ];
    let mut found = Vec::new();
    let mut pending = vec![(start.to_path_buf(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !path.is_dir() || name.starts_with('.') || SKIP.contains(&name.as_str()) {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(path.join("Cargo.toml")) {
                if is_frus_application(&path, &text) {
                    found.push(path.strip_prefix(start).unwrap_or(&path).to_path_buf());
                }
            }
            if depth < 3 {
                pending.push((path, depth + 1));
            }
        }
    }
    found.sort();
    found
}

/// A package that runs: it depends on frus and has a library or a program.
fn is_frus_application(root: &Path, text: &str) -> bool {
    let depends = text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("frus =") || line.starts_with("frus.") || line.starts_with("frus=")
    });
    depends && Project::from_manifest(root, text).is_some()
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

[[package.metadata.frus.devices]]
id = "pixel-9"
name = "Pixel 9"   # as it is called
platform = "android"
width = 412
height = 924
ratio = 2.625
safe-area = [52, 0, 24, 0]

[[package.metadata.frus.devices]]
id = "kiosk"
platform = "ios"
width = 800
height = 1280
ratio = 2  # the second table
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

    /// **A project's own devices are read**, one description a table, as the shell reads them:
    /// strings as their text, lists joined by commas, comments left out (milestone 727).
    #[test]
    fn the_project_s_own_devices_are_read() {
        let project = Project::from_manifest(Path::new("app"), MANIFEST).unwrap();
        assert_eq!(
            project.devices,
            [
                "id=pixel-9;name=Pixel 9;platform=android;width=412;height=924;ratio=2.625;\
                 safe-area=52,0,24,0",
                "id=kiosk;platform=ios;width=800;height=1280;ratio=2",
            ]
        );
        // And they are the shell's to read: the core takes them whole.
        for spec in &project.devices {
            frus_core::DeviceProfile::parse(spec).unwrap();
        }
        assert_eq!(
            spec_value(r#""a;b=c""#).as_deref(),
            Some("a b c"),
            "nothing ends a pair early"
        );
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
    fn a_workspace_root_points_at_its_applications() {
        let root = std::env::temp_dir().join(format!("frus-find-{}", std::process::id()));
        let app = root.join("examples").join("counter");
        let library = root.join("crates").join("widgets");
        let built = root.join("target").join("copy");
        for dir in [&app, &library, &built] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        let manifest =
            "[package]\nname = \"counter\"\n\n[dependencies]\nfrus = { path = \"../..\" }\n";
        std::fs::write(app.join("Cargo.toml"), manifest).unwrap();
        std::fs::write(built.join("Cargo.toml"), manifest).unwrap();
        std::fs::write(
            library.join("Cargo.toml"),
            "[package]\nname = \"widgets\"\n",
        )
        .unwrap();

        assert_eq!(
            applications_below(&root),
            [Path::new("examples").join("counter")]
        );
        let error = Project::find(&root).unwrap_err();
        assert!(error.contains("The applications below it"), "{error}");
        assert!(error.contains("cd examples"), "{error}");
        std::fs::remove_dir_all(&root).unwrap();
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
