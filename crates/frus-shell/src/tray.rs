//! **The tray icon** (milestone 706, part of #219): the application's icon in the system's
//! notification area — the Windows taskbar's corner, the macOS menu bar, a Linux desktop's
//! status area — with a menu, and the choice of running on there once the window is closed.
//!
//! An application describes its tray from its state, as it does its title and its
//! subscriptions: [`Application::tray`](crate::Application::tray), or `FrusApp::tray` for an
//! application of components. The shell builds it, changes it when the description changes, and
//! takes it away when there is none:
//!
//! ```
//! # use frus_shell::Tray;
//! # #[derive(Clone)] enum Msg { NewTask }
//! let tray: Tray<Msg> = Tray::new("Tasks")
//!     .item("New task", Msg::NewTask)
//!     .separator()
//!     .show_item("Open Tasks")
//!     .quit_item("Quit")
//!     .keep_running();
//! ```
//!
//! - A **click** on the icon brings the window forward, unless [`on_click`](Tray::on_click)
//!   says otherwise; the menu opens on the other button (on macOS, on the click itself, as the
//!   menu bar does).
//! - An **item** sends its message; [`show_item`](Tray::show_item) and
//!   [`quit_item`](Tray::quit_item) do what a tray's menu always offers, with no message to
//!   write.
//! - [`keep_running`](Tray::keep_running): closing the window **hides** it, and the
//!   application goes on in the tray until a quit item — what a chat client or a sync tool does.
//!
//! Behind the `tray` feature, on the desktop. On Linux the icon is the desktop's status area,
//! reached over D-Bus; building there needs GTK 3's development files (`libgtk-3-dev`), which
//! the menus are made with.

// Without the system's tray, the description is kept but read by nobody.
#![cfg_attr(not(all(desktop, feature = "tray")), allow(dead_code))]

use crate::AppIcon;

/// What a click or an item does.
pub(crate) enum TrayAction<Msg> {
    /// Sends a message to the application.
    Message(Msg),
    /// Brings the window forward, showing it if it was hidden.
    ShowWindow,
    /// Ends the application.
    Quit,
}

/// One line of the tray's menu.
pub(crate) enum TrayEntry<Msg> {
    Item {
        label: String,
        enabled: bool,
        action: Option<TrayAction<Msg>>,
    },
    Separator,
}

/// The application's tray icon, described: its tooltip, its icon, its menu, what a click on it
/// does, and whether closing the window leaves the application running in it.
pub struct Tray<Msg> {
    pub(crate) tooltip: String,
    pub(crate) icon: Option<AppIcon>,
    pub(crate) entries: Vec<TrayEntry<Msg>>,
    pub(crate) on_click: TrayAction<Msg>,
    pub(crate) keep_running: bool,
}

/// What the person did with the tray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrayEvent {
    /// A click on the icon.
    Click,
    /// The menu's item at this place, separators counted.
    Item(usize),
}

/// What the system is shown of a tray: everything but the messages, which it never sees. Two
/// descriptions with the same shape need nothing changed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shape {
    pub(crate) tooltip: String,
    pub(crate) icon: Option<Vec<u8>>,
    /// `Some((label, enabled))` for an item, `None` for a separator.
    pub(crate) entries: Vec<Option<(String, bool)>>,
}

impl<Msg> Tray<Msg> {
    /// A tray icon showing `tooltip` when the pointer rests on it, with the application's icon
    /// and an empty menu; a click on it brings the window forward.
    pub fn new(tooltip: impl Into<String>) -> Self {
        Self {
            tooltip: tooltip.into(),
            icon: None,
            entries: Vec::new(),
            on_click: TrayAction::ShowWindow,
            keep_running: false,
        }
    }

    /// The icon, when it is not the application's own.
    pub fn icon(mut self, icon: AppIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Adds an item labelled `label` that sends `message`.
    pub fn item(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.entries.push(TrayEntry::Item {
            label: label.into(),
            enabled: true,
            action: Some(TrayAction::Message(message)),
        });
        self
    }

    /// Adds an item that is shown but cannot be chosen: a status line, or an action not
    /// available now.
    pub fn disabled_item(mut self, label: impl Into<String>) -> Self {
        self.entries.push(TrayEntry::Item {
            label: label.into(),
            enabled: false,
            action: None,
        });
        self
    }

    /// Adds an item that brings the window forward, showing it if it was hidden.
    pub fn show_item(mut self, label: impl Into<String>) -> Self {
        self.entries.push(TrayEntry::Item {
            label: label.into(),
            enabled: true,
            action: Some(TrayAction::ShowWindow),
        });
        self
    }

    /// Adds an item that ends the application.
    pub fn quit_item(mut self, label: impl Into<String>) -> Self {
        self.entries.push(TrayEntry::Item {
            label: label.into(),
            enabled: true,
            action: Some(TrayAction::Quit),
        });
        self
    }

    /// Adds a line between items.
    pub fn separator(mut self) -> Self {
        self.entries.push(TrayEntry::Separator);
        self
    }

    /// What a click on the icon sends, in place of bringing the window forward.
    pub fn on_click(mut self, message: Msg) -> Self {
        self.on_click = TrayAction::Message(message);
        self
    }

    /// Closing the window **hides** it, and the application goes on in the tray until a
    /// [`quit_item`](Self::quit_item) — or the system — ends it. Without one, closing the
    /// window ends the application as it always does.
    pub fn keep_running(mut self) -> Self {
        self.keep_running = true;
        self
    }

    /// What the system is shown of it.
    #[cfg(any(desktop, web, test))]
    pub(crate) fn shape(&self, default_icon: &AppIcon) -> Shape {
        Shape {
            tooltip: self.tooltip.clone(),
            icon: self
                .icon
                .as_ref()
                .unwrap_or(default_icon)
                .png()
                .map(<[u8]>::to_vec),
            entries: self
                .entries
                .iter()
                .map(|entry| match entry {
                    TrayEntry::Item { label, enabled, .. } => Some((label.clone(), *enabled)),
                    TrayEntry::Separator => None,
                })
                .collect(),
        }
    }

    /// What `event` does in this description: `None` for a separator, a disabled item, or a
    /// place the menu no longer has.
    pub(crate) fn action(self, event: TrayEvent) -> Option<TrayAction<Msg>> {
        match event {
            TrayEvent::Click => Some(self.on_click),
            TrayEvent::Item(at) => match self.entries.into_iter().nth(at)? {
                TrayEntry::Item { action, .. } => action,
                TrayEntry::Separator => None,
            },
        }
    }
}

/// The tray as the system has it (desktop, `tray` feature).
#[cfg(all(desktop, feature = "tray"))]
pub(crate) mod native {
    use std::sync::{Arc, Mutex, Once};

    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };
    use winit::window::Window;

    use super::{Shape, TrayEvent};

    /// The icon on the system's tray, and what it was made from.
    pub(crate) struct Live {
        icon: TrayIcon,
        shape: Shape,
    }

    /// What the person did, waiting for the event loop.
    static EVENTS: Mutex<Vec<TrayEvent>> = Mutex::new(Vec::new());

    /// The window to wake when something is waiting: a redraw asked of it brings the event
    /// loop round on every system, the tray's events arriving on a thread of their own on Linux.
    static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);

    fn push(event: TrayEvent) {
        if let Ok(mut events) = EVENTS.lock() {
            events.push(event);
        }
        if let Some(window) = WINDOW.lock().ok().and_then(|window| window.clone()) {
            window.request_redraw();
        }
    }

    /// What the person did since the last call.
    pub(crate) fn take_events() -> Vec<TrayEvent> {
        EVENTS
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }

    /// The handlers, which the library takes once for the whole run.
    fn listen() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    push(TrayEvent::Click);
                }
            }));
            MenuEvent::set_event_handler(Some(|event: MenuEvent| {
                if let Ok(at) = event.id.0.parse::<usize>() {
                    push(TrayEvent::Item(at));
                }
            }));
        });
    }

    fn menu(shape: &Shape) -> Menu {
        let menu = Menu::new();
        for (at, entry) in shape.entries.iter().enumerate() {
            let added = match entry {
                Some((label, enabled)) => {
                    menu.append(&MenuItem::with_id(at.to_string(), label, *enabled, None))
                }
                None => menu.append(&PredefinedMenuItem::separator()),
            };
            if let Err(err) = added {
                log::warn!("tray: an item could not be added to the menu ({err})");
            }
        }
        menu
    }

    fn icon(shape: &Shape) -> Option<Icon> {
        let image = frus_image::decode(shape.icon.as_deref()?)
            .map_err(|err| log::warn!("tray: the icon is not a PNG that can be read ({err})"))
            .ok()?;
        Icon::from_rgba(image.rgba().to_vec(), image.width(), image.height())
            .map_err(|err| log::warn!("tray: the icon cannot be used ({err})"))
            .ok()
    }

    /// Makes the system's tray show `shape`: built the first time, changed where it differs,
    /// untouched where it does not.
    pub(crate) fn sync(live: &mut Option<Live>, shape: Shape, window: Option<&Arc<Window>>) {
        if let Ok(mut slot) = WINDOW.lock() {
            *slot = window.cloned();
        }
        match live {
            Some(live) if live.shape == shape => {}
            Some(live) => {
                if live.shape.entries != shape.entries {
                    live.icon.set_menu(Some(Box::new(menu(&shape))));
                }
                if live.shape.tooltip != shape.tooltip {
                    let _ = live.icon.set_tooltip(Some(&shape.tooltip));
                }
                if live.shape.icon != shape.icon {
                    let _ = live.icon.set_icon(icon(&shape));
                }
                live.shape = shape;
            }
            None => {
                listen();
                let mut builder = TrayIconBuilder::new()
                    .with_tooltip(&shape.tooltip)
                    .with_menu(Box::new(menu(&shape)))
                    .with_menu_on_left_click(cfg!(target_os = "macos"));
                if let Some(icon) = icon(&shape) {
                    builder = builder.with_icon(icon);
                }
                match builder.build() {
                    Ok(icon) => *live = Some(Live { icon, shape }),
                    Err(err) => log::warn!("tray: the icon could not be made ({err})"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        New,
        Ping,
    }

    fn tray() -> Tray<Msg> {
        Tray::new("Tasks")
            .item("New task", Msg::New)
            .disabled_item("3 to do")
            .separator()
            .show_item("Open")
            .quit_item("Quit")
    }

    fn message(action: Option<TrayAction<Msg>>) -> Option<Msg> {
        match action? {
            TrayAction::Message(msg) => Some(msg),
            _ => None,
        }
    }

    /// **Each place in the menu does what it says**: an item its message, a disabled item and a
    /// separator nothing, the built-in items theirs, a place past the end nothing.
    #[test]
    fn each_place_does_what_it_says() {
        assert_eq!(message(tray().action(TrayEvent::Item(0))), Some(Msg::New));
        assert!(tray().action(TrayEvent::Item(1)).is_none(), "disabled");
        assert!(tray().action(TrayEvent::Item(2)).is_none(), "a separator");
        assert!(matches!(
            tray().action(TrayEvent::Item(3)),
            Some(TrayAction::ShowWindow)
        ));
        assert!(matches!(
            tray().action(TrayEvent::Item(4)),
            Some(TrayAction::Quit)
        ));
        assert!(tray().action(TrayEvent::Item(5)).is_none(), "past the end");
    }

    /// **A click brings the window forward, unless it is given a message.**
    #[test]
    fn a_click_shows_the_window_unless_told() {
        assert!(matches!(
            tray().action(TrayEvent::Click),
            Some(TrayAction::ShowWindow)
        ));
        assert_eq!(
            message(tray().on_click(Msg::Ping).action(TrayEvent::Click)),
            Some(Msg::Ping)
        );
    }

    /// **No system tray without a real window.** An application that declares a tray, run
    /// through the windowless driver tests use, makes none: on macOS a menu made off the main
    /// thread is a panic, and every test of such an application would be one.
    #[cfg(all(desktop, feature = "tray"))]
    #[test]
    fn no_system_tray_without_a_window() {
        let app = crate::FrusApp::from_fn(|_| Box::new(frus_widgets::Spacer::default()))
            .tray(|| Tray::new("Tasks").quit_item("Quit").keep_running());
        let mut driver = crate::testing::Driver::new(app, 200.0, 200.0);
        driver.frame(0.016);
        assert!(!driver.has_system_tray());
    }

    /// **The shape is what the system sees**: the labels, which are enabled, the separators,
    /// the tooltip and the icon — and not the messages, so two trays that differ only in what
    /// an item sends need nothing changed.
    #[test]
    fn the_shape_leaves_the_messages_out() {
        let icon = AppIcon::frus();
        let shape = tray().shape(&icon);
        assert_eq!(shape.tooltip, "Tasks");
        assert_eq!(
            shape.entries,
            vec![
                Some(("New task".into(), true)),
                Some(("3 to do".into(), false)),
                None,
                Some(("Open".into(), true)),
                Some(("Quit".into(), true)),
            ]
        );
        assert_eq!(
            shape.icon.as_deref(),
            icon.png(),
            "the application's icon by default"
        );
        let other = Tray::new("Tasks").item("New task", Msg::Ping);
        let same = Tray::new("Tasks").item("New task", Msg::New);
        assert_eq!(other.shape(&icon), same.shape(&icon));
        assert_ne!(
            Tray::<Msg>::new("Tasks").icon(AppIcon::none()).shape(&icon),
            Tray::<Msg>::new("Tasks").shape(&icon)
        );
        assert!(!tray().keep_running && tray().keep_running().keep_running);
    }
}
