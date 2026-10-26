//! **The system's file dialogs** (milestone 698, part of #219): open one file or several,
//! choose a folder, choose where to save — the dialogs the system draws itself, so that a
//! frus application asks for a file the way every other application on the machine does.
//!
//! [`FileDialog`] describes the dialog; [`FileDialog::pick_file`] and its companions show it
//! and wait for the answer. They **block** until the person answers, so they belong on a
//! background thread, where every other wait goes:
//!
//! ```no_run
//! # use std::path::PathBuf;
//! # use frus_shell::{Command, FileDialog};
//! # enum Msg { Opened(Option<PathBuf>) }
//! # fn f() -> Command<Msg> {
//! Command::perform(|| {
//!     Msg::Opened(
//!         FileDialog::new()
//!             .title("Open a picture")
//!             .filter("Images", &["png", "jpg", "jpeg"])
//!             .pick_file(),
//!     )
//! })
//! # }
//! ```
//!
//! and from a component, `frus_widgets::spawn(|| FileDialog::new().pick_file(), …)`.
//!
//! Asked for on the interface's thread, a dialog is **refused**: the call returns as if the
//! person had cancelled and logs an error that says where to call it from. Showing it there
//! would stop every frame until it closed, and on macOS, where the dialog itself must run on
//! that thread, it would never open at all.
//!
//! The dialog belongs to the application's window: on top of it, and, on macOS, a sheet
//! attached to it. Windows, macOS and Linux (through the desktop portal, the one sandboxed
//! applications use too). Behind the `file-dialogs` feature.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

use winit::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

/// A system file dialog, described: what it says, where it opens, which files it offers.
/// Shown by [`pick_file`](Self::pick_file), [`pick_files`](Self::pick_files),
/// [`pick_folder`](Self::pick_folder) or [`save_file`](Self::save_file), from a background
/// thread.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileDialog {
    title: Option<String>,
    directory: Option<PathBuf>,
    file_name: Option<String>,
    filters: Vec<(String, Vec<String>)>,
}

impl FileDialog {
    /// A dialog with the system's own title, opening where the system chooses, offering every
    /// file.
    pub fn new() -> Self {
        Self::default()
    }

    /// The dialog's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The folder the dialog opens in.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// The name the dialog suggests: what [`save_file`](Self::save_file) offers to save as.
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// Offers the files with one of these `extensions`, under `name` ("Images"). Each call
    /// adds a choice; the first is the one shown first. Extensions are written without
    /// the dot — `"png"` — and a leading `.` or `*.` is taken off.
    pub fn filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        let extensions = extensions
            .iter()
            .map(|e| e.trim_start_matches('*').trim_start_matches('.').to_owned())
            .filter(|e| !e.is_empty())
            .collect();
        self.filters.push((name.into(), extensions));
        self
    }

    /// Asks for **one file** to open: its path, or `None` when the person cancelled.
    pub fn pick_file(self) -> Option<PathBuf> {
        let dialog = self.native("pick_file")?;
        pollster::block_on(dialog.pick_file()).map(|f| f.path().to_owned())
    }

    /// Asks for **files** to open, one or several: their paths, none when the person
    /// cancelled.
    pub fn pick_files(self) -> Vec<PathBuf> {
        let Some(dialog) = self.native("pick_files") else {
            return Vec::new();
        };
        pollster::block_on(dialog.pick_files())
            .unwrap_or_default()
            .iter()
            .map(|f| f.path().to_owned())
            .collect()
    }

    /// Asks for **a folder**: its path, or `None` when the person cancelled.
    pub fn pick_folder(self) -> Option<PathBuf> {
        let dialog = self.native("pick_folder")?;
        pollster::block_on(dialog.pick_folder()).map(|f| f.path().to_owned())
    }

    /// Asks **where to save**: the path chosen, or `None` when the person cancelled. Nothing is
    /// written; the file may not exist yet, and when it does the system has asked whether to
    /// replace it.
    pub fn save_file(self) -> Option<PathBuf> {
        let dialog = self.native("save_file")?;
        pollster::block_on(dialog.save_file()).map(|f| f.path().to_owned())
    }

    /// The system's dialog, set up as described and owned by the application's window; `None`
    /// on the interface's thread, where it must not be shown.
    fn native(self, what: &str) -> Option<rfd::AsyncFileDialog> {
        if on_interface_thread() {
            log::error!(
                "FileDialog::{what} was called on the interface's thread, which it would stop: \
                 call it from Command::perform, or from spawn in a component"
            );
            return None;
        }
        let mut dialog = rfd::AsyncFileDialog::new();
        if let Some(title) = self.title {
            dialog = dialog.set_title(title);
        }
        if let Some(directory) = self.directory {
            dialog = dialog.set_directory(directory);
        }
        if let Some(name) = self.file_name {
            dialog = dialog.set_file_name(name);
        }
        for (name, extensions) in &self.filters {
            dialog = dialog.add_filter(name, extensions.as_slice());
        }
        if let Some(parent) = PARENT.lock().ok().and_then(|parent| *parent) {
            dialog = dialog.set_parent(&parent);
        }
        Some(dialog)
    }
}

/// The window the dialogs belong to, as the system knows it.
#[derive(Clone, Copy)]
struct Parent {
    window: RawWindowHandle,
    display: RawDisplayHandle,
}

// SAFETY: the handles are identifiers the system resolves, not data read through. They are
// only handed to the system's dialog, which uses them from whichever thread the system
// requires (on macOS, rfd moves the dialog to the main thread), and the window they name
// lives until the program ends.
unsafe impl Send for Parent {}

impl HasWindowHandle for Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: the window lives until the program ends (see `Send` above).
        Ok(unsafe { WindowHandle::borrow_raw(self.window) })
    }
}

impl HasDisplayHandle for Parent {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: as for the window.
        Ok(unsafe { DisplayHandle::borrow_raw(self.display) })
    }
}

static PARENT: Mutex<Option<Parent>> = Mutex::new(None);

/// The thread the event loop runs on.
static INTERFACE: OnceLock<ThreadId> = OnceLock::new();

/// The application's window exists: the dialogs will belong to it. Called on the interface's
/// thread, which is how that thread is known.
pub(crate) fn window_made(window: &winit::window::Window) {
    INTERFACE.get_or_init(|| std::thread::current().id());
    let parent = match (window.window_handle(), window.display_handle()) {
        (Ok(w), Ok(d)) => Some(Parent {
            window: w.as_raw(),
            display: d.as_raw(),
        }),
        _ => None,
    };
    if let Ok(mut slot) = PARENT.lock() {
        *slot = parent;
    }
}

fn on_interface_thread() -> bool {
    INTERFACE.get() == Some(&std::thread::current().id())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A filter keeps its extensions bare**, whichever way they were written.
    #[test]
    fn a_filter_takes_the_dot_off() {
        let dialog = FileDialog::new()
            .filter("Images", &["png", ".jpg", "*.jpeg", "", "*"])
            .filter("Text", &["txt"]);
        assert_eq!(
            dialog.filters,
            vec![
                (
                    "Images".to_owned(),
                    vec!["png".into(), "jpg".into(), "jpeg".into()]
                ),
                ("Text".to_owned(), vec!["txt".into()]),
            ]
        );
    }

    /// **The description is kept as given**: title, folder and suggested name.
    #[test]
    fn the_description_is_kept() {
        let dialog = FileDialog::new()
            .title("Export")
            .directory("/tmp")
            .file_name("notes.txt");
        assert_eq!(dialog.title.as_deref(), Some("Export"));
        assert_eq!(dialog.directory, Some(PathBuf::from("/tmp")));
        assert_eq!(dialog.file_name.as_deref(), Some("notes.txt"));
    }

    /// **On the interface's thread a dialog is refused**, as a cancel: it would stop the
    /// frames, and on macOS never open. Any other thread may show it.
    #[test]
    fn the_interface_thread_is_refused() {
        // The test's own thread plays the interface's; the guard is checked before anything
        // reaches the system, so no dialog opens.
        let me = std::thread::current().id();
        let interface = *INTERFACE.get_or_init(|| me);
        if interface != me {
            return; // another test claimed it first: nothing to check from here
        }
        assert!(on_interface_thread());
        assert_eq!(FileDialog::new().pick_file(), None);
        assert!(FileDialog::new().pick_files().is_empty());
        assert_eq!(FileDialog::new().pick_folder(), None);
        assert_eq!(FileDialog::new().save_file(), None);
        let elsewhere = std::thread::spawn(on_interface_thread).join().unwrap();
        assert!(!elsewhere, "another thread is not the interface's");
    }
}
