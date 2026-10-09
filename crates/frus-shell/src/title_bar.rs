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
//! ([`buttons`]), in the state the compositor reports ([`buttons_state`]).

use std::cell::RefCell;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
    DWMWA_CAPTION_BUTTON_BOUNDS,
};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, InvalidateRect, ScreenToClient};
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsZoomed, SetWindowPos, HTCAPTION, HTCLIENT, HTCLOSE, HTMAXBUTTON, HTMINBUTTON,
    HTSYSMENU, HTTOP, NCCALCSIZE_PARAMS, SM_CXPADDEDBORDER, SM_CYCAPTION, SM_CYSIZEFRAME,
    SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_LBUTTONUP, WM_NCCALCSIZE,
    WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE,
};

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
