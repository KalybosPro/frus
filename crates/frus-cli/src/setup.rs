//! What a command needs on this machine, found — or installed, then found.
//!
//! A developer who runs `frus run -d android` on a machine that has never built for Android
//! should get the application on their phone, not a list of things to install first. So each
//! command asks here for what it needs, and what is missing is fetched with the tools every
//! platform already has: `rustup` and `cargo install` for Rust's part, and `curl` with the
//! system's archive tool for the Android SDK, the NDK and a JDK — into `~/.frus/` when there is
//! no SDK on the machine yet, and into the existing SDK when there is one.
//!
//! Each download says what it is and roughly how big before it starts.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cargo;

/// The version of Google's command-line tools fetched when there is no SDK at all.
const CMDLINE_TOOLS: &str = "11076708";
/// The NDK installed when the SDK has none (the one frus is built and tested with).
const NDK: &str = "27.0.12077973";
/// The JDK `sdkmanager` runs on.
const JDK_MAJOR: &str = "17";

/// frus's own directory: `~/.frus`, or `FRUS_HOME`.
pub(crate) fn frus_home() -> PathBuf {
    if let Some(home) = std::env::var_os("FRUS_HOME") {
        return PathBuf::from(home);
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".frus")
}

// --- Rust ---

/// The compiler can build for `target` — installed with rustup if it cannot yet.
pub(crate) fn rust_target(target: &str) -> Result<(), String> {
    let installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == target)
        });
    match installed {
        Ok(true) => Ok(()),
        Ok(false) => {
            crate::report::say(&format!(
                "Installing Rust's {target} target (rustup, about 30 MB)…"
            ));
            cargo::run(Command::new("rustup").args(["target", "add", target]))
        }
        // No rustup (a distribution's packaged Rust): let the build say what is missing.
        Err(_) => Ok(()),
    }
}

/// The program `program`, ready to run: the one on the `PATH` when it will do, or one
/// `cargo install` puts in place.
///
/// A tool pinned to a `version` (wasm-bindgen must match the library exactly) that is installed
/// at another version is **not replaced**: other projects on the machine may need the one that
/// is there. The pinned version goes in a directory of its own, `~/.frus/tools/<crate>-<version>`,
/// and that copy is the one used for this project. `probe` is an argument the program answers
/// successfully (`--version`; cargo-apk has only `--help`).
pub(crate) fn cargo_tool(
    program: &str,
    probe: &str,
    krate: &str,
    version: Option<&str>,
) -> Result<PathBuf, String> {
    let answers = |exe: &Path| {
        Command::new(exe)
            .arg(probe)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let on_path = PathBuf::from(program);
    let found = answers(&on_path);
    let Some(version) = version else {
        if found.is_some() {
            return Ok(on_path);
        }
        crate::report::say(&format!(
            "Installing {krate} (cargo install; it compiles, a few minutes)…"
        ));
        cargo::run(cargo_install(krate).arg("--locked"))?;
        return Ok(on_path);
    };
    if found
        .as_deref()
        .is_some_and(|text| text.split_whitespace().any(|word| word == version))
    {
        return Ok(on_path);
    }
    let private = frus_home().join("tools").join(format!("{krate}-{version}"));
    let exe = private.join("bin").join(if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_string()
    });
    if answers(&exe).is_some() {
        return Ok(exe);
    }
    crate::report::say(&format!(
        "This application needs {krate} {version}{}: installing it for frus in {} \
         (cargo install; it compiles, a few minutes the first time)…",
        match found.as_deref().and_then(|t| t.split_whitespace().nth(1)) {
            Some(other) => format!(" (the one on the PATH is {other}, and is left alone)"),
            None => String::new(),
        },
        private.display()
    ));
    cargo::run(
        cargo_install(krate)
            .args(["--locked", "--version", version, "--root"])
            .arg(&private),
    )?;
    Ok(exe)
}

/// `cargo install <krate>`, compiled in a directory of its own rather than in whatever
/// `CARGO_TARGET_DIR` the developer's project uses.
fn cargo_install(krate: &str) -> Command {
    let mut command = Command::new("cargo");
    command
        .args(["install", krate])
        .env_remove("CARGO_TARGET_DIR");
    command
}

// --- Android ---

/// The SDK this machine already has, wherever it is — without installing anything. What
/// `frus devices` reads, so listing devices never downloads a gigabyte.
pub(crate) fn existing_sdk() -> Option<PathBuf> {
    sdk_candidates().into_iter().find(|dir| dir.is_dir())
}

/// Where an SDK is looked for: `FRUS_ANDROID_SDK`, `ANDROID_HOME`, `ANDROID_SDK_ROOT`, Android
/// Studio's default for this system, then frus's own.
fn sdk_candidates() -> Vec<PathBuf> {
    if let Some(forced) = std::env::var_os("FRUS_ANDROID_SDK") {
        return vec![PathBuf::from(forced)];
    }
    let mut candidates: Vec<PathBuf> = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    let studio = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Android").join("Sdk"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|d| PathBuf::from(d).join("Library/Android/sdk"))
    } else {
        std::env::var_os("HOME").map(|d| PathBuf::from(d).join("Android/Sdk"))
    };
    candidates.extend(studio);
    candidates.push(frus_home().join("android-sdk"));
    candidates
}

/// An SDK ready to build with: found, completed with what it lacks, or installed from nothing.
/// The variables cargo-apk reads are set for the rest of this process.
pub(crate) fn android_sdk(target_sdk: u32) -> Result<PathBuf, String> {
    let home = match existing_sdk() {
        Some(home) => home,
        None => {
            let home = sdk_candidates()
                .pop()
                .unwrap_or_else(|| frus_home().join("android-sdk"));
            install_command_line_tools(&home)?;
            home
        }
    };

    let mut missing = Vec::new();
    if !home.join("platform-tools").is_dir() {
        missing.push("platform-tools".to_string());
    }
    let platform = format!("platforms;android-{target_sdk}");
    if !home
        .join("platforms")
        .join(format!("android-{target_sdk}"))
        .is_dir()
    {
        missing.push(platform);
    }
    if newest_version_dir(&home.join("build-tools")).is_none() {
        missing.push(format!("build-tools;{target_sdk}.0.0"));
    }
    let ndk_dir = std::env::var_os("ANDROID_NDK_ROOT")
        .or_else(|| std::env::var_os("ANDROID_NDK_HOME"))
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
        .or_else(|| newest_version_dir(&home.join("ndk")));
    if ndk_dir.is_none() {
        missing.push(format!("ndk;{NDK}"));
    }
    if !missing.is_empty() {
        sdkmanager(&home, &missing)?;
    }
    let ndk = ndk_dir
        .or_else(|| newest_version_dir(&home.join("ndk")))
        .ok_or("the NDK was installed but cannot be found in the SDK")?;

    // cargo-apk and adb read these; set once, every command started from here on sees them.
    std::env::set_var("ANDROID_HOME", &home);
    std::env::set_var("ANDROID_NDK_ROOT", &ndk);
    Ok(home)
}

/// The system image a new emulator gets when none is installed: a recent Android with Google's
/// APIs, for this computer's processor.
fn default_system_image() -> String {
    let abi = if cfg!(target_arch = "aarch64") {
        "arm64-v8a"
    } else {
        "x86_64"
    };
    format!("system-images;android-35;google_apis;{abi}")
}

/// The newest system image installed in the SDK at `home`, as sdkmanager names it:
/// `system-images;android-36.1;google_apis_playstore;x86_64`.
pub(crate) fn installed_system_image(home: &Path) -> Option<String> {
    let images = home.join("system-images");
    let mut found = Vec::new();
    for api in std::fs::read_dir(&images).ok()?.filter_map(Result::ok) {
        for tag in std::fs::read_dir(api.path())
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
        {
            for abi in std::fs::read_dir(tag.path())
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
            {
                if abi.path().join("system.img").is_file() || abi.path().is_dir() {
                    let name = |e: &std::fs::DirEntry| e.file_name().to_string_lossy().into_owned();
                    found.push((name(&api), name(&tag), name(&abi)));
                }
            }
        }
    }
    found
        .into_iter()
        .max_by_key(|(api, tag, _)| {
            (
                version_key(api.trim_start_matches("android-")),
                // Google Play last among equals: the plain Google APIs image is lighter.
                tag != "google_apis_playstore",
            )
        })
        .map(|(api, tag, abi)| format!("system-images;{api};{tag};{abi}"))
}

/// A free emulator name: `wanted`, or `frus_emulator`, with a number added if it is taken.
pub(crate) fn free_avd_name(wanted: Option<&str>, taken: &[String]) -> String {
    let base = wanted
        .map(|w| w.trim().replace(' ', "_"))
        .filter(|w| !w.is_empty())
        .unwrap_or_else(|| "frus_emulator".to_string());
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|name| !taken.contains(name))
        .unwrap_or(base)
}

/// Creates an Android emulator (an AVD) and gives its name. What it needs is installed first:
/// the SDK's tools, the emulator, a system image if none is there.
pub(crate) fn create_avd(wanted: Option<&str>, taken: &[String]) -> Result<String, String> {
    let home = match existing_sdk() {
        Some(home) => home,
        None => {
            let home = sdk_candidates()
                .pop()
                .unwrap_or_else(|| frus_home().join("android-sdk"));
            install_command_line_tools(&home)?;
            home
        }
    };
    let mut missing = Vec::new();
    if !home.join("emulator").is_dir() {
        missing.push("emulator".to_string());
    }
    if !home.join("platform-tools").is_dir() {
        missing.push("platform-tools".to_string());
    }
    let image = match installed_system_image(&home) {
        Some(image) => image,
        None => {
            let image = default_system_image();
            missing.push(image.clone());
            image
        }
    };
    if !missing.is_empty() {
        sdkmanager(&home, &missing)?;
    }
    let script = if cfg!(windows) {
        "avdmanager.bat"
    } else {
        "avdmanager"
    };
    let avdmanager = home
        .join("cmdline-tools")
        .join("latest")
        .join("bin")
        .join(script);
    if !avdmanager.is_file() {
        install_command_line_tools(&home)?;
    }
    let name = free_avd_name(wanted, taken);
    crate::report::say(&format!("Creating the {name} emulator ({image})…"));
    let mut child = Command::new(&avdmanager)
        .args([
            "create",
            "avd",
            "--name",
            &name,
            "--package",
            &image,
            "--device",
            "pixel",
        ])
        .env("JAVA_HOME", java()?)
        .env("ANDROID_HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run avdmanager: {e}"))?;
    // It asks whether to make a custom hardware profile.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = std::io::Write::write_all(&mut stdin, b"no\n");
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        give_memory(&name);
        Ok(name)
    } else {
        Err(format!(
            "avdmanager could not create the emulator: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// The memory a new emulator gets, in megabytes.
///
/// avdmanager's profile gives 2 GB, and a recent Android with Google's services does not run in
/// it: on Android 16 with Google Play, the system interface stopped answering ("System UI isn't
/// responding") with 74 MB free, before any application was installed. 4 GB leaves it half free.
const EMULATOR_RAM_MB: u32 = 4096;

/// Gives the emulator `name` [`EMULATOR_RAM_MB`] of memory, in its `config.ini`. Best effort: an
/// emulator whose files are not where avdmanager puts them keeps avdmanager's figure.
fn give_memory(name: &str) {
    let Some(config) = avd_dir(name).map(|dir| dir.join("config.ini")) else {
        return;
    };
    if let Ok(text) = std::fs::read_to_string(&config) {
        // In megabytes, a bare number: the emulator reads `4G` as nothing and starts with its
        // minimum, 2048 MB (emulator 36.4).
        let changed = with_setting(&text, "hw.ramSize", &EMULATOR_RAM_MB.to_string());
        if std::fs::write(&config, changed).is_ok() {
            crate::report::say(&format!(
                "Gave the emulator {} GB of memory.",
                EMULATOR_RAM_MB / 1024
            ));
        }
    }
}

/// Where avdmanager keeps the emulators: `ANDROID_AVD_HOME`, else `ANDROID_USER_HOME/avd`, else
/// `.android/avd` in the home folder.
fn avd_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("ANDROID_AVD_HOME") {
        return Some(PathBuf::from(home));
    }
    if let Some(user) = std::env::var_os("ANDROID_USER_HOME") {
        return Some(PathBuf::from(user).join("avd"));
    }
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(|home| PathBuf::from(home).join(".android").join("avd"))
}

/// The folder of the emulator `name`: the `path=` its `<name>.ini` names, else `<name>.avd`
/// beside it.
fn avd_dir(name: &str) -> Option<PathBuf> {
    let home = avd_home()?;
    let named = std::fs::read_to_string(home.join(format!("{name}.ini")))
        .ok()
        .and_then(|ini| {
            ini.lines()
                .find_map(|line| line.trim().strip_prefix("path="))
                .map(|path| PathBuf::from(path.trim()))
        });
    Some(named.unwrap_or_else(|| home.join(format!("{name}.avd"))))
}

/// `text`, an emulator's `config.ini`, with `key` set to `value`: its line rewritten where it is
/// one, added at the end where it is not. Every other line, and the line endings, kept.
pub(crate) fn with_setting(text: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| match line.split_once('=') {
            Some((k, _)) if k.trim() == key => {
                found = true;
                format!("{key}={value}")
            }
            _ => line.to_string(),
        })
        .collect();
    if !found {
        lines.push(format!("{key}={value}"));
    }
    let mut out = lines.join(newline);
    out.push_str(newline);
    out
}

/// [`newest_version_dir`], for the doctor.
pub(crate) fn newest_in(dir: &Path) -> Option<PathBuf> {
    newest_version_dir(dir)
}

/// A JDK already here, without installing one: `JAVA_HOME`, then frus's own.
pub(crate) fn find_java() -> Option<PathBuf> {
    let java_exe = if cfg!(windows) { "java.exe" } else { "java" };
    if let Some(home) = std::env::var_os("JAVA_HOME").map(PathBuf::from) {
        if home.join("bin").join(java_exe).is_file() {
            return Some(home);
        }
    }
    jdk_in(&frus_home().join("jdk"), java_exe)
}

/// The highest-versioned directory in `dir` (`ndk/27.0.12077973`, `build-tools/35.0.0`).
fn newest_version_dir(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .max_by_key(|p| version_key(p.file_name().and_then(|n| n.to_str()).unwrap_or("")))
}

/// `27.0.12077973` → `[27, 0, 12077973]`, so `10.0.0` sorts after `9.0.0`.
fn version_key(name: &str) -> Vec<u64> {
    name.split(['.', '-'])
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// Google's command-line tools (`sdkmanager`), into `home/cmdline-tools/latest`.
fn install_command_line_tools(home: &Path) -> Result<(), String> {
    let os = if cfg!(windows) {
        "win"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    crate::report::say(&format!(
        "No Android SDK on this machine: installing one in {} (the command-line tools, about \
         150 MB, then the parts frus needs).",
        home.display()
    ));
    let url = format!(
        "https://dl.google.com/android/repository/commandlinetools-{os}-{CMDLINE_TOOLS}_latest.zip"
    );
    let staging = home.join(".download");
    let archive = staging.join("cmdline-tools.zip");
    download(&url, &archive)?;
    let unpacked = staging.join("unpacked");
    let _ = std::fs::remove_dir_all(&unpacked);
    extract(&archive, &unpacked)?;
    let latest = home.join("cmdline-tools").join("latest");
    std::fs::create_dir_all(latest.parent().unwrap_or(home)).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&latest);
    std::fs::rename(unpacked.join("cmdline-tools"), &latest)
        .map_err(|e| format!("could not put the command-line tools in place: {e}"))?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(())
}

/// Installs SDK packages, accepting their licences on the developer's behalf (what Android
/// Studio's first-run wizard does with one click).
fn sdkmanager(home: &Path, packages: &[String]) -> Result<(), String> {
    let script = if cfg!(windows) {
        "sdkmanager.bat"
    } else {
        "sdkmanager"
    };
    let tool = home
        .join("cmdline-tools")
        .join("latest")
        .join("bin")
        .join(script);
    if !tool.is_file() {
        install_command_line_tools(home)?;
    }
    let java_home = java()?;
    crate::report::say(&format!(
        "Installing Android {} (sdkmanager; the NDK alone is about 1 GB)…",
        packages.join(", ")
    ));
    let run = |args: &[String], quiet: bool| -> Result<(), String> {
        let mut command = Command::new(&tool);
        command
            .arg(format!("--sdk_root={}", home.display()))
            .args(args)
            .env("JAVA_HOME", &java_home)
            .stdin(Stdio::piped());
        if quiet {
            // The licences' full texts: pages nobody reads in a terminal.
            command.stdout(Stdio::null());
        } else if crate::report::is_machine() {
            // An editor reads standard output as JSON: the progress goes to standard error.
            command.stdout(std::io::stderr());
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("could not run sdkmanager: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            // Every licence prompt answered yes.
            let _ = stdin.write_all("y\n".repeat(50).as_bytes());
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("sdkmanager failed ({status})"))
        }
    };
    crate::report::say("Accepting the Android SDK's licences (sdkmanager --licenses)…");
    run(&["--licenses".to_string()], true)?;
    run(packages, false)
}

/// A `JAVA_HOME` for sdkmanager (and the hot-reload engine's Gradle project): the machine's, or
/// a JDK fetched into `~/.frus/jdk`.
pub(crate) fn java() -> Result<PathBuf, String> {
    let java_exe = if cfg!(windows) { "java.exe" } else { "java" };
    if let Some(home) = std::env::var_os("JAVA_HOME").map(PathBuf::from) {
        if home.join("bin").join(java_exe).is_file() {
            return Ok(home);
        }
    }
    let ours = frus_home().join("jdk");
    if let Some(home) = jdk_in(&ours, java_exe) {
        return Ok(home);
    }
    let (os, ext) = if cfg!(windows) {
        ("windows", "zip")
    } else if cfg!(target_os = "macos") {
        ("mac", "tar.gz")
    } else {
        ("linux", "tar.gz")
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    };
    crate::report::say(&format!(
        "The Android SDK's tools run on Java, and there is none here: installing a JDK {JDK_MAJOR} \
         in {} (about 190 MB)…",
        ours.display()
    ));
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{JDK_MAJOR}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );
    let archive = frus_home().join(format!("jdk.{ext}"));
    download(&url, &archive)?;
    let _ = std::fs::remove_dir_all(&ours);
    extract(&archive, &ours)?;
    let _ = std::fs::remove_file(&archive);
    jdk_in(&ours, java_exe).ok_or_else(|| "the JDK was unpacked but has no java in it".into())
}

/// The JDK unpacked somewhere under `dir` (`jdk-17…/`, or `…/Contents/Home` on a Mac).
fn jdk_in(dir: &Path, java_exe: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find_map(|top| {
            [top.clone(), top.join("Contents").join("Home")]
                .into_iter()
                .find(|home| home.join("bin").join(java_exe).is_file())
        })
}

// --- DXC, DirectX 12's fast shader compiler (Windows) ---

/// The DXC release frus fetches: `dxcompiler.dll` and `dxil.dll`, which DirectX 12 compiles a
/// program's shaders with in a fraction of the time the system's own compiler (FXC) takes
/// (milestone 684, #217).
const DXC_VERSION: &str = "1.9.2609";
const DXC_URL: &str =
    "https://github.com/microsoft/DirectXShaderCompiler/releases/download/v1.9.2609/dxc_2026_09_29.zip";
/// The two files a program loads, beside it.
pub(crate) const DXC_FILES: [&str; 2] = ["dxcompiler.dll", "dxil.dll"];

/// The folder holding DXC's DLLs for this computer's processor, fetched into `~/.frus/tools`
/// the first time (about 32 MB to download).
pub(crate) fn dxc_dlls() -> Result<PathBuf, String> {
    let root = frus_home().join("tools").join(format!("dxc-{DXC_VERSION}"));
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    let dir = root.join("bin").join(arch);
    if DXC_FILES.iter().all(|f| dir.join(f).is_file()) {
        return Ok(dir);
    }
    crate::report::say(&format!(
        "Fetching DXC {DXC_VERSION}, the shader compiler that makes a program start faster on \
         Windows (about 32 MB, once)…"
    ));
    let archive = frus_home().join("tools").join("dxc.zip");
    download(DXC_URL, &archive)?;
    let _ = std::fs::remove_dir_all(&root);
    extract(&archive, &root)?;
    let _ = std::fs::remove_file(&archive);
    if DXC_FILES.iter().all(|f| dir.join(f).is_file()) {
        Ok(dir)
    } else {
        Err(format!(
            "DXC was unpacked but {} lacks its DLLs",
            dir.display()
        ))
    }
}

/// DXC's DLLs beside a Windows program in `dir`, where DirectX 12 finds them; copied only when
/// missing or different. A failure is said, not fatal: the program starts anyway, with the
/// slower compiler.
pub(crate) fn place_dxc(dir: &Path) {
    let source = match dxc_dlls() {
        Ok(source) => source,
        Err(error) => {
            crate::report::warn(&format!(
                "{error}: the program uses Windows' own shader compiler, and starts slower"
            ));
            return;
        }
    };
    for file in DXC_FILES {
        let (from, to) = (source.join(file), dir.join(file));
        let same = |a: &Path, b: &Path| match (std::fs::metadata(a), std::fs::metadata(b)) {
            (Ok(a), Ok(b)) => a.len() == b.len(),
            _ => false,
        };
        if !same(&from, &to) {
            if let Err(error) = std::fs::copy(&from, &to) {
                crate::report::warn(&format!("could not place {file}: {error}"));
            }
        }
    }
}

// --- Fetching ---

/// `url` to `file`, with curl's progress bar — curl ships with Windows 10 and later, macOS and
/// every Linux.
fn download(url: &str, file: &Path) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    cargo::run(
        Command::new("curl")
            .args(["--fail", "--location", "--progress-bar", "--output"])
            .arg(file)
            .arg(url),
    )
    .map_err(|e| format!("could not download {url}: {e}"))
}

/// Unpacks a `.zip` or `.tar.gz` into `dir`, with the system's own tool.
fn extract(archive: &Path, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let zip = archive.extension().is_some_and(|e| e == "zip");
    let mut command = if cfg!(windows) {
        // Windows' own tar (bsdtar) reads zips; a GNU tar earlier on the PATH would not.
        let system = std::env::var_os("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32").join("tar.exe"))
            .filter(|tar| tar.is_file())
            .unwrap_or_else(|| PathBuf::from("tar"));
        let mut tar = Command::new(system);
        tar.arg("-xf").arg(archive).arg("-C").arg(dir);
        tar
    } else if zip {
        let mut unzip = Command::new("unzip");
        unzip.arg("-q").arg("-o").arg(archive).arg("-d").arg(dir);
        unzip
    } else {
        let mut tar = Command::new("tar");
        tar.arg("-xzf").arg(archive).arg("-C").arg(dir);
        tar
    };
    cargo::run(&mut command).map_err(|e| format!("could not unpack {}: {e}", archive.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_is_rewritten_where_it_is_and_added_where_it_is_not() {
        let config = "avd.ini.encoding=UTF-8\nhw.ramSize=2G\nhw.lcd.density=420\n";
        assert_eq!(
            with_setting(config, "hw.ramSize", "4096"),
            "avd.ini.encoding=UTF-8\nhw.ramSize=4096\nhw.lcd.density=420\n"
        );
        assert_eq!(
            with_setting("hw.lcd.density=420\n", "hw.ramSize", "4096"),
            "hw.lcd.density=420\nhw.ramSize=4096\n"
        );
        // Spaces around `=`, and Windows line endings, as an edited file may have them.
        assert_eq!(
            with_setting(
                "hw.ramSize = 2048\r\nvm.heapSize=512\r\n",
                "hw.ramSize",
                "4096"
            ),
            "hw.ramSize=4096\r\nvm.heapSize=512\r\n"
        );
        // A key that merely starts the same is another key.
        assert_eq!(
            with_setting("hw.ramSizeMax=1\n", "hw.ramSize", "4096"),
            "hw.ramSizeMax=1\nhw.ramSize=4096\n"
        );
    }

    #[test]
    fn a_new_emulator_gets_a_free_name() {
        assert_eq!(free_avd_name(None, &[]), "frus_emulator");
        let taken = ["frus_emulator".to_string(), "frus_emulator_2".to_string()];
        assert_eq!(free_avd_name(None, &taken), "frus_emulator_3");
        assert_eq!(free_avd_name(Some("My Phone"), &taken), "My_Phone");
        assert_eq!(free_avd_name(Some("  "), &[]), "frus_emulator");
    }

    #[test]
    fn the_newest_system_image_is_chosen() {
        let home = std::env::temp_dir().join(format!("frus-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        for (api, tag) in [
            ("android-34", "google_apis"),
            ("android-36.1", "google_apis_playstore"),
            ("android-36.1", "google_apis"),
            ("android-9", "default"),
        ] {
            std::fs::create_dir_all(
                home.join("system-images")
                    .join(api)
                    .join(tag)
                    .join("x86_64"),
            )
            .unwrap();
        }
        assert_eq!(
            installed_system_image(&home).as_deref(),
            Some("system-images;android-36.1;google_apis;x86_64")
        );
        std::fs::remove_dir_all(&home).unwrap();
        assert_eq!(installed_system_image(&home), None);
    }

    #[test]
    fn versions_sort_as_numbers() {
        assert!(version_key("10.0.0") > version_key("9.0.0"));
        assert!(version_key("27.0.12077973") > version_key("26.3.11579264"));
        assert!(version_key("36.1.0") > version_key("36.0.0"));
    }

    #[test]
    fn the_newest_version_directory_wins() {
        let dir = std::env::temp_dir().join(format!("frus-ndk-{}", std::process::id()));
        for version in ["26.3.11579264", "27.0.12077973", "9.9.9"] {
            std::fs::create_dir_all(dir.join(version)).unwrap();
        }
        assert_eq!(newest_version_dir(&dir), Some(dir.join("27.0.12077973")));
        assert_eq!(newest_version_dir(&dir.join("missing")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn frus_own_sdk_is_the_last_resort() {
        if std::env::var_os("FRUS_ANDROID_SDK").is_none() {
            assert!(sdk_candidates().last().unwrap().ends_with("android-sdk"));
        }
    }
}
