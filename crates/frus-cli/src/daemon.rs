//! `frus daemon`: what an editor keeps running to know the devices.
//!
//! It speaks the same JSON lines as `frus run --machine` (see `report`): commands on standard
//! input, `{"id":1,"method":"device.getDevices"}`, and on standard output their answers,
//! `{"id":1,"result":[…]}`, and events, `{"event":"device.added","params":{…}}`.
//!
//! | method | answer |
//! |---|---|
//! | `daemon.version` | the tool's version |
//! | `daemon.shutdown` | `true`, then the daemon ends |
//! | `device.getDevices` | the devices now, as `frus devices` lists them |
//! | `device.enable` | `true`; from then on `device.added` and `device.removed` as phones are plugged in and out, emulators started and stopped |
//! | `device.disable` | `true`; no more device events |
//! | `device.takeScreenshot` | the screen of an Android device `deviceId`, a PNG in base64 |
//! | `daemon.getSupportedPlatforms` | which platforms the application in `projectRoot` builds for, and for the others why not and what to do |
//! | `emulator.getEmulators` | the Android emulators (and the iOS simulators on a Mac) that can be started |
//! | `emulator.launch` | starts `emulatorId` (`coldBoot`: without its saved state); it arrives as `device.added` once up |
//! | `emulator.create` | creates an Android emulator, named `name` or `frus_emulator`, installing what it needs: `{"success", "emulatorName", "error"}` |
//!
//! What the tool says on the way — installing, starting — comes as `daemon.logMessage` events,
//! and what the developer should see now as `daemon.showMessage`.
//!
//! Running an application is `frus run --machine -d <id>`, one process a run.

use std::collections::HashMap;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::devices::{self, Device, Kind, Platform};
use crate::json::{object, parse, Value};
use crate::report;

/// How often the devices are looked at again, once an editor asked to hear of changes.
const POLL: Duration = Duration::from_secs(3);

pub(crate) fn daemon() -> Result<ExitCode, String> {
    report::daemon();
    report::line(&object([
        ("event", "daemon.connected".into()),
        (
            "params",
            object([
                ("version", env!("CARGO_PKG_VERSION").into()),
                ("pid", u64::from(std::process::id()).into()),
            ]),
        ),
    ]));
    let watching = Arc::new(AtomicBool::new(false));
    let stdin = std::io::stdin();
    for line in std::io::BufRead::lines(stdin.lock()).map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let command = match parse(&line) {
            Ok(command) => command,
            Err(error) => {
                report::line(&object([("id", Value::Null), ("error", error.into())]));
                continue;
            }
        };
        let id = command.get("id").cloned().unwrap_or(Value::Null);
        let method = command.get("method").and_then(Value::as_str).unwrap_or("");
        let result = match method {
            "daemon.version" => Ok(Value::from(env!("CARGO_PKG_VERSION"))),
            "daemon.shutdown" => {
                report::line(&object([("id", id), ("result", true.into())]));
                return Ok(ExitCode::SUCCESS);
            }
            "device.getDevices" => Ok(Value::Array(
                devices::discover().iter().map(device_json).collect(),
            )),
            "device.enable" => {
                if !watching.swap(true, Ordering::SeqCst) {
                    watch(watching.clone());
                }
                Ok(Value::Bool(true))
            }
            "device.disable" => {
                watching.store(false, Ordering::SeqCst);
                Ok(Value::Bool(true))
            }
            "device.takeScreenshot" => take_screenshot(param(&command, "deviceId")),
            "daemon.getSupportedPlatforms" => param(&command, "projectRoot")
                .ok_or_else(|| "projectRoot is missing".to_string())
                .map(|root| supported_platforms(std::path::Path::new(&root))),
            "emulator.getEmulators" => Ok(Value::Array(emulators())),
            "emulator.launch" => launch_emulator(
                param(&command, "emulatorId"),
                command
                    .get("params")
                    .and_then(|p| p.get("coldBoot"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
            "emulator.create" => Ok(create_emulator(param(&command, "name").as_deref())),
            other => Err(format!("no method `{other}`")),
        };
        report::line(&match result {
            Ok(value) => object([("id", id), ("result", value)]),
            Err(error) => object([("id", id), ("error", error.into())]),
        });
    }
    Ok(ExitCode::SUCCESS)
}

/// A string parameter of a command.
fn param(command: &Value, name: &str) -> Option<String> {
    command
        .get("params")?
        .get(name)?
        .as_str()
        .map(str::to_string)
}

/// The emulators that can be started: every Android AVD, and the iOS simulators on a Mac.
fn emulators() -> Vec<Value> {
    let mut found = Vec::new();
    if let Some(emulator) = devices::emulator() {
        if let Ok(output) = std::process::Command::new(emulator)
            .arg("-list-avds")
            .output()
        {
            for avd in devices::parse_avds(&String::from_utf8_lossy(&output.stdout)) {
                found.push(emulator_json(&avd.id, &avd.name, "android"));
            }
        }
    }
    for device in devices::discover() {
        if let Kind::IosSimulator { udid } = &device.kind {
            found.push(emulator_json(udid, &device.name, "ios"));
        }
    }
    found
}

fn emulator_json(id: &str, name: &str, platform: &str) -> Value {
    object([
        ("id", id.into()),
        ("name", name.into()),
        ("category", "mobile".into()),
        ("platformType", platform.into()),
    ])
}

/// Starts an emulator and answers at once: it shows up as `device.added` when it is up.
fn launch_emulator(id: Option<String>, cold: bool) -> Result<Value, String> {
    let id = id.ok_or("emulatorId is missing")?;
    let known = emulators();
    let found = known
        .iter()
        .find(|e| e.get("id").and_then(Value::as_str) == Some(id.as_str()))
        .ok_or_else(|| format!("emulator '{id}' not found"))?;
    if found.get("platformType").and_then(Value::as_str) == Some("ios") {
        std::process::Command::new("xcrun")
            .args(["simctl", "boot", &id])
            .status()
            .map_err(|e| format!("could not boot the simulator: {e}"))?;
        let _ = std::process::Command::new("open")
            .args(["-a", "Simulator"])
            .status();
        return Ok(Value::Null);
    }
    let emulator = devices::emulator().ok_or("the Android emulator was not found in the SDK")?;
    report::say(&format!("Starting the {} emulator…", id.replace('_', " ")));
    let mut command = std::process::Command::new(&emulator);
    command.args(["-avd", &id]);
    // It outlives the daemon: started from its own folder, it holds no one else's.
    if let Some(dir) = emulator.parent() {
        command.current_dir(dir);
    }
    if cold {
        command.arg("-no-snapshot-load");
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the emulator: {e}"))?;
    Ok(Value::Null)
}

/// `emulator.create`, answered as the reference's daemon does: never an error, a result saying
/// whether it worked.
fn create_emulator(name: Option<&str>) -> Value {
    let taken: Vec<String> = emulators()
        .iter()
        .filter_map(|e| e.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    match crate::setup::create_avd(name, &taken) {
        Ok(created) => object([
            ("success", true.into()),
            ("emulatorName", created.as_str().into()),
            ("error", Value::Null),
        ]),
        Err(error) => {
            report::show("error", &error);
            object([
                ("success", false.into()),
                ("emulatorName", name.into()),
                ("error", error.into()),
            ])
        }
    }
}

/// The screen of an Android device, as a PNG in base64.
fn take_screenshot(id: Option<String>) -> Result<Value, String> {
    let id = id.ok_or("deviceId is missing")?;
    let device = devices::discover()
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| format!("device '{id}' not found"))?;
    let Kind::Android { serial } = &device.kind else {
        return Err(format!(
            "{} has no screenshot from here: in `frus run`, `s` saves the application's",
            device.name
        ));
    };
    let adb = devices::adb().ok_or("adb was not found")?;
    let output = std::process::Command::new(adb)
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .output()
        .map_err(|e| format!("could not take the screenshot: {e}"))?;
    if !output.status.success() || !output.stdout.starts_with(b"\x89PNG") {
        return Err(format!("{} took no screenshot", device.name));
    }
    Ok(Value::from(base64(&output.stdout).as_str()))
}

/// Standard base64, with padding.
pub(crate) fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// What the application in `root` builds for, as the reference's daemon answers it: the list of
/// platforms, and each one with whether it is supported and, if not, why and what to do.
pub(crate) fn supported_platforms(root: &std::path::Path) -> Value {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    let has_binary = root.join("src").join("main.rs").is_file()
        || std::fs::read_dir(root.join("src").join("bin")).is_ok_and(|mut e| e.next().is_some())
        || manifest.contains("[[bin]]");
    let has_android =
        manifest.contains("cdylib") && manifest.contains("[package.metadata.android]");
    let has_web = root.join("web").join("index.html").is_file();
    let reason =
        |text: &str, fix: &str| object([("reasonText", text.into()), ("fixText", fix.into())]);
    let desktop = |ok: bool| {
        if ok {
            Vec::new()
        } else {
            vec![reason(
                "the application has no desktop program (src/bin/<name>.rs)",
                "Run \"frus create .\" in your application directory",
            )]
        }
    };
    let platforms: Vec<(&str, Vec<Value>)> = vec![
        ("windows", desktop(has_binary)),
        ("macos", desktop(has_binary)),
        ("linux", desktop(has_binary)),
        (
            "android",
            if has_android {
                Vec::new()
            } else {
                vec![reason(
                    "the application is not set up for Android (crate-type \"cdylib\" and [package.metadata.android] in Cargo.toml)",
                    "Compare its Cargo.toml with one \"frus create\" writes",
                )]
            },
        ),
        (
            "web",
            if has_web {
                Vec::new()
            } else {
                vec![reason(
                    "the application has no web/index.html",
                    "Run \"frus create .\" in your application directory",
                )]
            },
        ),
        (
            "ios",
            vec![reason(
                "frus cannot run on iOS yet: its shell does not exist (ROADMAP, issue #19)",
                "None yet",
            )],
        ),
    ];
    let supported: Vec<Value> = platforms
        .iter()
        .filter(|(_, reasons)| reasons.is_empty())
        .map(|(name, _)| Value::from(*name))
        .collect();
    let types: Vec<(String, Value)> = platforms
        .into_iter()
        .map(|(name, reasons)| {
            let entry = if reasons.is_empty() {
                object([("isSupported", true.into())])
            } else {
                object([
                    ("isSupported", false.into()),
                    ("reasons", Value::Array(reasons)),
                ])
            };
            (name.to_string(), entry)
        })
        .collect();
    object([
        ("platforms", Value::Array(supported)),
        ("platformTypes", Value::Object(types)),
    ])
}

/// Looks at the devices every few seconds while `watching`, and says what came and went.
fn watch(watching: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut known: HashMap<String, Value> = HashMap::new();
        while watching.load(Ordering::SeqCst) {
            let now: HashMap<String, Value> = devices::discover()
                .iter()
                .map(|device| (device.id.clone(), device_json(device)))
                .collect();
            for (event, devices) in changes(&known, &now) {
                for device in devices {
                    report::line(&object([("event", event.into()), ("params", device)]));
                }
            }
            known = now;
            std::thread::sleep(POLL);
        }
    });
}

/// What appeared (or changed: a phone authorised) and what went, between two looks.
pub(crate) fn changes(
    before: &HashMap<String, Value>,
    now: &HashMap<String, Value>,
) -> Vec<(&'static str, Vec<Value>)> {
    let mut removed: Vec<Value> = before
        .iter()
        .filter(|(id, device)| now.get(*id) != Some(device))
        .map(|(_, device)| device.clone())
        .collect();
    let mut added: Vec<Value> = now
        .iter()
        .filter(|(id, device)| before.get(*id) != Some(device))
        .map(|(_, device)| device.clone())
        .collect();
    let id = |v: &Value| {
        v.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    removed.sort_by_key(id);
    added.sort_by_key(id);
    vec![("device.removed", removed), ("device.added", added)]
}

/// A device as an editor sees it.
pub(crate) fn device_json(device: &Device) -> Value {
    let (platform, category) = match device.platform {
        Platform::Desktop => (std::env::consts::OS, "desktop"),
        Platform::Android => ("android", "mobile"),
        Platform::Ios => ("ios", "mobile"),
        Platform::Web => ("web", "web"),
    };
    let emulator = matches!(
        device.kind,
        Kind::Emulator { .. } | Kind::IosSimulator { .. }
    ) || device.id.starts_with("emulator-");
    object([
        ("id", device.id.as_str().into()),
        ("name", device.name.as_str().into()),
        ("platform", platform.into()),
        ("category", category.into()),
        ("detail", device.detail.as_str().into()),
        ("emulator", emulator.into()),
        ("available", device.unavailable.is_none().into()),
        ("unavailableReason", device.unavailable.as_deref().into()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone(id: &str, available: bool) -> (String, Value) {
        (
            id.to_string(),
            object([("id", id.into()), ("available", available.into())]),
        )
    }

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe]), "//4=");
    }

    #[test]
    fn the_platforms_an_application_builds_for() {
        let dir = std::env::temp_dir().join(format!("frus-platforms-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src").join("bin")).unwrap();
        std::fs::write(dir.join("src").join("bin").join("app.rs"), "").unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"app\"\n[lib]\ncrate-type = [\"rlib\", \"cdylib\"]\n[package.metadata.android]\n",
        )
        .unwrap();
        let answer = supported_platforms(&dir);
        let names: Vec<&str> = match answer.get("platforms") {
            Some(Value::Array(list)) => list.iter().filter_map(Value::as_str).collect(),
            _ => panic!("no platforms"),
        };
        assert_eq!(names, ["windows", "macos", "linux", "android"]);
        let web = answer
            .get("platformTypes")
            .and_then(|t| t.get("web"))
            .unwrap();
        assert_eq!(web.get("isSupported").and_then(Value::as_bool), Some(false));
        assert!(web.to_json().contains("frus create ."), "{}", web.to_json());

        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(dir.join("web").join("index.html"), "").unwrap();
        let answer = supported_platforms(&dir);
        assert!(answer
            .get("platforms")
            .unwrap()
            .to_json()
            .contains("\"web\""));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn changes_are_what_came_and_what_went() {
        let before: HashMap<_, _> = [phone("windows", true), phone("A1", false)].into();
        let now: HashMap<_, _> =
            [phone("windows", true), phone("A1", true), phone("B2", true)].into();
        let found = changes(&before, &now);
        let ids = |list: &Vec<Value>| -> Vec<String> {
            list.iter()
                .map(|v| v.get("id").and_then(Value::as_str).unwrap().to_string())
                .collect()
        };
        // An authorised phone is removed as it was and added as it is.
        assert_eq!(found[0].0, "device.removed");
        assert_eq!(ids(&found[0].1), ["A1"]);
        assert_eq!(found[1].0, "device.added");
        assert_eq!(ids(&found[1].1), ["A1", "B2"]);
        assert!(changes(&now, &now).iter().all(|(_, list)| list.is_empty()));
    }

    #[test]
    fn a_device_is_described_for_an_editor() {
        let parsed = devices::parse_adb_devices(
            "List of devices attached\nemulator-5554 device model:Pixel_7 transport_id:1\n",
        );
        let json = device_json(&parsed[0]);
        assert_eq!(
            json.get("platform").and_then(Value::as_str),
            Some("android")
        );
        assert_eq!(json.get("emulator").and_then(Value::as_bool), Some(true));
        assert_eq!(json.get("name").and_then(Value::as_str), Some("Pixel 7"));
        assert_eq!(json.get("available").and_then(Value::as_bool), Some(true));
    }
}
