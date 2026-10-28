//! **System notifications** (milestone 701, part of #219): a message the system shows outside
//! the application's window, where it shows every other application's, and keeps in its list
//! when the person was away.
//!
//! [`SystemNotification`] describes one — a title, a body, buttons, a picture, and whether it
//! makes the system's sound; [`SystemNotification::show`] shows it and **waits** for what the
//! person does with it: a click, which brings the window forward, a button pressed, or nothing,
//! until it is dismissed or the system takes it off the screen. Like the file dialogs, it
//! belongs on a background thread, and its answer comes back as a message:
//!
//! ```no_run
//! # use frus_shell::{Command, NotificationError, NotificationResponse, SystemNotification};
//! # enum Msg { Reminded(Result<NotificationResponse, NotificationError>) }
//! # fn f() -> Command<Msg> {
//! Command::perform(|| {
//!     Msg::Reminded(
//!         SystemNotification::new("Water the plants")
//!             .body("Due today")
//!             .button("done", "Done")
//!             .button("snooze", "Later")
//!             .show(),
//!     )
//! })
//! # }
//! ```
//!
//! A notification is for the person who is not looking at the application: Windows shows no
//! banner for the application whose window is in front.
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
//! - **Android**: the system's notifications, under the application's own name and icon, in a
//!   channel the person can tune in the settings (milestone 703). A click opens the application.
//!   From Android 13 the person must allow notifications: the first one asks them, and the
//!   application's manifest must ask for `android.permission.POST_NOTIFICATIONS` — for
//!   cargo-apk, `[[package.metadata.android.uses_permission]]` with that name — or the system
//!   refuses without asking, and `show` says why. A notification stays in the shade until the
//!   person does something with it, so `show` may wait a long time there.
//! - **The Web**: the browser's notifications, through [`show_async`](SystemNotification::show_async)
//!   — a page cannot block (milestone 704). The first asks the person for the page's permission;
//!   some browsers only put the question after a click of theirs, so show the first from a press.
//!   A page's notification has no buttons and no picture (those need a service worker): it is
//!   shown without them, with the tab's icon. A click focuses the page.
//!
//! Behind the `notifications` feature, on the desktop, Android and the Web.

use std::fmt;
use std::path::PathBuf;
#[cfg(desktop)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(desktop)]
use std::sync::Mutex;

/// A notification from the application, described: its title, the text under it, its buttons,
/// a picture and whether it makes a sound. Shown by [`show`](Self::show), from a background
/// thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemNotification {
    pub(crate) title: String,
    pub(crate) body: String,
    /// `(id, label)`, in order.
    pub(crate) buttons: Vec<(String, String)>,
    pub(crate) image: Option<PathBuf>,
    pub(crate) silent: bool,
}

/// What the person did with a [`SystemNotification`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationResponse {
    /// They clicked the notification itself. The application's window has been brought
    /// forward.
    Clicked,
    /// They pressed one of its [buttons](SystemNotification::button): its id. The window is left
    /// where it is — a button such as *Snooze* is answered without coming back — and the
    /// application brings itself forward when the button says to.
    Button(String),
    /// It went away without a click: dismissed, or taken off the screen by the system.
    Closed,
}

/// Why a [`SystemNotification`] could not be shown: the system refused it, there is no
/// notification server to show it, or it was asked for on the interface's thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationError(pub(crate) String);

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
            ..Self::default()
        }
    }

    /// The text under the title.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Adds a button labelled `label`, after any added before; a press answers
    /// [`NotificationResponse::Button`] with its `id`.
    ///
    /// Windows shows up to five side by side, Android three, Linux as many as the desktop's
    /// notification server offers, and macOS one: with several, a menu of them on its single
    /// button.
    pub fn button(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.buttons.push((id.into(), label.into()));
        self
    }

    /// A picture shown in the notification: the full path of a PNG or JPEG file. Windows shows
    /// it under the text, macOS beside it, and Linux where its notification server puts one.
    pub fn image(mut self, path: impl Into<PathBuf>) -> Self {
        self.image = Some(path.into());
        self
    }

    /// Shows it **without a sound**. By default a notification makes the system's own sound for
    /// one, as every other application's does.
    pub fn silent(mut self) -> Self {
        self.silent = true;
        self
    }

    /// Shows the notification and waits for its answer **without blocking**: the call for the
    /// Web, where nothing may block, and for `Command::perform_async` or a component's
    /// `spawn_async` on every platform. Natively it is [`show`](Self::show), moved to a thread
    /// made for waiting.
    pub async fn show_async(self) -> Result<NotificationResponse, NotificationError> {
        #[cfg(web)]
        {
            log::debug!("notification {:?}: showing", self.title);
            let answer = crate::notification_web::show(&self).await;
            log::debug!("notification {:?}: {answer:?}", self.title);
            answer
        }
        #[cfg(not(web))]
        {
            blocking::unblock(move || self.show()).await
        }
    }

    /// Shows the notification and **waits** until the person clicks it or it goes away. A
    /// click also brings the application's window forward. Not on the Web, where nothing may
    /// block: [`show_async`](Self::show_async) is the call there.
    #[cfg(not(web))]
    pub fn show(self) -> Result<NotificationResponse, NotificationError> {
        if crate::interface_thread::current() {
            let error = "SystemNotification::show was called on the interface's thread, which \
                         it would stop: call it from Command::perform, or from spawn in a \
                         component";
            log::error!("{error}");
            return Err(NotificationError(error.to_owned()));
        }
        log::debug!("notification {:?}: showing", self.title);
        #[cfg(android)]
        let answer = crate::android_notification::show(&self)?;
        #[cfg(desktop)]
        let answer = self.show_desktop()?;
        log::debug!("notification {:?}: {answer:?}", self.title);
        Ok(answer)
    }

    /// Shows it through the desktop's library, and brings the window forward on a click.
    #[cfg(desktop)]
    fn show_desktop(&self) -> Result<NotificationResponse, NotificationError> {
        let sender = SENDER.lock().ok().and_then(|sender| sender.clone());
        let mut native = notify_rust::Notification::new();
        native.summary(&self.title).body(&self.body);
        for (id, label) in &self.buttons {
            native.action(&format!("{BUTTON}{id}"), label);
        }
        if let Some(image) = &self.image {
            native.image_path(&image.to_string_lossy());
        }
        if let Some(sound) = sound(self.silent) {
            native.sound_name(sound);
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        if self.silent {
            native.hint(notify_rust::Hint::SuppressSound(true));
        }
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
        let mut answer = NotificationResponse::Closed;
        native
            .show()
            .map_err(error)?
            .wait_for_response(|platform: &notify_rust::NotificationResponse| {
                answer = response(platform);
            })
            .map_err(error)?;
        if answer == NotificationResponse::Clicked {
            RAISE.store(true, Ordering::Relaxed);
        }
        Ok(answer)
    }
}

#[cfg_attr(web, allow(dead_code))]
/// What a button's id is sent as: marked, so that no id an application chooses can be taken for
/// the click on the notification itself, which Linux names `default`.
const BUTTON: &str = "button:";

/// The button an action names, when it is a marked one.
#[cfg_attr(web, allow(dead_code))]
pub(crate) fn button(action: &str) -> Option<NotificationResponse> {
    action
        .strip_prefix(BUTTON)
        .map(|id| NotificationResponse::Button(id.to_owned()))
}

/// What the person did with a notification that left the shade, from what was recorded for it:
/// a button, a dismissal, or — nothing recorded — a click, which is the one answer Android
/// sends no word of.
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) fn left_the_shade(recorded: Option<&str>) -> NotificationResponse {
    match recorded {
        None => NotificationResponse::Clicked,
        Some(action) => button(action).unwrap_or(NotificationResponse::Closed),
    }
}

/// What the person did, from what the system reported: a marked action is a button, a close is a
/// close, and anything else a click on the notification.
#[cfg(desktop)]
fn response(platform: &notify_rust::NotificationResponse) -> NotificationResponse {
    use notify_rust::NotificationResponse as Platform;
    match platform {
        Platform::Closed(_) => NotificationResponse::Closed,
        Platform::Action(action) => button(action).unwrap_or(NotificationResponse::Clicked),
        _ => NotificationResponse::Clicked,
    }
}

/// The sound named for a notification: the system's own, or none when it is `silent`. Linux
/// names none either way — its notification server plays its own, and is told to keep quiet by
/// a hint — while left unnamed, a Windows toast is silent and a macOS one too.
#[cfg(desktop)]
fn sound(silent: bool) -> Option<&'static str> {
    if silent || cfg!(not(any(windows, target_os = "macos"))) {
        None
    } else if cfg!(windows) {
        Some("Default")
    } else {
        Some("NSUserNotificationDefaultSoundName")
    }
}

/// Who sends the notifications: the application, as its window presents it.
#[cfg(desktop)]
#[derive(Clone, Debug)]
struct Sender {
    /// The window's title.
    name: String,
    /// The application's icon, as a PNG.
    icon: Option<Vec<u8>>,
}

#[cfg(desktop)]
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

#[cfg(desktop)]
static SENDER: Mutex<Option<Sender>> = Mutex::new(None);

/// A click asked for the window to come forward, which the interface's thread does when the
/// click's message arrives. On Android the system opens the application itself.
#[cfg(desktop)]
static RAISE: AtomicBool = AtomicBool::new(false);

/// The application's window exists: notifications are sent under its title and icon.
#[cfg(desktop)]
pub(crate) fn window_made(title: &str, icon: Option<&[u8]>) {
    if let Ok(mut slot) = SENDER.lock() {
        *slot = Some(Sender {
            name: title.to_owned(),
            icon: icon.map(<[u8]>::to_vec),
        });
    }
}

/// Whether a click asked for the window to come forward since the last call.
#[cfg(desktop)]
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

#[cfg(all(test, desktop))]
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

    /// **Buttons keep their order, and the picture and the silence are kept.**
    #[test]
    fn buttons_picture_and_silence_are_kept() {
        let notification = SystemNotification::new("Water the plants")
            .button("done", "Done")
            .button("snooze", "Later")
            .image("/pictures/plant.png")
            .silent();
        assert_eq!(
            notification.buttons,
            vec![
                ("done".to_owned(), "Done".to_owned()),
                ("snooze".to_owned(), "Later".to_owned()),
            ]
        );
        assert_eq!(
            notification.image,
            Some(PathBuf::from("/pictures/plant.png"))
        );
        assert!(notification.silent);
        let plain = SystemNotification::new("Done");
        assert!(plain.buttons.is_empty() && plain.image.is_none() && !plain.silent);
    }

    /// **A close is a close, a marked action a button, and anything else a click**: the
    /// notification itself, Linux's `default` action and a reply are all the person acting on
    /// the notification.
    #[test]
    fn what_the_system_reports_is_read() {
        use notify_rust::{CloseReason, NotificationResponse as Platform};
        assert_eq!(response(&Platform::Default), NotificationResponse::Clicked);
        assert_eq!(
            response(&Platform::Action("default".into())),
            NotificationResponse::Clicked
        );
        assert_eq!(
            response(&Platform::Action(format!("{BUTTON}snooze"))),
            NotificationResponse::Button("snooze".into())
        );
        // An id that looks like the click on the notification is still the button's.
        assert_eq!(
            response(&Platform::Action(format!("{BUTTON}default"))),
            NotificationResponse::Button("default".into())
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

    /// **What Android recorded is read**: nothing is a click, `closed` a dismissal, a marked
    /// id a button, and anything else a dismissal rather than a click it was not.
    #[test]
    fn what_left_the_shade_is_read() {
        assert_eq!(left_the_shade(None), NotificationResponse::Clicked);
        assert_eq!(left_the_shade(Some("closed")), NotificationResponse::Closed);
        assert_eq!(
            left_the_shade(Some(&format!("{BUTTON}done"))),
            NotificationResponse::Button("done".into())
        );
        assert_eq!(left_the_shade(Some("other")), NotificationResponse::Closed);
    }

    /// **A notification makes the system's sound unless it is silent**: named on Windows and
    /// macOS, where an unnamed one is silent, and left to the server on Linux.
    #[test]
    fn the_systems_sound_unless_silent() {
        assert_eq!(sound(true), None);
        let expected = if cfg!(windows) {
            Some("Default")
        } else if cfg!(target_os = "macos") {
            Some("NSUserNotificationDefaultSoundName")
        } else {
            None
        };
        assert_eq!(sound(false), expected);
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
