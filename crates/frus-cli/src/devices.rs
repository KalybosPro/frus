//! What an application can run on, right now: this computer, the Android devices `adb` sees
//! and the emulators that can be started, the iOS simulators on a Mac, the browsers installed
//! here, and a bare web server for any other browser.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The platforms frus targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Desktop,
    Android,
    /// Listed so a developer sees their simulators, and is told plainly that frus cannot run
    /// there yet: the iOS shell does not exist (ROADMAP, #19).
    Ios,
    Web,
}

impl Platform {
    fn word(self) -> &'static str {
        match self {
            Platform::Desktop => "desktop",
            Platform::Android | Platform::Ios => "mobile",
            Platform::Web => "web",
        }
    }
}

/// Something to run on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Device {
    /// What `-d` takes: `windows`, an adb serial, `edge`, `web-server`.
    pub id: String,
    /// What a person calls it.
    pub name: String,
    pub platform: Platform,
    pub kind: Kind,
    /// The platform in more detail: `windows-x64`, `android-arm64 (Android 10)`.
    pub detail: String,
    /// Why it cannot be used now (`unauthorized`, `offline`), if it cannot.
    pub unavailable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The computer this runs on.
    Host,
    /// A device `adb` knows by this serial.
    Android { serial: String },
    /// An Android emulator that is not running: `frus run` starts it first.
    Emulator { avd: String },
    /// An iOS simulator, by its UDID.
    IosSimulator { udid: String },
    /// A browser frus opens the application in.
    Browser { exe: PathBuf },
    /// No browser: the application is served, and any browser can open it.
    WebServer,
}

/// Every device there is now, the desktop first.
pub(crate) fn discover() -> Vec<Device> {
    let mut devices = vec![host()];
    let mut running_avds = Vec::new();
    if let Some(adb) = adb() {
        if let Ok(output) = Command::new(&adb).args(["devices", "-l"]).output() {
            for mut device in parse_adb_devices(&String::from_utf8_lossy(&output.stdout)) {
                if device.unavailable.is_none() {
                    device.detail = android_detail(&adb, &device.id);
                    if let Some(avd) = running_avd(&adb, &device.id) {
                        device.name = avd.replace('_', " ");
                        running_avds.push(avd);
                    }
                }
                devices.push(device);
            }
        }
    }
    if let Some(emulator) = emulator() {
        if let Ok(output) = Command::new(emulator).arg("-list-avds").output() {
            devices.extend(
                parse_avds(&String::from_utf8_lossy(&output.stdout))
                    .into_iter()
                    .filter(|device| !running_avds.contains(&device.id)),
            );
        }
    }
    if cfg!(target_os = "macos") {
        if let Ok(output) = Command::new("xcrun")
            .args(["simctl", "list", "devices", "available"])
            .output()
        {
            devices.extend(parse_simulators(&String::from_utf8_lossy(&output.stdout)));
        }
    }
    devices.extend(browsers());
    devices.push(Device {
        id: "web-server".into(),
        name: "Web Server".into(),
        platform: Platform::Web,
        kind: Kind::WebServer,
        detail: "web-javascript".into(),
        unavailable: None,
    });
    devices
}

/// The computer itself.
fn host() -> Device {
    let (id, name) = match std::env::consts::OS {
        "windows" => ("windows", "Windows"),
        "macos" => ("macos", "macOS"),
        "linux" => ("linux", "Linux"),
        other => (other, other),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    Device {
        id: id.into(),
        name: name.into(),
        platform: Platform::Desktop,
        kind: Kind::Host,
        detail: format!("{id}-{arch}"),
        unavailable: None,
    }
}

/// `adb devices -l`, read: one device a line after the header.
pub(crate) fn parse_adb_devices(output: &str) -> Vec<Device> {
    output
        .lines()
        .filter(|line| !line.starts_with("List of devices") && !line.starts_with('*'))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let serial = fields.next()?;
            let state = fields.next()?;
            let model = fields
                .find_map(|field| field.strip_prefix("model:"))
                .map(|model| model.replace('_', " "));
            Some(Device {
                id: serial.to_string(),
                name: model.unwrap_or_else(|| serial.to_string()),
                platform: Platform::Android,
                kind: Kind::Android {
                    serial: serial.to_string(),
                },
                detail: "android".into(),
                unavailable: match state {
                    "device" => None,
                    "unauthorized" => {
                        Some("unauthorized: allow USB debugging on the device".into())
                    }
                    other => Some(other.to_string()),
                },
            })
        })
        .collect()
}

/// The AVD an emulator was started from — `None` for a phone.
fn running_avd(adb: &Path, serial: &str) -> Option<String> {
    if !serial.starts_with("emulator-") {
        return None;
    }
    let output = Command::new(adb)
        .args(["-s", serial, "emu", "avd", "name"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "OK")
        .map(str::to_string)
}

/// `emulator -list-avds`: one AVD a line, each an emulator that can be started.
pub(crate) fn parse_avds(output: &str) -> Vec<Device> {
    output
        .lines()
        .map(str::trim)
        // The emulator prints its own warnings first on some systems, `INFO | ...`.
        .filter(|line| !line.is_empty() && !line.contains(' ') && !line.contains('|'))
        .map(|avd| Device {
            id: avd.to_string(),
            name: avd.replace('_', " "),
            platform: Platform::Android,
            kind: Kind::Emulator {
                avd: avd.to_string(),
            },
            detail: "android emulator (not started)".into(),
            unavailable: None,
        })
        .collect()
}

/// `xcrun simctl list devices available`: the iOS simulators, under their runtime's heading.
///
/// ```text
/// == Devices ==
/// -- iOS 17.2 --
///     iPhone 15 (5A1B…) (Shutdown)
/// ```
pub(crate) fn parse_simulators(output: &str) -> Vec<Device> {
    let mut runtime = None;
    let mut devices = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed
            .strip_prefix("-- ")
            .and_then(|h| h.strip_suffix(" --"))
        {
            runtime = heading.starts_with("iOS").then(|| heading.to_string());
            continue;
        }
        let Some(runtime) = &runtime else { continue };
        // `Name (UDID) (State)`, the name itself possibly holding parentheses.
        let mut parts = trimmed.rsplitn(3, " (");
        let (Some(state), Some(udid), Some(name)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (state, udid) = (state.trim_end_matches(')'), udid.trim_end_matches(')'));
        if udid.len() < 30 {
            continue;
        }
        devices.push(Device {
            id: udid.to_string(),
            name: name.to_string(),
            platform: Platform::Ios,
            kind: Kind::IosSimulator {
                udid: udid.to_string(),
            },
            detail: format!("ios simulator ({runtime}, {})", state.to_lowercase()),
            unavailable: None,
        });
    }
    devices
}

/// `android-arm64 (Android 10)`, asked of the device.
fn android_detail(adb: &Path, serial: &str) -> String {
    let prop = |name: &str| {
        Command::new(adb)
            .args(["-s", serial, "shell", "getprop", name])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let abi = match prop("ro.product.cpu.abi").as_deref() {
        Some("arm64-v8a") => "android-arm64",
        Some("armeabi-v7a") => "android-arm",
        Some("x86_64") => "android-x64",
        Some("x86") => "android-x86",
        _ => "android",
    };
    match prop("ro.build.version.release") {
        Some(version) => format!("{abi} (Android {version})"),
        None => abi.to_string(),
    }
}

/// The `adb` program: the SDK's, or one on the `PATH`.
pub(crate) fn adb() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "adb.exe" } else { "adb" };
    crate::setup::existing_sdk()
        .map(|sdk| sdk.join("platform-tools").join(exe))
        .filter(|path| path.is_file())
        .or_else(|| on_path(exe))
}

/// The Android `emulator` program: on the `PATH`, or in the SDK.
pub(crate) fn emulator() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "emulator.exe"
    } else {
        "emulator"
    };
    crate::setup::existing_sdk()
        .map(|sdk| sdk.join("emulator").join(exe))
        .filter(|path| path.is_file())
        .or_else(|| on_path(exe))
}

/// The browsers frus knows how to open a page in, those installed here.
fn browsers() -> Vec<Device> {
    let candidates: &[(&str, &str, &[&str])] = if cfg!(windows) {
        &[
            (
                "edge",
                "Edge",
                &[
                    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
                ],
            ),
            (
                "chrome",
                "Chrome",
                &[
                    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                ],
            ),
        ]
    } else if cfg!(target_os = "macos") {
        &[
            (
                "chrome",
                "Chrome",
                &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"],
            ),
            (
                "edge",
                "Edge",
                &["/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"],
            ),
        ]
    } else {
        &[
            (
                "chrome",
                "Chrome",
                &[
                    "google-chrome",
                    "google-chrome-stable",
                    "chromium",
                    "chromium-browser",
                ],
            ),
            ("edge", "Edge", &["microsoft-edge", "microsoft-edge-stable"]),
        ]
    };
    candidates
        .iter()
        .filter_map(|(id, name, paths)| {
            let exe = paths.iter().find_map(|path| {
                let path = Path::new(path);
                if path.is_absolute() {
                    path.is_file().then(|| path.to_path_buf())
                } else {
                    on_path(path.to_str()?)
                }
            })?;
            Some(Device {
                id: id.to_string(),
                name: name.to_string(),
                platform: Platform::Web,
                kind: Kind::Browser { exe },
                detail: "web-javascript".into(),
                unavailable: None,
            })
        })
        .collect()
}

/// `name` in one of the `PATH`'s directories.
fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// The device `-d` names: its id, then its name, then a platform (`android`, `web`) when only
/// one device has it. Without `-d`, the desktop.
pub(crate) fn select(devices: &[Device], wanted: Option<&str>) -> Result<Device, String> {
    let Some(wanted) = wanted else {
        return Ok(devices[0].clone());
    };
    let wanted_lower = wanted.to_lowercase();
    let usable = |device: &Device| match &device.unavailable {
        None => Ok(device.clone()),
        Some(why) => Err(format!(
            "{} ({}) cannot be used: {why}",
            device.name, device.id
        )),
    };
    if let Some(device) = devices.iter().find(|d| d.id.to_lowercase() == wanted_lower) {
        return usable(device);
    }
    let platform = match wanted_lower.as_str() {
        "desktop" => Some(Platform::Desktop),
        "android" => Some(Platform::Android),
        "ios" | "iphone" | "ipad" => Some(Platform::Ios),
        "web" => Some(Platform::Web),
        _ => None,
    };
    let matches: Vec<&Device> = devices
        .iter()
        .filter(|d| match platform {
            Some(platform) => d.platform == platform && d.kind != Kind::WebServer,
            None => d.name.to_lowercase().contains(&wanted_lower),
        })
        .collect();
    match matches.as_slice() {
        [device] => usable(device),
        [] => Err(format!(
            "no device matches `{wanted}`. The devices:\n\n{}",
            table(devices)
        )),
        several => Err(format!(
            "`{wanted}` matches more than one device; name one by its id:\n\n{}",
            table(&several.iter().map(|d| (*d).clone()).collect::<Vec<_>>())
        )),
    }
}

/// The devices as `frus devices` prints them: name • id • detail, in aligned columns.
pub(crate) fn table(devices: &[Device]) -> String {
    let names: Vec<String> = devices
        .iter()
        .map(|d| format!("{} ({})", d.name, d.platform.word()))
        .collect();
    let name_width = names.iter().map(|n| n.chars().count()).max().unwrap_or(0);
    let id_width = devices
        .iter()
        .map(|d| d.id.chars().count())
        .max()
        .unwrap_or(0);
    devices
        .iter()
        .zip(&names)
        .map(|(device, name)| {
            let mut line = format!(
                "{name:<name_width$} • {:<id_width$} • {}",
                device.id, device.detail
            );
            if let Some(why) = &device.unavailable {
                line.push_str(&format!(" — {why}"));
            }
            line.trim_end().to_string() + "\n"
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADB: &str = "List of devices attached\n\
        XMJNW19B23011768       device usb:1-1 product:STK-L21 model:STK_L21 device:HWSTK-HF transport_id:3\n\
        emulator-5554          offline transport_id:4\n\
        R58M12345              unauthorized usb:1-2 transport_id:5\n\n";

    #[test]
    fn adb_devices_are_read() {
        let devices = parse_adb_devices(ADB);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].id, "XMJNW19B23011768");
        assert_eq!(devices[0].name, "STK L21");
        assert_eq!(devices[0].unavailable, None);
        assert_eq!(devices[1].name, "emulator-5554");
        assert_eq!(devices[1].unavailable.as_deref(), Some("offline"));
        assert!(devices[2]
            .unavailable
            .as_ref()
            .unwrap()
            .contains("allow USB debugging"));
        assert!(parse_adb_devices("List of devices attached\n\n").is_empty());
        assert!(
            parse_adb_devices("* daemon started successfully\nList of devices attached\n")
                .is_empty()
        );
    }

    fn sample() -> Vec<Device> {
        let mut devices = vec![host()];
        devices.extend(parse_adb_devices(ADB));
        devices.push(Device {
            id: "edge".into(),
            name: "Edge".into(),
            platform: Platform::Web,
            kind: Kind::Browser {
                exe: PathBuf::from("msedge"),
            },
            detail: "web-javascript".into(),
            unavailable: None,
        });
        devices.push(Device {
            id: "web-server".into(),
            name: "Web Server".into(),
            platform: Platform::Web,
            kind: Kind::WebServer,
            detail: "web-javascript".into(),
            unavailable: None,
        });
        devices
    }

    #[test]
    fn devices_are_picked_by_id_name_or_platform() {
        let devices = sample();
        assert_eq!(select(&devices, None).unwrap().platform, Platform::Desktop);
        assert_eq!(select(&devices, Some("edge")).unwrap().id, "edge");
        assert_eq!(select(&devices, Some("EDGE")).unwrap().id, "edge");
        assert_eq!(
            select(&devices, Some("stk")).unwrap().id,
            "XMJNW19B23011768"
        );
        // `web` means a browser, not the bare server, so it is not ambiguous.
        assert_eq!(select(&devices, Some("web")).unwrap().id, "edge");
        assert_eq!(
            select(&devices, Some("web-server")).unwrap().kind,
            Kind::WebServer
        );
        assert_eq!(select(&devices, Some("desktop")).unwrap().kind, Kind::Host);
        let mut with_simulator = devices.clone();
        with_simulator.extend(parse_simulators(
            "-- iOS 17.2 --
    iPhone 15 (5A1B2C3D-0000-1111-2222-333344445555) (Shutdown)
",
        ));
        assert_eq!(
            select(&with_simulator, Some("ios")).unwrap().platform,
            Platform::Ios
        );
    }

    #[test]
    fn a_wrong_or_vague_name_says_what_there_is() {
        let devices = sample();
        let none = select(&devices, Some("iphone")).unwrap_err();
        assert!(none.contains("no device matches `iphone`"), "{none}");
        assert!(none.contains("Edge (web)"), "{none}");
        // Three Android devices: `android` alone does not say which.
        let vague = select(&devices, Some("android")).unwrap_err();
        assert!(vague.contains("more than one"), "{vague}");
        let unusable = select(&devices, Some("emulator-5554")).unwrap_err();
        assert!(unusable.contains("cannot be used: offline"), "{unusable}");
    }

    #[test]
    fn emulators_and_simulators_are_read() {
        let avds =
            parse_avds("INFO    | Storing crashdata in: C:\\tmp\nPixel_7_API_34\nSmall_Phone\n\n");
        assert_eq!(avds.len(), 2);
        assert_eq!(avds[0].id, "Pixel_7_API_34");
        assert_eq!(avds[0].name, "Pixel 7 API 34");
        assert_eq!(
            avds[0].kind,
            Kind::Emulator {
                avd: "Pixel_7_API_34".into()
            }
        );

        let simctl = "== Devices ==\n-- iOS 17.2 --\n    iPhone 15 (5A1B2C3D-0000-1111-2222-333344445555) (Shutdown) \n    \
                      iPad Pro (11-inch) (4th generation) (9F8E7D6C-0000-1111-2222-333344445555) (Booted) \n\
                      -- watchOS 10.2 --\n    Apple Watch (AAAAAAAA-0000-1111-2222-333344445555) (Shutdown) \n";
        let simulators = parse_simulators(simctl);
        assert_eq!(simulators.len(), 2, "{simulators:?}");
        assert_eq!(simulators[0].name, "iPhone 15");
        assert_eq!(simulators[0].id, "5A1B2C3D-0000-1111-2222-333344445555");
        assert_eq!(simulators[0].detail, "ios simulator (iOS 17.2, shutdown)");
        assert_eq!(simulators[1].name, "iPad Pro (11-inch) (4th generation)");
        assert_eq!(simulators[1].platform, Platform::Ios);
    }

    #[test]
    fn the_table_lines_up() {
        let table = table(&sample());
        let lines: Vec<&str> = table.lines().collect();
        let column = |line: &str| line.find('•').unwrap();
        assert!(
            lines.iter().all(|line| column(line) == column(lines[0])),
            "{table}"
        );
        assert!(table.contains("STK L21 (mobile)"), "{table}");
        assert!(table.contains("— offline"), "{table}");
    }
}
