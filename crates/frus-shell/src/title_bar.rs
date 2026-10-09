//! **The application's content on the title bar's line**, on Windows (milestone 640).
//!
//! Windows draws a window's caption in the non-client area, where an application cannot
//! draw. The documented way to share that line ("Custom Window Frame Using DWM") keeps the
//! window the system's:
//!
//! - the frame is extended into the client area by the caption's height
//!   (`DwmExtendFrameIntoClientArea`), so the desktop compositor keeps its three buttons
//!   there and acts on them: it answers the hit test for them (`DwmDefWindowProc`),
//!   minimizes, maximizes and closes, and shows its snap layouts;
//! - `WM_NCCALCSIZE` gives the caption's height to the client, keeping the side and bottom
//!   borders, so the application's content starts at the top of the window;
//! - `WM_NCHITTEST` answers the rest of the line as the caption, so the system moves the
//!   window by it and maximizes it on a double click — except what the application has
//!   there (its menu bar's words), and the window's icon, which opens the window menu.
//!
//! The application **paints** the line: the compositor's buttons are under its content and
//! would not be seen, so frus paints them where the compositor has them
//! ([`buttons`]), in the state the compositor reports ([`buttons_state`]), in the colours
//! the system would draw its own caption in ([`caption_colors`], milestone 642).

use std::cell::RefCell;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
    DWMWA_CAPTION_BUTTON_BOUNDS,
};
use windows_sys::Win32::Graphics::Gdi::{
    ClientToScreen, GetSysColor, InvalidateRect, ScreenToClient, COLOR_ACTIVECAPTION,
    COLOR_CAPTIONTEXT, COLOR_INACTIVECAPTION, COLOR_INACTIVECAPTIONTEXT,
};
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsZoomed, SetWindowPos, SystemParametersInfoW, HTCAPTION, HTCLIENT, HTCLOSE,
    HTMAXBUTTON, HTMINBUTTON, HTSYSMENU, HTTOP, NCCALCSIZE_PARAMS, SM_CXPADDEDBORDER, SM_CYCAPTION,
    SM_CYSIZEFRAME, SPI_GETHIGHCONTRAST, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    WM_DWMCOLORIZATIONCOLORCHANGED, WM_LBUTTONUP, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDOWN,
    WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_SETTINGCHANGE, WM_THEMECHANGED,
};

use frus_widgets::Color;

/// The subclass's identity.
const SUBCLASS: usize = 640;
/// How thick the resizing edge along the top is, in physical pixels.
const RESIZE_EDGE: i32 = 6;

/// What the window procedure knows about the line, set by the shell each frame.
#[derive(Default)]
struct Line {
    /// The line's height, in physical pixels.
    height: i32,
    /// Where the application's own content on the line is, in client pixels.
    interactive: Vec<RECT>,
    /// Where the window's icon is, in client pixels.
    icon: Option<RECT>,
    /// The compositor's button the pointer is over, and the one pressed (hit-test codes).
    hovered: u32,
    pressed: u32,
    /// What the system says of its captions, read once and again when it says it changed.
    caption: Option<Caption>,
}

/// What the system says of its captions: what [`caption_colors`] is made from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Caption {
    /// High contrast's caption colours, when it is on: active and inactive, surface and ink.
    high_contrast: Option<[Color; 4]>,
    /// The accent colour, when the person asked for it on title bars, and the one for an
    /// inactive window if the system keeps one.
    accent: Option<(Color, Option<Color>)>,
}

thread_local! {
    static LINE: RefCell<Line> = RefCell::new(Line::default());
    /// Asks the shell for a frame: a repaint message alone does not reach it.
    static WAKE: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// What to call when the line's state changes and a frame is owed.
pub(crate) fn on_change(wake: impl Fn() + 'static) {
    WAKE.with(|slot| *slot.borrow_mut() = Some(Box::new(wake)));
}

/// **The colours the system would draw `hwnd`'s caption in now**: its surface and its words,
/// for a light or a `dark` window, `active` or not (milestone 642).
///
/// - Under **high contrast**, the scheme's caption colours.
/// - With the **accent colour on title bars** (Settings › Personalization › Colours), the
///   accent behind an active window's caption, with white or black words, whichever reads;
///   an inactive window's is the system's own inactive accent if it keeps one, else the
///   plain caption's.
/// - Otherwise the desktop's **plain caption**: `#F3F3F3` (Windows 11; `#FFFFFF` before)
///   or `#202020` dark, black or white words, grey on an inactive window — measured on
///   Windows 11's own captions. An active window's caption there is tinted by the wallpaper
///   (Mica), which an opaque surface cannot follow: the untinted surface is the nearest.
pub(crate) fn caption_colors(dark: bool, active: bool) -> (Color, Color) {
    let caption = LINE.with(|line| *line.borrow_mut().caption.get_or_insert_with(read_caption));
    if let Some([surface, ink, inactive_surface, inactive_ink]) = caption.high_contrast {
        return if active {
            (surface, ink)
        } else {
            (inactive_surface, inactive_ink)
        };
    }
    match caption.accent {
        Some((accent, _)) if active => {
            let words = if accent.compute_luminance() > 0.179 {
                Color::BLACK
            } else {
                Color::WHITE
            };
            return (accent, words);
        }
        Some((_, Some(inactive))) => {
            let words = if inactive.compute_luminance() > 0.179 {
                Color::rgb8(0x91, 0x91, 0x91)
            } else {
                Color::rgb8(0x79, 0x79, 0x79)
            };
            return (inactive, words);
        }
        _ => {}
    }
    match (dark, active) {
        (false, true) => (light_caption(), Color::BLACK),
        (false, false) => (light_caption(), Color::rgb8(0x91, 0x91, 0x91)),
        (true, true) => (Color::rgb8(0x20, 0x20, 0x20), Color::WHITE),
        (true, false) => (Color::rgb8(0x20, 0x20, 0x20), Color::rgb8(0x79, 0x79, 0x79)),
    }
}

/// The plain light caption: Windows 11's grey, white before it.
fn light_caption() -> Color {
    // Windows 11 is build 22000 and later of "Windows 10".
    let build = os_build();
    if build >= 22000 || build == 0 {
        Color::rgb8(0xF3, 0xF3, 0xF3)
    } else {
        Color::WHITE
    }
}

/// The system's build number, `0` if it cannot be read.
fn os_build() -> u32 {
    type RtlGetVersion = unsafe extern "system" fn(
        *mut windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW,
    ) -> i32;
    thread_local! {
        static BUILD: u32 = unsafe {
            use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
            let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr() as *const u8);
            match GetProcAddress(ntdll, c"RtlGetVersion".as_ptr() as *const u8) {
                Some(f) => {
                    let get: RtlGetVersion = std::mem::transmute(f);
                    let mut info: windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW =
                        std::mem::zeroed();
                    info.dwOSVersionInfoSize = std::mem::size_of_val(&info) as u32;
                    if get(&mut info) >= 0 {
                        info.dwBuildNumber
                    } else {
                        0
                    }
                }
                None => 0,
            }
        };
    }
    BUILD.with(|build| *build)
}

/// Reads what the system says of its captions.
fn read_caption() -> Caption {
    // SAFETY: plain queries into structures sized for them.
    unsafe {
        let mut contrast: HIGHCONTRASTW = std::mem::zeroed();
        contrast.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
        let on = SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            &mut contrast as *mut HIGHCONTRASTW as *mut core::ffi::c_void,
            0,
        ) != 0
            && contrast.dwFlags & HCF_HIGHCONTRASTON != 0;
        let high_contrast = on.then(|| {
            [
                COLOR_ACTIVECAPTION,
                COLOR_CAPTIONTEXT,
                COLOR_INACTIVECAPTION,
                COLOR_INACTIVECAPTIONTEXT,
            ]
            .map(|index| colorref(GetSysColor(index)))
        });
        let accent = (dwm_dword("ColorPrevalence") == Some(1))
            .then(|| dwm_dword("AccentColor").map(abgr))
            .flatten()
            .map(|accent| (accent, dwm_dword("AccentColorInactive").map(abgr)));
        Caption {
            high_contrast,
            accent,
        }
    }
}

/// A `DWORD` of the desktop compositor's settings for the person.
fn dwm_dword(name: &str) -> Option<u32> {
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\DWM\0"
        .encode_utf16()
        .collect();
    let value: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut data: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: the strings end in a nul, and the buffer is a `DWORD` as asked for.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut core::ffi::c_void,
            &mut size,
        )
    };
    (status == 0).then_some(data)
}

/// A `COLORREF` (`0x00BBGGRR`).
fn colorref(c: u32) -> Color {
    Color::rgb8(c as u8, (c >> 8) as u8, (c >> 16) as u8)
}

/// The compositor's accent (`0xAABBGGRR`), opaque.
fn abgr(c: u32) -> Color {
    colorref(c & 0x00FF_FFFF)
}

/// The system's caption height for `hwnd`'s screen, in physical pixels.
unsafe fn caption_height(hwnd: HWND) -> i32 {
    let dpi = GetDpiForWindow(hwnd);
    GetSystemMetricsForDpi(SM_CYCAPTION, dpi)
        + GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
}

/// Shares `hwnd`'s title bar line with its content.
///
/// # Safety
///
/// `hwnd` must be a live top-level window owned by this thread.
pub(crate) unsafe fn enable(hwnd: HWND) {
    let height = caption_height(hwnd);
    LINE.with(|line| line.borrow_mut().height = height);
    let margins = MARGINS {
        cxLeftWidth: 0,
        cxRightWidth: 0,
        cyTopHeight: height,
        cyBottomHeight: 0,
    };
    DwmExtendFrameIntoClientArea(hwnd, &margins);
    SetWindowSubclass(hwnd, Some(subclass), SUBCLASS, 0);
    frame_changed(hwnd);
}

/// Gives `hwnd`'s title bar line back to the system.
///
/// # Safety
///
/// As [`enable`].
pub(crate) unsafe fn disable(hwnd: HWND) {
    RemoveWindowSubclass(hwnd, Some(subclass), SUBCLASS);
    let margins = MARGINS {
        cxLeftWidth: 0,
        cxRightWidth: 0,
        cyTopHeight: 0,
        cyBottomHeight: 0,
    };
    DwmExtendFrameIntoClientArea(hwnd, &margins);
    LINE.with(|line| *line.borrow_mut() = Line::default());
    frame_changed(hwnd);
}

unsafe fn frame_changed(hwnd: HWND) {
    SetWindowPos(
        hwnd,
        std::ptr::null_mut(),
        0,
        0,
        0,
        0,
        SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
    );
}

/// The line's height, in physical pixels.
pub(crate) fn height() -> i32 {
    LINE.with(|line| line.borrow().height)
}

/// Where the compositor keeps its three buttons, in client pixels.
///
/// # Safety
///
/// As [`enable`].
pub(crate) unsafe fn buttons(hwnd: HWND) -> Option<RECT> {
    let mut bounds = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let got = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CAPTION_BUTTON_BOUNDS as u32,
        &mut bounds as *mut RECT as *mut core::ffi::c_void,
        std::mem::size_of::<RECT>() as u32,
    );
    if got != 0 || bounds.right <= bounds.left {
        return None;
    }
    // The bounds are the window's; the client starts past its left border.
    let mut window = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    GetWindowRect(hwnd, &mut window);
    let mut origin = POINT { x: 0, y: 0 };
    ClientToScreen(hwnd, &mut origin);
    let (dx, dy) = (origin.x - window.left, origin.y - window.top);
    Some(RECT {
        left: bounds.left - dx,
        top: (bounds.top - dy).max(0),
        right: bounds.right - dx,
        bottom: bounds.bottom - dy,
    })
}

/// What the application has on the line, and where the window's icon is, in client
/// pixels: told every frame.
pub(crate) fn publish(interactive: Vec<RECT>, icon: Option<RECT>) {
    LINE.with(|line| {
        let mut line = line.borrow_mut();
        line.interactive = interactive;
        line.icon = icon;
    });
}

/// The compositor's button under the pointer and the one pressed, as hit-test codes (`0`
/// for none).
pub(crate) fn buttons_state() -> (u32, u32) {
    LINE.with(|line| {
        let line = line.borrow();
        (line.hovered, line.pressed)
    })
}

/// Records a new hover or press and asks for a frame if it changed.
unsafe fn set_state(hwnd: HWND, hovered: Option<u32>, pressed: Option<u32>) {
    let changed = LINE.with(|line| {
        let mut line = line.borrow_mut();
        let before = (line.hovered, line.pressed);
        if let Some(h) = hovered {
            line.hovered = h;
        }
        if let Some(p) = pressed {
            line.pressed = p;
        }
        before != (line.hovered, line.pressed)
    });
    if changed {
        InvalidateRect(hwnd, std::ptr::null(), 0);
        WAKE.with(|slot| {
            if let Some(wake) = slot.borrow().as_ref() {
                wake();
            }
        });
    }
}

fn is_button(code: u32) -> bool {
    matches!(code, HTMINBUTTON | HTMAXBUTTON | HTCLOSE)
}

fn contains(r: &RECT, at: &POINT) -> bool {
    at.x >= r.left && at.x < r.right && at.y >= r.top && at.y < r.bottom
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_NCLBUTTONDOWN if is_button(wparam as u32) => {
            set_state(hwnd, None, Some(wparam as u32));
        }
        WM_NCLBUTTONUP | WM_LBUTTONUP => set_state(hwnd, None, Some(0)),
        WM_NCMOUSELEAVE => set_state(hwnd, Some(0), Some(0)),
        // The person changed the colours, the accent's place, the contrast or the theme:
        // the caption is read again, and a frame asked for (milestone 642).
        WM_SETTINGCHANGE | WM_DWMCOLORIZATIONCOLORCHANGED | WM_THEMECHANGED => {
            LINE.with(|line| line.borrow_mut().caption = None);
            WAKE.with(|slot| {
                if let Some(wake) = slot.borrow().as_ref() {
                    wake();
                }
            });
        }
        _ => {}
    }
    // The compositor answers for its own buttons first.
    let mut answered: LRESULT = 0;
    if DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut answered) != 0 {
        if msg == WM_NCHITTEST {
            let code = answered as u32;
            set_state(hwnd, Some(if is_button(code) { code } else { 0 }), None);
            if is_button(code) {
                // To hear the pointer leave the line.
                let mut track = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE | TME_NONCLIENT,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut track);
            }
        }
        return answered;
    }
    match msg {
        WM_NCCALCSIZE if wparam != 0 => {
            let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
            let top = params.rgrc[0].top;
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            // The caption goes to the client; the sides and the bottom keep their borders.
            // Maximized, the window hangs past the screen by its frame, so the client starts
            // that far down.
            params.rgrc[0].top = top;
            if IsZoomed(hwnd) != 0 {
                let mut frame = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                GetWindowRect(hwnd, &mut frame);
                params.rgrc[0].top += (params.rgrc[0].left - frame.left).max(0);
            }
            result
        }
        WM_NCHITTEST => {
            set_state(hwnd, Some(0), None);
            let hit = DefSubclassProc(hwnd, msg, wparam, lparam);
            if hit != HTCLIENT as LRESULT {
                return hit;
            }
            let mut at = POINT {
                x: (lparam & 0xffff) as i16 as i32,
                y: ((lparam >> 16) & 0xffff) as i16 as i32,
            };
            ScreenToClient(hwnd, &mut at);
            if at.y < RESIZE_EDGE && IsZoomed(hwnd) == 0 {
                return HTTOP as LRESULT;
            }
            LINE.with(|line| {
                let line = line.borrow();
                if at.y >= line.height {
                    hit
                } else if line.icon.as_ref().is_some_and(|r| contains(r, &at)) {
                    HTSYSMENU as LRESULT
                } else if line.interactive.iter().any(|r| contains(r, &at)) {
                    hit
                } else {
                    HTCAPTION as LRESULT
                }
            })
        }
        _ => DefSubclassProc(hwnd, msg, wparam, lparam),
    }
}
