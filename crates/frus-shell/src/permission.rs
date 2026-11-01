//! **Permissions** (milestone 730): asking the person for the camera, the microphone, the
//! location and notifications, and knowing what they said.
//!
//! [`Permission::status`] says where a permission stands without asking anything;
//! [`Permission::request`] asks when there is something to ask, and waits for the answer. Both
//! are `async`, because the web answers no other way; from a handler they read in order:
//!
//! ```no_run
//! # use frus_shell::{Permission, PermissionStatus};
//! # async fn f() {
//! match Permission::Camera.request().await {
//!     PermissionStatus::Granted => { /* open the camera */ }
//!     PermissionStatus::Blocked => frus_shell::open_permission_settings(),
//!     _ => { /* explain why the camera is needed, and offer to ask again */ }
//! }
//! # }
//! ```
//!
//! What each platform does:
//!
//! - **Android**: the system's runtime permissions — `CAMERA`, `RECORD_AUDIO`,
//!   `ACCESS_FINE_LOCATION` with `ACCESS_COARSE_LOCATION`, and `POST_NOTIFICATIONS` from Android
//!   13. The application's manifest must ask for each one it uses (for cargo-apk,
//!   `[[package.metadata.android.uses_permission]]`), or the system refuses without asking: the
//!   status is then [`PermissionStatus::Blocked`] and the log says which is missing. A
//!   NativeActivity is not told the answer, so it is read from the permission once the
//!   question's dialog has gone. After two refusals (Android 11 and later) the system stops
//!   asking: [`PermissionStatus::Blocked`], and [`open_permission_settings`] is the way left.
//! - **The web**: the browser's permissions. The status comes from `navigator.permissions`; a
//!   request is what the browser asks on — the camera or the microphone opened for a moment, the
//!   position read once, notifications asked for. A page served over plain `http` from anywhere
//!   but `localhost` gets [`PermissionStatus::Unavailable`] for the camera, the microphone and
//!   the location: browsers keep them for secure pages.
//! - **Windows**: a desktop application is not asked; the person's privacy settings say whether
//!   desktop applications may use the camera, the microphone and the location, and the status
//!   reads them ([`PermissionStatus::Blocked`] when they say no). [`open_permission_settings`] opens the
//!   page for that permission.
//! - **Linux**: nothing to ask: [`PermissionStatus::Granted`].
//! - **macOS**: the system asks the person itself, the first time the application uses the
//!   camera, the microphone or the location — if the application's bundle says why it wants
//!   them (`NSCameraUsageDescription` and the others). frus does not ask ahead of that yet, so
//!   the status is [`PermissionStatus::Ask`] and a request answers the same.
//!
//! A request must not be made on the interface's thread natively, where it would stop every
//! frame while the question is up: call it from an `async` handler
//! (`cx.async_callback`), which runs it where waiting is allowed.

use std::fmt;

/// Something an application asks the person's permission for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    /// The camera.
    Camera,
    /// The microphone.
    Microphone,
    /// Where the device is.
    Location,
    /// Showing notifications.
    Notifications,
}

/// Where a permission stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PermissionStatus {
    /// Allowed.
    Granted,
    /// Not decided yet: a request shows the question.
    Ask,
    /// Refused, and a request shows the question again.
    Denied,
    /// Refused in a way only the system's settings can change — for good, by a policy, or
    /// because the application does not declare it. [`open_permission_settings`] goes there.
    Blocked,
    /// Not something this platform, or this page, can grant.
    Unavailable,
}

impl PermissionStatus {
    /// Whether the permission is granted.
    pub fn is_granted(self) -> bool {
        self == PermissionStatus::Granted
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Permission::Camera => "the camera",
            Permission::Microphone => "the microphone",
            Permission::Location => "the location",
            Permission::Notifications => "notifications",
        })
    }
}

impl Permission {
    /// Where this permission stands, without asking anything.
    pub async fn status(self) -> PermissionStatus {
        #[cfg(android)]
        {
            crate::android_permission::status(self)
        }
        #[cfg(web)]
        {
            crate::web_permission::status(self).await
        }
        #[cfg(desktop)]
        {
            // Windows reads its privacy settings, which takes a process: not on the
            // interface's thread.
            blocking::unblock(move || desktop::status(self)).await
        }
        #[cfg(ios)]
        {
            PermissionStatus::Unavailable
        }
    }

    /// Asks the person for this permission when there is something to ask, and answers where
    /// it stands afterwards. Granted, blocked or unavailable, nothing is asked.
    pub async fn request(self) -> PermissionStatus {
        #[cfg(android)]
        {
            crate::android_permission::request(self).await
        }
        #[cfg(web)]
        {
            crate::web_permission::request(self).await
        }
        #[cfg(desktop)]
        {
            // Nothing asks ahead on the desktop (see the module's notes): where it stands is
            // the answer.
            self.status().await
        }
        #[cfg(ios)]
        {
            PermissionStatus::Unavailable
        }
    }
}

/// Opens the system's settings where the application's permissions are changed: the
/// application's page on Android, the privacy pages on Windows and macOS. Nothing on Linux and
/// the web, which have no such page to open.
pub fn open_permission_settings() {
    #[cfg(android)]
    crate::android_permission::open_settings();
    #[cfg(desktop)]
    desktop::open_settings();
}

/// Where an Android permission's question stands (milestones 705, 730), from what is seen while
/// it is asked: `allowed` nonzero once the answer is known to be yes (or a no the system gave
/// without asking), 0 while it is not,
/// whether the application's window has the focus, whether it lost it since the question was
/// put, and how long ago that was.
///
/// A refusal reads exactly like a question not yet answered — the permission is simply not
/// granted — so the focus tells them apart: the question is a dialog over the window, which
/// takes the focus while it is up and gives it back once answered. A focus never lost means no
/// dialog came: Android refuses without asking once the person has said no twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) enum Question {
    /// Still being asked.
    Open,
    /// Answered, or refused without asking: what `allowed` now says is the answer.
    Settled,
    /// No answer in the time given.
    Unanswered,
}

/// How long a question that put up no dialog is given before it is taken as refused.
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) const NO_DIALOG: std::time::Duration = std::time::Duration::from_secs(2);

/// How long the person is given to answer the question.
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) const ASKING: std::time::Duration = std::time::Duration::from_secs(60);

/// See [`Question`].
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) fn question(
    allowed: i32,
    focused: bool,
    was_away: bool,
    elapsed: std::time::Duration,
) -> Question {
    if allowed != 0 || (was_away && focused) || (!was_away && elapsed >= NO_DIALOG) {
        Question::Settled
    } else if elapsed >= ASKING {
        Question::Unanswered
    } else {
        Question::Open
    }
}

/// The Android permissions that stand for `permission`, every one of which must be granted.
#[cfg_attr(not(any(android, test)), allow(dead_code))]
pub(crate) fn android_names(permission: Permission) -> &'static [&'static str] {
    match permission {
        Permission::Camera => &["android.permission.CAMERA"],
        Permission::Microphone => &["android.permission.RECORD_AUDIO"],
        Permission::Location => &[
            "android.permission.ACCESS_FINE_LOCATION",
            "android.permission.ACCESS_COARSE_LOCATION",
        ],
        Permission::Notifications => &["android.permission.POST_NOTIFICATIONS"],
    }
}

/// One code for `permission` from the codes of the Android permissions that stand for it, in
/// `android_names`'s order (`FrusPermissions.status`'s numbers: 1 granted, 0 not asked, -1
/// refused, -2 refused for good, -3 not in the manifest).
///
/// The location is granted by **either** of its two: from Android 12 the person may allow only
/// the approximate one, which is still the location (seen on an emulator, milestone 730: the
/// "every one" rule called that blocked). One the manifest leaves out does not count against
/// the other. Everything else needs every one of its permissions.
#[cfg_attr(not(any(android, test)), allow(dead_code))]
pub(crate) fn combine(permission: Permission, codes: &[i32]) -> i32 {
    let declared: Vec<i32> = codes.iter().copied().filter(|&code| code != -3).collect();
    if declared.is_empty() {
        return -3;
    }
    match permission {
        Permission::Location if declared.contains(&1) => 1,
        Permission::Location => declared.into_iter().max().unwrap_or(-3),
        _ if declared.len() < codes.len() => -3,
        _ => declared.into_iter().min().unwrap_or(-3),
    }
}

/// What `FrusPermissions.status` answers, as a status.
#[cfg_attr(not(any(android, test)), allow(dead_code))]
pub(crate) fn from_android(code: i32) -> PermissionStatus {
    match code {
        1 => PermissionStatus::Granted,
        0 => PermissionStatus::Ask,
        -1 => PermissionStatus::Denied,
        _ => PermissionStatus::Blocked,
    }
}

/// What the browser's `PermissionStatus.state` says, as a status.
#[cfg_attr(not(any(web, test)), allow(dead_code))]
pub(crate) fn from_browser(state: &str) -> PermissionStatus {
    match state {
        "granted" => PermissionStatus::Granted,
        "prompt" => PermissionStatus::Ask,
        // A page cannot ask again once refused: only the site's settings change it.
        "denied" => PermissionStatus::Blocked,
        _ => PermissionStatus::Unavailable,
    }
}

/// What a Windows privacy setting's `Value` says, as a status.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) fn from_windows_consent(value: Option<&str>) -> PermissionStatus {
    match value {
        Some(value) if value.eq_ignore_ascii_case("deny") => PermissionStatus::Blocked,
        // "Allow", or nothing written, which is the system's default: allowed.
        _ => PermissionStatus::Granted,
    }
}

#[cfg(desktop)]
mod desktop {
    use super::{Permission, PermissionStatus};

    pub(super) fn status(permission: Permission) -> PermissionStatus {
        #[cfg(windows)]
        {
            windows_status(permission)
        }
        #[cfg(target_os = "macos")]
        {
            match permission {
                Permission::Notifications => PermissionStatus::Granted,
                _ => PermissionStatus::Ask,
            }
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let _ = permission;
            PermissionStatus::Granted
        }
    }

    /// The privacy setting's capability for `permission`.
    #[cfg(windows)]
    fn capability(permission: Permission) -> Option<&'static str> {
        match permission {
            Permission::Camera => Some("webcam"),
            Permission::Microphone => Some("microphone"),
            Permission::Location => Some("location"),
            Permission::Notifications => None,
        }
    }

    /// Windows asks no desktop application; its privacy settings say whether desktop
    /// applications may use a device at all, under the current user and, by policy, under the
    /// machine. Either saying no blocks it.
    #[cfg(windows)]
    fn windows_status(permission: Permission) -> PermissionStatus {
        let Some(capability) = capability(permission) else {
            return PermissionStatus::Granted;
        };
        let store = format!(
            r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\{capability}"
        );
        let keys = [
            format!(r"HKLM\{store}"),
            format!(r"HKCU\{store}"),
            format!(r"HKCU\{store}\NonPackaged"),
        ];
        for key in keys {
            let value = read_value(&key);
            if super::from_windows_consent(value.as_deref()) == PermissionStatus::Blocked {
                return PermissionStatus::Blocked;
            }
        }
        PermissionStatus::Granted
    }

    /// A key's `Value`, through `reg query`, which every Windows has.
    #[cfg(windows)]
    fn read_value(key: &str) -> Option<String> {
        use std::os::windows::process::CommandExt;
        // No console window flashing up for a program that has its own.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = std::process::Command::new("reg")
            .args(["query", key, "/v", "Value"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        out.status.success().then_some(())?;
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| {
                let mut words = line.split_whitespace();
                (words.next() == Some("Value"))
                    .then(|| words.nth(1).map(str::to_owned))
                    .flatten()
            })
    }

    pub(super) fn open_settings() {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", "ms-settings:privacy"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy")
                .spawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Android's question is settled by an answer, by the focus coming back, or by no dialog
    /// at all** — a refusal reads like no answer yet, and the focus is what tells them apart.
    #[test]
    fn the_question_is_settled_by_the_focus_coming_back() {
        use std::time::Duration;
        let soon = Duration::from_millis(500);
        // Allowed or refused outright: settled at once.
        assert_eq!(question(1, true, false, soon), Question::Settled);
        assert_eq!(question(-1, true, false, soon), Question::Settled);
        // The dialog is up: the window has lost the focus.
        assert_eq!(question(0, false, true, soon), Question::Open);
        assert_eq!(
            question(0, false, true, Duration::from_secs(30)),
            Question::Open
        );
        // The focus is back and the permission still not granted: the person said no.
        assert_eq!(question(0, true, true, soon), Question::Settled);
        // No dialog came: open for a moment, then refused without asking.
        assert_eq!(question(0, true, false, soon), Question::Open);
        assert_eq!(question(0, true, false, NO_DIALOG), Question::Settled);
        // A dialog left unanswered: given up on.
        assert_eq!(question(0, false, true, ASKING), Question::Unanswered);
    }

    #[test]
    fn the_android_answers_map_to_statuses() {
        assert_eq!(from_android(1), PermissionStatus::Granted);
        assert_eq!(from_android(0), PermissionStatus::Ask);
        assert_eq!(from_android(-1), PermissionStatus::Denied);
        assert_eq!(from_android(-2), PermissionStatus::Blocked);
        assert_eq!(from_android(-3), PermissionStatus::Blocked);
    }

    #[test]
    fn an_approximate_location_is_the_location() {
        // Only the approximate one allowed: granted (the precise one says "for good").
        assert_eq!(combine(Permission::Location, &[-2, 1]), 1);
        // A manifest that names only the approximate one.
        assert_eq!(combine(Permission::Location, &[-3, 0]), 0);
        assert_eq!(combine(Permission::Location, &[-3, -3]), -3);
        // Neither allowed: the more hopeful of the two, which a request can still change.
        assert_eq!(combine(Permission::Location, &[-1, -2]), -1);
    }

    #[test]
    fn the_camera_needs_every_one_of_its_permissions() {
        assert_eq!(combine(Permission::Camera, &[1]), 1);
        assert_eq!(combine(Permission::Camera, &[-3]), -3);
    }

    #[test]
    fn a_location_is_asked_with_both_android_permissions() {
        assert_eq!(android_names(Permission::Location).len(), 2);
        assert_eq!(
            android_names(Permission::Camera),
            ["android.permission.CAMERA"]
        );
    }

    #[test]
    fn a_page_refused_once_is_blocked() {
        assert_eq!(from_browser("granted"), PermissionStatus::Granted);
        assert_eq!(from_browser("prompt"), PermissionStatus::Ask);
        assert_eq!(from_browser("denied"), PermissionStatus::Blocked);
        assert_eq!(from_browser("anything else"), PermissionStatus::Unavailable);
    }

    #[test]
    fn a_windows_privacy_setting_blocks_only_when_it_says_deny() {
        assert_eq!(
            from_windows_consent(Some("Deny")),
            PermissionStatus::Blocked
        );
        assert_eq!(
            from_windows_consent(Some("Allow")),
            PermissionStatus::Granted
        );
        assert_eq!(from_windows_consent(None), PermissionStatus::Granted);
    }

    #[test]
    fn a_status_says_whether_it_is_granted() {
        assert!(PermissionStatus::Granted.is_granted());
        assert!(!PermissionStatus::Ask.is_granted());
    }
}
