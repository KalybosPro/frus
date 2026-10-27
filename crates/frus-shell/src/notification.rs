//! **System notifications** (milestone 701, part of #219): a message the system shows outside
//! the application's window, where it shows every other application's, and keeps in its list
//! when the person was away.
//!
//! [`SystemNotification`] describes one; [`SystemNotification::show`] shows it and **waits**
//! for what the person does with it: a click, which brings the window forward, or nothing,
//! until it is dismissed or the system takes it off the screen. Like the file dialogs, it
//! belongs on a background thread, and its answer comes back as a message:
//!
//! ```no_run
//! # use frus_shell::{Command, NotificationResponse, SystemNotification};
//! # enum Msg { Reminded(bool) }
//! # fn f() -> Command<Msg> {
//! Command::perform(|| {
//!     let response = SystemNotification::new("Water the plants")
//!         .body("Due today")
//!         .show();
//!     Msg::Reminded(response == Ok(NotificationResponse::Clicked))
//! })
//! # }
//! ```
//!
//! and from a component, `frus_widgets::spawn(|| SystemNotification::new("…").show(), …)`.
//! Asked for on the interface's thread, a notification is **refused** with an error that says
//! where to call it from.
//!
//! It is sent as the application: its window's title names it, and its icon — the frus logo
//! unless the application has its own — goes with it where the system shows one.
//!
//! - **Windows**: a toast. The system needs to know the sender, so the first notification
//!   records the application under the current user (`HKCU\Software\Classes\AppUserModelId`,
//!   one key named after the program, with its name and an icon kept in its cache folder). A
//!   click on the toast while it is on the screen comes back; one in the notification centre
//!   later does not, as nothing is registered to start the program again.
//! - **macOS**: the system's notifications, as the application's bundle. A program run
//!   outside a bundle is shown as the Finder, which is how macOS attributes it.
//! - **Linux**: the desktop's notification server, over D-Bus; with no server running,
//!   [`show`](SystemNotification::show) returns the error.
//!
//! Behind the `notifications` feature, on the desktop.

use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// A notification from the application, described: its title and the text under it. Shown by
/// [`show`](Self::show), from a background thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemNotification {
    title: String,
    body: String,
}

/// What the person did with a [`SystemNotification`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationResponse {
    /// They clicked it. The application's window has been brought forward.
    Clicked,
    /// It went away without a click: dismissed, or taken off the screen by the system.
    Closed,
}

/// Why a [`SystemNotification`] could not be shown: the system refused it, there is no
/// notification server to show it, or it was asked for on the interface's thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationError(String);

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NotificationError {}

impl SystemNotification {
    /// A notification titled `title`, with no text under it.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
        }
    }

    /// The text under the title.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Shows the notification and **waits** until the person clicks it or it goes away. A
    /// click also brings the application's window forward.
    pub fn show(self) -> Result<NotificationResponse, NotificationError> {
        if crate::interface_thread::current() {
            let error = "SystemNotification::show was called on the interface's thread, which \
                         it would stop: call it from Command::perform, or from spawn in a \
                         component";
            log::error!("{error}");
            return Err(NotificationError(error.to_owned()));
        }
        let sender = SENDER.lock().ok().and_then(|sender| sender.clone());
        let mut native = notify_rust::Notification::new();
        native.summary(&self.title).body(&self.body);
        #[cfg(windows)]
        if let Some(id) = sender.as_ref().and_then(windows::registered) {
            native.app_id(&id);
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            // A click on the notification itself is the action named "default", which the
            // server only offers when it is listed.
            native.action("default", "Open");
            if let Some(sender) = &sender {
                native.appname(&sender.name);
                if let Some(icon) = sender.icon_file() {
                    native.icon(&icon.to_string_lossy());
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            let _ = &sender;
            // Started from the Finder or the Dock, the bundle's identifier is in the
            // environment; elsewhere the library's default stands.
            if let Ok(bundle) = std::env::var("__CFBundleIdentifier") {
                let _ = notify_rust::set_application(&bundle);
            }
        }
        let error = |e: notify_rust::error::Error| NotificationError(e.to_string());
        log::debug!("notification {:?}: showing", self.title);
        let mut answer = NotificationResponse::Closed;
        native
            .show()
            .map_err(error)?
            .wait_for_response(|platform: &notify_rust::NotificationResponse| {
                answer = response(platform);
            })
            .map_err(error)?;
        log::debug!("notification {:?}: {answer:?}", self.title);
        if answer == NotificationResponse::Clicked {
            RAISE.store(true, Ordering::Relaxed);
        }
        Ok(answer)
    }
}

/// What the person did, from what the system reported: anything but a close is a click on the
/// notification or on one of its parts.
fn response(platform: &notify_rust::NotificationResponse) -> NotificationResponse {
    match platform {
        notify_rust::NotificationResponse::Closed(_) => NotificationResponse::Closed,
        _ => NotificationResponse::Clicked,
    }
}

/// Who sends the notifications: the application, as its window presents it.
#[derive(Clone, Debug)]
struct Sender {
    /// The window's title.
    name: String,
    /// The application's icon, as a PNG.
    icon: Option<Vec<u8>>,
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
impl Sender {
    /// The icon written to the application's cache folder, where the system can read it; `None`
    /// without an icon or a cache folder.
    fn icon_file(&self) -> Option<PathBuf> {
        let png = self.icon.as_deref()?;
        let dir = std::env::current_exe().ok().and_then(|exe| {
            crate::cache_dir::desktop(
                std::env::consts::OS,
                |name| std::env::var_os(name),
                exe.file_stem()?,
            )
        })?;
        let path = dir.join("notification-icon.png");
        if std::fs::read(&path).ok().as_deref() != Some(png) {
            std::fs::create_dir_all(&dir).ok()?;
            std::fs::write(&path, png).ok()?;
        }
        Some(path)
    }
}

static SENDER: Mutex<Option<Sender>> = Mutex::new(None);

/// A click asked for the window to come forward, which the interface's thread does when the
/// click's message arrives.
static RAISE: AtomicBool = AtomicBool::new(false);

/// The application's window exists: notifications are sent under its title and icon.
pub(crate) fn window_made(title: &str, icon: Option<&[u8]>) {
    if let Ok(mut slot) = SENDER.lock() {
        *slot = Some(Sender {
            name: title.to_owned(),
            icon: icon.map(<[u8]>::to_vec),
        });
    }
}

/// Whether a click asked for the window to come forward since the last call.
pub(crate) fn take_raise() -> bool {
    RAISE.swap(false, Ordering::Relaxed)
}

/// The sender's identity on Windows, which a toast cannot be shown without.
#[cfg(windows)]
mod windows {
    use std::sync::OnceLock;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};

    use super::Sender;

    /// The application's identity, recorded under the current user the first time it is
    /// needed in this run; `None` when it could not be.
    pub(super) fn registered(sender: &Sender) -> Option<String> {
        static ID: OnceLock<Option<String>> = OnceLock::new();
        ID.get_or_init(|| {
            let exe = std::env::current_exe().ok()?;
            let id = app_id(&exe.file_stem()?.to_string_lossy());
            let icon = sender.icon_file();
            let mut values = vec![("DisplayName", sender.name.clone())];
            if let Some(icon) = icon {
                values.push(("IconUri", icon.to_string_lossy().into_owned()));
            }
            match write(&format!(r"Software\Classes\AppUserModelId\{id}"), &values) {
                Ok(()) => Some(id),
                Err(status) => {
                    log::warn!("notifications: could not record the application ({status})");
                    None
                }
            }
        })
        .clone()
    }

    /// The identity a program is recorded under: `frus.app.` and its name, kept to the
    /// characters such an identity may hold.
    pub(super) fn app_id(program: &str) -> String {
        let name: String = program
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .take(100)
            .collect();
        format!("frus.app.{name}")
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// Writes `values` as strings under `HKEY_CURRENT_USER\path`, creating the key.
    fn write(path: &str, values: &[(&str, String)]) -> Result<(), u32> {
        let path = wide(path);
        for (name, value) in values {
            let data = wide(value);
            // SAFETY: the key, name and data are live, NUL-terminated wide strings, and the
            // size is the data's in bytes, its NUL included.
            let status = unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    path.as_ptr(),
                    wide(name).as_ptr(),
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            };
            if status != ERROR_SUCCESS {
                return Err(status);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The description is kept as given.**
    #[test]
    fn the_description_is_kept() {
        let notification = SystemNotification::new("Water the plants").body("Due today");
        assert_eq!(notification.title, "Water the plants");
        assert_eq!(notification.body, "Due today");
        assert_eq!(SystemNotification::new("Done").body, "");
    }

    /// **Only a close is not a click**: the notification itself, a button on it and a reply
    /// are all the person acting on it.
    #[test]
    fn anything_but_a_close_is_a_click() {
        use notify_rust::{CloseReason, NotificationResponse as Platform};
        assert_eq!(response(&Platform::Default), NotificationResponse::Clicked);
        assert_eq!(
            response(&Platform::Action("open".into())),
            NotificationResponse::Clicked
        );
        assert_eq!(
            response(&Platform::Reply("ok".into())),
            NotificationResponse::Clicked
        );
        for reason in [
            CloseReason::Expired,
            CloseReason::Dismissed,
            CloseReason::CloseAction,
            CloseReason::Other(4),
        ] {
            assert_eq!(
                response(&Platform::Closed(reason)),
                NotificationResponse::Closed
            );
        }
    }

    /// **On the interface's thread a notification is refused** with an error, before anything
    /// reaches the system.
    #[test]
    fn the_interface_thread_is_refused() {
        if !crate::interface_thread::claim_for_test() {
            return; // another test claimed it first: nothing to check from here
        }
        let refused = SystemNotification::new("Never shown").show();
        assert!(
            refused
                .as_ref()
                .is_err_and(|e| e.to_string().contains("Command::perform")),
            "{refused:?}"
        );
    }

    /// **A raise is taken once.**
    #[test]
    fn a_raise_is_taken_once() {
        RAISE.store(true, Ordering::Relaxed);
        assert!(take_raise());
        assert!(!take_raise());
    }

    /// **The Windows identity holds only what one may**: the program's name, with anything
    /// else made a dash, under the framework's prefix.
    #[cfg(windows)]
    #[test]
    fn the_identity_is_the_programs_name() {
        assert_eq!(windows::app_id("frus-demo"), "frus.app.frus-demo");
        assert_eq!(windows::app_id("my app.v2"), "frus.app.my-app-v2");
        assert_eq!(windows::app_id(&"x".repeat(300)).len(), 109);
    }
}
