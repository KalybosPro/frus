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
            other => Err(format!("no method `{other}`")),
        };
        report::line(&match result {
            Ok(value) => object([("id", id), ("result", value)]),
            Err(error) => object([("id", id), ("error", error.into())]),
        });
    }
    Ok(ExitCode::SUCCESS)
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
