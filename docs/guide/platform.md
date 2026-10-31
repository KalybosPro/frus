# The system around the application

An application lives among others: it opens the files the person picks, tells them something
when they are looking elsewhere, sits in the tray, answers a shortcut pressed in another
window. frus reaches each of these through the system's own — the dialogs, notifications and
icons the person already knows — behind one API on every platform that has them.

Most are optional features, so an application that does not use them does not carry them:

```toml
[dependencies]
frus = { version = "0.5", features = ["file-dialogs", "notifications", "tray", "taskbar", "global-shortcuts"] }
```

| Service | Feature | Desktop | Android | Web |
| --- | --- | --- | --- | --- |
| [File dialogs](#picking-a-file) | `file-dialogs` | ✓ | — | — |
| [Files dropped on the window](#files-dropped-on-the-window) | — | ✓ | — | — |
| [Notifications](#notifications) | `notifications` | ✓ | ✓ | ✓ |
| [Tray icon](#the-tray) | `tray` | ✓ | — | — |
| [Taskbar badge and progress](#the-taskbar-badge-and-progress) | `taskbar` | ✓ | — | — |
| [Global shortcuts](#global-shortcuts) | `global-shortcuts` | ✓ | — | — |
| [The application's icon](#the-applications-icon) | — | ✓ | ✓ | ✓ |
| [One instance at a time](#one-instance-at-a-time) | — | ✓ | — | — |

## Picking a file

`FileDialog` describes the system's dialog — a title, filters, a starting folder — and
`pick_file`, `pick_files`, `pick_folder` and `save_file` show it. They **wait** for the person,
so they run off the interface's thread: `host::spawn(work, then)` runs the dialog on another
thread and hands its answer to `then` on the interface's.

```rust
use std::path::PathBuf;

use frus::{component, host, Button, Column, FileDialog, Text, Widget};

fn open_picture() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let picked = cx.use_state(|| None::<PathBuf>);
        let store = picked.clone();
        let open = Button::new("Open a picture").on_press(move || {
            let store = store.clone();
            host::spawn(
                || {
                    FileDialog::new()
                        .title("Open a picture")
                        .filter("Images", &["png", "jpg", "jpeg"])
                        .pick_file()
                },
                move |path| store.set(path),
            );
        });
        let shown = match picked.get() {
            Some(path) => path.display().to_string(),
            None => "Nothing open".to_string(),
        };
        Box::new(Column::new().spacing(12.0).child(open).child(Text::new(shown)))
    }))
}
```

Asked for on the interface's thread, a dialog is refused — it answers as if cancelled and logs
where to call it from — because showing it there would stop every frame until it closed.

## Files dropped on the window

A `DragTarget` with `on_files` takes the files dragged onto the window from the system's file
manager, and lights up while they are held over it:

```rust
use std::path::PathBuf;

use frus::{component, Container, DragTarget, Text, Widget};

fn attachments() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let files = cx.use_state(Vec::<PathBuf>::new);
        let add = files.clone();
        Box::new(
            DragTarget::new(
                Container::new()
                    .padding(24.0)
                    .child(Text::new(format!("{} files — drop more here", files.get().len()))),
            )
            .on_files(move |paths: Vec<PathBuf>| add.update(|all| all.extend(paths)))
            .accepts_files(|paths| paths.iter().all(|p| p.extension().is_some_and(|e| e == "pdf"))),
        )
    }))
}
```

## Notifications

`SystemNotification` is a message the system shows outside the window — a toast on Windows,
the notification centre on macOS, the desktop's server on Linux, the shade on Android, the
browser's on the web — with buttons and a picture where the system allows them. `show_async`
waits for what the person does without blocking, on every platform, so an `async` handler
says it in order:

```rust
use frus::{component, Button, NotificationResponse, SystemNotification, Widget};

fn reminder() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let remind = cx.async_callback(|| async {
            let answer = SystemNotification::new("Water the plants")
                .body("Due today")
                .button("done", "Done")
                .button("snooze", "Later")
                .show_async()
                .await;
            match answer {
                Ok(NotificationResponse::Button(id)) if id == "snooze" => { /* ask again later */ }
                Ok(_) => {}
                Err(error) => frus::log::warn!("no notification: {error}"),
            }
        });
        Box::new(Button::new("Remind me").on_press(remind))
    }))
}
```

On Android 13 and later the person must allow notifications, and the application's manifest
must ask for `android.permission.POST_NOTIFICATIONS`. A browser asks for permission the first
time, and some only after a click — show the first notification from a press.

## The tray

An application describes its tray icon from its state, like its title: what the menu holds,
and whether closing the window keeps the application running there. `show_item` and
`quit_item` are the two entries every tray menu has, with nothing to write:

```rust
use frus::{text, Callback, FrusApp, Tray};

fn app() -> FrusApp {
    FrusApp::from_fn(|_| Box::new(text("Tasks")))
        .title("Tasks")
        .tray(|| {
            Tray::new("Tasks")
                .item("New task", Callback::new(|| frus::log::info!("new task")))
                .separator()
                .show_item("Open Tasks")
                .quit_item("Quit")
                .keep_running()
        })
}
```

## The taskbar badge and progress

A count on the taskbar button or the Dock icon, and the progress of something long, seen from
any other window. Also described from the state; the shell changes the system's button only
when the description changes:

```rust
use frus::{text, FrusApp, Taskbar};

fn app(unread: u32, export: Option<f32>) -> FrusApp {
    FrusApp::from_fn(|_| Box::new(text("Mail")))
        .taskbar(move || match export {
            Some(done) => Taskbar::new().count(unread).progress(done),
            None => Taskbar::new().count(unread),
        })
}
```

## Global shortcuts

A key combination that reaches the application whatever window has the keyboard — a quick-add,
a push-to-talk. `Primary` is `Cmd` on macOS and `Ctrl` elsewhere:

```rust
use frus::{text, Callback, FrusApp, GlobalShortcut};

fn app() -> FrusApp {
    FrusApp::from_fn(|_| Box::new(text("Notes")))
        .global_shortcuts(|| {
            vec![
                GlobalShortcut::new("Primary+Shift+Space", Callback::new(|| frus::log::info!("quick add"))),
                GlobalShortcut::show_window("Primary+Alt+N"),
            ]
        })
}
```

A combination another application holds cannot be taken: the system refuses it, the log says
which, and the application keeps its other shortcuts. A bare key, or `Shift` and a key, is
refused too — it would take typing away from every other application. Windows, macOS and
Linux under X11; Wayland lets no application take a combination for itself.

## The application's icon

Every frus application has the frus logo until it names its own, in the title bar, the taskbar
and Alt+Tab on the desktop. `AppIcon::from_png` takes the application's, and `AppIcon::none()`
leaves the system's default:

```rust
use frus::{text, AppIcon, FrusApp};

static ICON: &[u8] = &[]; // `include_bytes!("../assets/icon.png")` in an application

fn app() -> FrusApp {
    FrusApp::from_fn(|_| Box::new(text("Tides"))).icon(AppIcon::from_png(ICON))
}
```

On Android the launcher's icon comes from the package's resources, and on the web from the
page's `<link rel="icon">`; the [getting-started page](../getting-started.md#the-applications-icon)
says where each one goes.

## One instance at a time

`FrusApp::single_instance("com.example.notes")` makes a second launch hand the link it was
opened with to the instance already running, instead of opening a second window. With [links](navigation.md) it is how `myapp://notes/42`
opens a note in the application already open.
