//! **The taskbar's badge and progress** (milestone 708, part of #219): a count on the
//! application's taskbar button or Dock icon, and the progress of something long — a download,
//! an export — on that same button, where the person sees it from any other window.
//!
//! Declared from the state, like the title and the tray: [`Application::taskbar`](crate::Application::taskbar),
//! or `FrusApp::taskbar` for an application of components. The shell changes the system's
//! button only when the description changes.
//!
//! ```
//! # use frus_shell::Taskbar;
//! let downloading = Taskbar::new().count(3).progress(0.4);
//! let waiting = Taskbar::new().indeterminate();
//! let nothing = Taskbar::new();
//! # let _ = (downloading, waiting, nothing);
//! ```
//!
//! - **Windows**: the progress is the taskbar button's own bar — green, yellow when paused, red
//!   for an error, moving when indeterminate. The count is an **overlay** on the button's icon:
//!   a red disc with the number, drawn here, as the system draws none.
//! - **macOS**: the count is the Dock icon's badge. The Dock has no progress bar of its own; it
//!   is not shown there.
//! - **Linux**: both go to the desktop's launcher — the dock or panel that shows the
//!   application — through the `com.canonical.Unity.LauncherEntry` signal that KDE, Ubuntu's
//!   dock and others read. The launcher finds the application by its desktop entry, named
//!   after the program (`<program>.desktop`) unless [`Taskbar::desktop_entry`] says otherwise.
//!
//! Behind the `taskbar` feature, on the desktop.

// Without the system's taskbar the description is read by nobody, and the overlay's drawing
// is Windows' alone; all of it is tested on the host either way.
#![cfg_attr(not(all(windows, feature = "taskbar")), allow(dead_code))]

/// The progress shown on the taskbar button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskbarProgress {
    /// Something is under way, and how far is not known.
    Indeterminate,
    /// This far along, from 0 to 1.
    Normal(f32),
    /// Stopped at this point, to go on later.
    Paused(f32),
    /// Stopped at this point by an error.
    Error(f32),
}

impl TaskbarProgress {
    /// The fraction it shows, kept within 0 and 1; `None` for an indeterminate progress.
    pub(crate) fn fraction(self) -> Option<f32> {
        match self {
            Self::Indeterminate => None,
            Self::Normal(f) | Self::Paused(f) | Self::Error(f) => {
                Some(if f.is_nan() { 0.0 } else { f.clamp(0.0, 1.0) })
            }
        }
    }
}

/// What the application's taskbar button shows: a count, a progress, or neither.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Taskbar {
    pub(crate) count: u32,
    pub(crate) progress: Option<TaskbarProgress>,
    pub(crate) desktop_entry: Option<String>,
}

impl Taskbar {
    /// A taskbar button showing nothing of the application's.
    pub fn new() -> Self {
        Self::default()
    }

    /// A count on the button: unread messages, waiting tasks. `0` shows none, so a count
    /// straight out of a model needs no `if`.
    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// A progress `fraction` of the way, from 0 to 1.
    pub fn progress(mut self, fraction: f32) -> Self {
        self.progress = Some(TaskbarProgress::Normal(fraction));
        self
    }

    /// Something under way, how far not known.
    pub fn indeterminate(mut self) -> Self {
        self.progress = Some(TaskbarProgress::Indeterminate);
        self
    }

    /// A progress stopped at `fraction`, to go on later.
    pub fn paused(mut self, fraction: f32) -> Self {
        self.progress = Some(TaskbarProgress::Paused(fraction));
        self
    }

    /// A progress stopped at `fraction` by an error.
    pub fn error(mut self, fraction: f32) -> Self {
        self.progress = Some(TaskbarProgress::Error(fraction));
        self
    }

    /// Linux: the desktop entry the launcher knows the application by, without `.desktop`
    /// (`org.example.Tasks`); the program's name otherwise.
    pub fn desktop_entry(mut self, name: impl Into<String>) -> Self {
        self.desktop_entry = Some(name.into());
        self
    }
}

/// What a count reads as on a badge too small for more: the number up to 99, then `99+`.
pub(crate) fn count_text(count: u32) -> Option<String> {
    match count {
        0 => None,
        1..=99 => Some(count.to_string()),
        _ => Some("99+".to_string()),
    }
}

/// The digits and the plus sign, three pixels wide and five tall, a row to a `u8` (the three
/// low bits, the leftmost the highest).
fn glyph(c: char) -> Option<[u8; 5]> {
    Some(match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        _ => return None,
    })
}

/// The side of the overlay badge drawn for Windows, in pixels: the size the taskbar asks an
/// overlay to be at twice the base density, which it scales down cleanly.
pub(crate) const BADGE_SIDE: u32 = 32;

/// The overlay badge for `text`: a red disc, the text in white over it, as `BADGE_SIDE`²
/// straight (not premultiplied) RGBA. The red is the reference scheme's error, the colour a
/// badge has everywhere else in frus.
pub(crate) fn badge_rgba(text: &str) -> Vec<u8> {
    let side = BADGE_SIDE as i32;
    let mut rgba = vec![0u8; (side * side * 4) as usize];
    let centre = (side as f32 - 1.0) / 2.0;
    let radius = side as f32 / 2.0;
    for y in 0..side {
        for x in 0..side {
            let dx = x as f32 - centre;
            let dy = y as f32 - centre;
            // The disc's edge softened over a pixel, so it is not jagged when scaled down.
            let coverage = (radius - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let at = ((y * side + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[0xB3, 0x26, 0x1E, (coverage * 255.0) as u8]);
        }
    }
    let glyphs: Vec<[u8; 5]> = text.chars().filter_map(glyph).collect();
    if glyphs.is_empty() {
        return rgba;
    }
    // The biggest whole scale at which the text, a pixel apart, fits across the disc's middle.
    let columns = glyphs.len() as i32 * 4 - 1;
    let scale = ((side - 8) / columns).clamp(1, 4);
    let (width, height) = (columns * scale, 5 * scale);
    let (left, top) = ((side - width) / 2, (side - height) / 2);
    for (i, rows) in glyphs.iter().enumerate() {
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..3 {
                if bits & (0b100 >> column) == 0 {
                    continue;
                }
                for py in 0..scale {
                    for px in 0..scale {
                        let x = left + (i as i32 * 4 + column) * scale + px;
                        let y = top + row as i32 * scale + py;
                        let at = ((y * side + x) * 4) as usize;
                        rgba[at..at + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
                    }
                }
            }
        }
    }
    rgba
}

/// The taskbar as the system has it (desktop, `taskbar` feature).
#[cfg(all(desktop, feature = "taskbar"))]
pub(crate) mod native {
    use super::Taskbar;

    /// Makes the system show `wanted` on `window`'s button, where it differs from `shown`;
    /// what is shown afterwards. A system that would not take it — the button not made yet —
    /// leaves `shown` as it was, to be tried again at the next change.
    pub(crate) fn apply(
        window: &winit::window::Window,
        shown: &Taskbar,
        wanted: &Taskbar,
    ) -> Taskbar {
        if shown == wanted {
            return wanted.clone();
        }
        match imp::apply(window, shown, wanted) {
            Ok(()) => wanted.clone(),
            Err(err) => {
                log::debug!("taskbar: not applied yet ({err})");
                shown.clone()
            }
        }
    }

    #[cfg(windows)]
    mod imp {
        use std::cell::RefCell;

        use windows::core::HSTRING;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{
            CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS,
        };
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
        use windows::Win32::UI::Shell::{
            ITaskbarList3, TaskbarList, TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS,
            TBPF_NORMAL, TBPF_PAUSED,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateIconIndirect, DestroyIcon, HICON, ICONINFO,
        };
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        use super::super::{badge_rgba, count_text, TaskbarProgress, BADGE_SIDE};
        use super::Taskbar;

        thread_local! {
            /// The shell's taskbar object, made once on the interface's thread.
            static LIST: RefCell<Option<ITaskbarList3>> = const { RefCell::new(None) };
        }

        fn list() -> windows::core::Result<ITaskbarList3> {
            LIST.with(|slot| {
                if let Some(list) = slot.borrow().as_ref() {
                    return Ok(list.clone());
                }
                // SAFETY: COM is initialised on this thread by the event loop (winit takes the
                // single-threaded apartment for drag and drop); the object is used here only.
                let list: ITaskbarList3 =
                    unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)? };
                unsafe { list.HrInit()? };
                *slot.borrow_mut() = Some(list.clone());
                Ok(list)
            })
        }

        fn hwnd(window: &winit::window::Window) -> Option<HWND> {
            match window.window_handle().ok()?.as_raw() {
                RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut _)),
                _ => None,
            }
        }

        pub(super) fn apply(
            window: &winit::window::Window,
            shown: &Taskbar,
            wanted: &Taskbar,
        ) -> windows::core::Result<()> {
            let Some(hwnd) = hwnd(window) else {
                return Ok(());
            };
            let list = list()?;
            if shown.progress != wanted.progress {
                let state = match wanted.progress {
                    None => TBPF_NOPROGRESS,
                    Some(TaskbarProgress::Indeterminate) => TBPF_INDETERMINATE,
                    Some(TaskbarProgress::Normal(_)) => TBPF_NORMAL,
                    Some(TaskbarProgress::Paused(_)) => TBPF_PAUSED,
                    Some(TaskbarProgress::Error(_)) => TBPF_ERROR,
                };
                // SAFETY: a valid window of this process, on its own thread.
                unsafe { list.SetProgressState(hwnd, state)? };
                if let Some(fraction) = wanted.progress.and_then(TaskbarProgress::fraction) {
                    unsafe { list.SetProgressValue(hwnd, (fraction * 1000.0) as u64, 1000)? };
                }
            }
            if count_text(shown.count) != count_text(wanted.count) {
                match count_text(wanted.count) {
                    None => unsafe { list.SetOverlayIcon(hwnd, HICON::default(), None)? },
                    Some(text) => {
                        let icon = icon(&badge_rgba(&text))?;
                        let said = HSTRING::from(text.as_str());
                        let set = unsafe { list.SetOverlayIcon(hwnd, icon, &said) };
                        // The taskbar keeps its own copy.
                        unsafe {
                            let _ = DestroyIcon(icon);
                        }
                        set?;
                    }
                }
            }
            Ok(())
        }

        /// A `BADGE_SIDE`² icon from straight RGBA.
        fn icon(rgba: &[u8]) -> windows::core::Result<HICON> {
            let side = BADGE_SIDE as i32;
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: side,
                    // Negative: the rows top to bottom, as the RGBA is.
                    biHeight: -side,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            // SAFETY: the DIB's memory is `side`² BGRA pixels, written once below; the bitmaps
            // are deleted once the icon has been made from them.
            unsafe {
                let colour = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
                let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, rgba.len());
                for (to, from) in pixels.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
                    to.copy_from_slice(&[from[2], from[1], from[0], from[3]]);
                }
                let mask = CreateBitmap(side, side, 1, 1, None);
                let made = CreateIconIndirect(&ICONINFO {
                    fIcon: true.into(),
                    xHotspot: 0,
                    yHotspot: 0,
                    hbmMask: mask,
                    hbmColor: colour,
                });
                let _ = DeleteObject(colour.into());
                let _ = DeleteObject(mask.into());
                made
            }
        }
    }

    #[cfg(target_os = "macos")]
    mod imp {
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSApplication;
        use objc2_foundation::NSString;

        use super::super::count_text;
        use super::Taskbar;

        pub(super) fn apply(
            _window: &winit::window::Window,
            _shown: &Taskbar,
            wanted: &Taskbar,
        ) -> Result<(), &'static str> {
            let main = MainThreadMarker::new().ok_or("not on the main thread")?;
            let tile = NSApplication::sharedApplication(main).dockTile();
            let label = count_text(wanted.count).map(|text| NSString::from_str(&text));
            tile.setBadgeLabel(label.as_deref());
            Ok(())
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    mod imp {
        use std::collections::HashMap;
        use std::sync::OnceLock;

        use zbus::blocking::Connection;
        use zbus::zvariant::Value;

        use super::super::TaskbarProgress;
        use super::Taskbar;

        fn session() -> Option<&'static Connection> {
            static SESSION: OnceLock<Option<Connection>> = OnceLock::new();
            SESSION
                .get_or_init(|| {
                    Connection::session()
                        .map_err(|err| log::warn!("taskbar: no D-Bus session ({err})"))
                        .ok()
                })
                .as_ref()
        }

        /// The desktop entry the launcher knows the application by.
        fn entry(wanted: &Taskbar) -> String {
            wanted.desktop_entry.clone().unwrap_or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|exe| Some(exe.file_stem()?.to_string_lossy().into_owned()))
                    .unwrap_or_default()
            })
        }

        pub(super) fn apply(
            _window: &winit::window::Window,
            _shown: &Taskbar,
            wanted: &Taskbar,
        ) -> zbus::Result<()> {
            let Some(session) = session() else {
                return Ok(());
            };
            let mut properties: HashMap<&str, Value> = HashMap::new();
            properties.insert("count", Value::from(i64::from(wanted.count)));
            properties.insert("count-visible", Value::from(wanted.count > 0));
            let fraction = wanted.progress.and_then(TaskbarProgress::fraction);
            properties.insert("progress", Value::from(f64::from(fraction.unwrap_or(0.0))));
            properties.insert("progress-visible", Value::from(fraction.is_some()));
            let uri = format!("application://{}.desktop", entry(wanted));
            session.emit_signal(
                None::<&str>,
                "/com/frus/LauncherEntry",
                "com.canonical.Unity.LauncherEntry",
                "Update",
                &(uri, properties),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The description keeps what it is told**, the last progress winning.
    #[test]
    fn the_description_is_kept() {
        let bar = Taskbar::new().count(3).progress(0.4);
        assert_eq!(bar.count, 3);
        assert_eq!(bar.progress, Some(TaskbarProgress::Normal(0.4)));
        assert_eq!(
            Taskbar::new().indeterminate().progress,
            Some(TaskbarProgress::Indeterminate)
        );
        assert_eq!(
            Taskbar::new().progress(0.2).paused(0.5).progress,
            Some(TaskbarProgress::Paused(0.5))
        );
        assert_eq!(
            Taskbar::new().error(1.0).progress,
            Some(TaskbarProgress::Error(1.0))
        );
        assert_eq!(Taskbar::new(), Taskbar::default());
    }

    /// **A fraction is kept within 0 and 1**, and an indeterminate progress has none.
    #[test]
    fn a_fraction_is_kept_within_bounds() {
        assert_eq!(TaskbarProgress::Normal(1.5).fraction(), Some(1.0));
        assert_eq!(TaskbarProgress::Paused(-0.5).fraction(), Some(0.0));
        assert_eq!(TaskbarProgress::Error(f32::NAN).fraction(), Some(0.0));
        assert_eq!(TaskbarProgress::Normal(0.25).fraction(), Some(0.25));
        assert_eq!(TaskbarProgress::Indeterminate.fraction(), None);
    }

    /// **A count reads as the number, up to `99+`**, and `0` as nothing.
    #[test]
    fn a_count_reads_up_to_99_plus() {
        assert_eq!(count_text(0), None);
        assert_eq!(count_text(7).as_deref(), Some("7"));
        assert_eq!(count_text(99).as_deref(), Some("99"));
        assert_eq!(count_text(100).as_deref(), Some("99+"));
    }

    fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let at = ((y * BADGE_SIDE + x) * 4) as usize;
        [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
    }

    /// **The overlay is a red disc with white text in its middle**: transparent at the corners,
    /// the error red inside, and white pixels where the digits are — more of them for `99+`
    /// than for `7`, and within the disc.
    #[test]
    fn the_overlay_is_a_red_disc_with_the_text() {
        let seven = badge_rgba("7");
        assert_eq!(seven.len(), (BADGE_SIDE * BADGE_SIDE * 4) as usize);
        assert_eq!(pixel(&seven, 0, 0)[3], 0, "a corner is outside the disc");
        assert_eq!(
            pixel(&seven, 3, 16),
            [0xB3, 0x26, 0x1E, 0xFF],
            "the red inside"
        );
        let white = |rgba: &[u8]| {
            rgba.chunks_exact(4)
                .filter(|p| *p == [0xFF, 0xFF, 0xFF, 0xFF])
                .count()
        };
        assert!(white(&seven) > 0, "the digit is drawn");
        assert!(white(&badge_rgba("99+")) > white(&seven));
        assert_eq!(white(&badge_rgba("")), 0);
        for (i, p) in badge_rgba("99+").chunks_exact(4).enumerate() {
            if p == [0xFF, 0xFF, 0xFF, 0xFF] {
                let (x, y) = (i as u32 % BADGE_SIDE, i as u32 / BADGE_SIDE);
                assert!(x > 2 && x < BADGE_SIDE - 3 && y > 2 && y < BADGE_SIDE - 3);
            }
        }
    }
}
