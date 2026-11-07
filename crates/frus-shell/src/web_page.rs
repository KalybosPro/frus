// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! The page under the canvas, on the Web (milestone 749).
//!
//! **The browser paints the phone's navigation bar with the page's background.** Chrome on
//! Android colours the system navigation bar, under its own window, with the background of
//! the document's root element, and follows it when it changes. A page whose background is
//! the loading screen's dark one therefore put a dark band under an application drawn light
//! — the band the widget gallery showed below its canvas. The shell now keeps that background
//! the colour the application asks for its navigation bar: the region at the bottom of the
//! frame, then the theme, as the Android shell does for its own bars.
//!
//! **The on-screen keyboard is an inset, as on a phone's own applications.** A phone's browser
//! lays the keyboard over the page: the page keeps its size and only the *visual* viewport
//! shrinks. The canvas keeps its size too, and the part of it the keyboard covers is reported
//! as the window's bottom inset, which the shell turns into the keyboard's `view_insets` —
//! the layout lifts what must stay in sight and the focused field is scrolled into view, as on
//! Android. The browser is told nothing changed size, so it has nothing to pan (see
//! `web_editing`, which keeps the element it edits through out of the keyboard's way).
//!
//! **Unless the keyboard leaves too little** (milestone 770): in landscape a phone's keyboard
//! leaves a page a few dozen pixels, and an application laid out in them has no room for the
//! field being typed into. Then the application keeps its whole size and the page is panned to
//! the field instead ([`KeyboardFit`]), as a phone pans a window it cannot shrink.
//!
//! Best-effort like the other bridges: with no document, the page keeps the colour it had, and
//! with no visual viewport the keyboard covers what it covers.

use frus_widgets::Color;

/// `color` as CSS spells it: `rgb(r g b / a)`, in sRGB like `Color` itself.
pub(crate) fn css_color(color: Color) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let alpha = (color.a.clamp(0.0, 1.0) * 1000.0).round() / 1000.0;
    format!(
        "rgb({} {} {} / {alpha})",
        byte(color.r),
        byte(color.g),
        byte(color.b)
    )
}

/// The background a navigation bar of `color` asks of the page, over `under` — the
/// application's own background. A page background has nothing behind it, so a translucent
/// bar is laid over `under` the way the renderer composites, in linear light, and the
/// result is opaque.
pub(crate) fn page_background(color: Color, under: Color) -> Color {
    let alpha = color.a.clamp(0.0, 1.0);
    let over = Color { a: 1.0, ..color }.to_linear();
    let under = Color { a: 1.0, ..under }.to_linear();
    under.lerp(over, alpha).to_srgb()
}

/// Paints the root element's background `color` (opaque), inline, where it outranks the page's own
/// stylesheet. Returns whether it reached the page, so that the shell asks again next frame
/// rather than believing a request that was never made.
#[cfg(web)]
pub(crate) fn paint_behind(color: Color) -> bool {
    use wasm_bindgen::JsCast;
    let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
        .and_then(|root| root.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return false;
    };
    root.style()
        .set_property("background-color", &css_color(color))
        .is_ok()
}

/// `color` as a `#rrggbb` hex triplet, its alpha dropped: the spelling every browser
/// takes in `<meta name="theme-color">`, where a translucent colour is not honoured.
pub(crate) fn css_hex(color: Color) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(color.r),
        byte(color.g),
        byte(color.b)
    )
}

/// The colour a system bar next to the page is given (milestone 751): the colour a region
/// at that edge states for it first — an explicit request — then the colour the frame
/// actually paints at that edge, then the theme's background; laid over the theme's
/// background when translucent, since nothing in the page is behind it.
///
/// The painted colour is what keeps the bar from being one step away from the screen: a
/// scaffold that paints `#F5F5F7` over a theme whose background is `#F5F6F8` gets a bar of
/// `#F5F5F7`, not of the theme's colour.
pub(crate) fn bar_color(stated: Option<Color>, painted: Option<Color>, theme: Color) -> Color {
    page_background(stated.or(painted).unwrap_or(theme), theme)
}

/// Sets the page's `<meta name="theme-color">` to `color`, creating it in the `<head>` when
/// the page has none. A phone's browser colours its toolbar and the status bar above it
/// with it, as a native application's status bar is coloured. Returns whether it reached
/// the page.
#[cfg(web)]
pub(crate) fn paint_toolbar(color: Color) -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    let meta = match document.query_selector("meta[name=\"theme-color\"]") {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            let Some(head) = document.query_selector("head").ok().flatten() else {
                return false;
            };
            let Ok(meta) = document.create_element("meta") else {
                return false;
            };
            if meta.set_attribute("name", "theme-color").is_err()
                || head.append_child(&meta).is_err()
            {
                return false;
            }
            meta
        }
        Err(_) => return false,
    };
    meta.set_attribute("content", &css_hex(color)).is_ok()
}

/// The safe area's four insets, in CSS pixels: what a notch, a rounded corner or a home
/// indicator takes from the page's edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SafeArea {
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
    pub(crate) left: f64,
}

/// The number of CSS pixels a computed length such as `"47px"` says; nought for anything
/// else (`""`, `"auto"`, a negative length).
pub(crate) fn css_px(value: &str) -> f64 {
    value
        .trim()
        .strip_suffix("px")
        .and_then(|number| number.trim().parse::<f64>().ok())
        .filter(|number| number.is_finite() && *number > 0.0)
        .unwrap_or(0.0)
}

/// The window's insets on the Web, in CSS pixels, top, right, bottom, left, from the safe
/// area and the on-screen keyboard's cover: the keyboard is laid over the bottom of the
/// page, the home indicator with it, so the bottom inset is the larger of the two — what
/// the shell's baseline then splits into padding and the keyboard's `view_insets`, as it
/// does a phone's own.
pub(crate) fn window_insets(safe: SafeArea, keyboard: f64) -> [f64; 4] {
    [safe.top, safe.right, safe.bottom.max(keyboard), safe.left]
}

/// The probe's style: an element that takes no room and is never seen, padded by the safe
/// area — a browser exposes the insets to style sheets only.
#[cfg(web)]
const PROBE_STYLE: &str = "position: fixed; left: 0; top: 0; width: 0; height: 0; \
    visibility: hidden; pointer-events: none; \
    padding: env(safe-area-inset-top, 0px) env(safe-area-inset-right, 0px) \
    env(safe-area-inset-bottom, 0px) env(safe-area-inset-left, 0px);";

/// Reads the safe area through a probe element padded by `env(safe-area-inset-*)`. The
/// insets are non-zero only on a page whose viewport asks for `viewport-fit=cover`, which
/// every page `frus create` writes does.
///
/// The answer is kept for the window's size: a notch moves from the top to a side only with
/// a rotation, and a rotation resizes the window.
#[cfg(web)]
#[inline(never)]
pub(crate) fn safe_area() -> SafeArea {
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
    type Probe = (web_sys::HtmlElement, (f64, f64), SafeArea);
    thread_local! {
        static PROBE: RefCell<Option<Probe>> = const { RefCell::new(None) };
    }
    let Some(window) = web_sys::window() else {
        return SafeArea::default();
    };
    let number = |value: Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>| {
        value.ok().and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    let size = (number(window.inner_width()), number(window.inner_height()));
    PROBE.with(|probe| {
        let mut probe = probe.borrow_mut();
        if let Some((_, at, safe)) = probe.as_ref() {
            if *at == size {
                return *safe;
            }
        }
        let element = match probe.as_ref() {
            Some((element, _, _)) => element.clone(),
            None => {
                let Some(document) = window.document() else {
                    return SafeArea::default();
                };
                let Some(element) = document
                    .create_element("div")
                    .ok()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                else {
                    return SafeArea::default();
                };
                let _ = element.set_attribute("aria-hidden", "true");
                let _ = element.set_attribute("style", PROBE_STYLE);
                let Some(body) = document.body() else {
                    return SafeArea::default();
                };
                if body.append_child(&element).is_err() {
                    return SafeArea::default();
                }
                element
            }
        };
        let safe = match window.get_computed_style(&element) {
            Ok(Some(style)) => {
                let side = |name: &str| css_px(&style.get_property_value(name).unwrap_or_default());
                SafeArea {
                    top: side("padding-top"),
                    right: side("padding-right"),
                    bottom: side("padding-bottom"),
                    left: side("padding-left"),
                }
            }
            _ => SafeArea::default(),
        };
        *probe = Some((element, size, safe));
        safe
    })
}

/// How many CSS pixels of a canvas whose bottom edge is at `canvas_bottom` (in the page's
/// layout coordinates) lie below what the visual viewport shows: the on-screen keyboard's
/// cover. `vv_top`, `vv_height` and `vv_scale` are the visual viewport's.
///
/// A pinch-zoomed page shows less of itself for a reason that is not a keyboard, and its
/// visual viewport is not an inset of the window: none is reported then. Less than a pixel is
/// rounding, not a keyboard.
///
/// Nor is a visual viewport with no height: Chrome on Android reports one after a rotation with
/// the keyboard up, while the page is shown above the keyboard all along (milestone 765). The
/// layout viewport, which that browser has already shrunk to what the keyboard leaves, is
/// believed instead.
pub(crate) fn keyboard_cover(
    canvas_bottom: f64,
    vv_top: f64,
    vv_height: f64,
    vv_scale: f64,
) -> f64 {
    if (vv_scale - 1.0).abs() > 0.01 || vv_height < 1.0 {
        return 0.0;
    }
    let cover = canvas_bottom - (vv_top + vv_height);
    if cover < 1.0 {
        0.0
    } else {
        cover
    }
}

/// How the page makes room for the on-screen keyboard (milestone 770).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum KeyboardFit {
    /// The keyboard is an inset: the application lays itself out in what the keyboard leaves
    /// and scrolls the focused field into it.
    #[default]
    Resize,
    /// The application keeps its whole size and the browser pans the page to the focused
    /// field, as a phone pans a window that cannot be shrunk to what its keyboard leaves. Taken
    /// when what the keyboard leaves cannot hold the field: a phone in landscape leaves a page
    /// a few dozen pixels, less than an app bar.
    Pan,
}

/// The page's viewport while the on-screen keyboard is up, as `[width, height, visible
/// height]` in whole CSS pixels; `None` when no keyboard is up. `inner_width` and
/// `inner_height` are the layout viewport's, `vv_height` and `vv_scale` the visual viewport's.
///
/// The keyboard is what the visual viewport leaves out of the layout viewport, whatever the
/// browser panned: a pan moves the visual viewport, not its height. A pinch-zoomed page and a
/// visual viewport with no height (Chrome's, after a rotation with the keyboard up) are not a
/// keyboard, as for [`keyboard_cover`].
pub(crate) fn keyboard_view(
    inner_width: f64,
    inner_height: f64,
    vv_height: f64,
    vv_scale: f64,
) -> Option<[i32; 3]> {
    if (vv_scale - 1.0).abs() > 0.01 || vv_height < 1.0 || inner_height - vv_height < 1.0 {
        return None;
    }
    Some([
        inner_width.round() as i32,
        inner_height.round() as i32,
        vv_height.round() as i32,
    ])
}

/// What the keyboard leaves of a canvas whose top is at `canvas_top`, in CSS pixels, when the
/// application is laid out above it: from the canvas's top to the visual viewport's bottom.
pub(crate) fn keyboard_room(canvas_top: f64, vv_top: f64, vv_height: f64) -> f64 {
    (vv_top + vv_height - canvas_top).max(0.0)
}

/// Which way the page makes room for the keyboard, after a frame laid out above it: `field` is
/// the height of the focused field in CSS pixels, `None` when that frame had no room to lay it
/// out at all, and `room` what the keyboard leaves ([`keyboard_room`]).
///
/// A field the layout cannot show above the keyboard — not laid out, or taller than all there
/// is — is never seen there however the application scrolls: the page pans to it instead.
pub(crate) fn keyboard_fit(field: Option<f64>, room: f64) -> KeyboardFit {
    match field {
        Some(height) if height <= room => KeyboardFit::Resize,
        _ => KeyboardFit::Pan,
    }
}

/// Remembers, for one keyboard over one page, which way the page makes room for it
/// ([`KeyboardFit`]).
///
/// The choice is made once per keyboard height and field, from a frame laid out above the
/// keyboard: a pan is kept while the keyboard stays as it is — moving the focus to the next
/// field does not lay the page out above it again — and a resize is kept for that field, so a
/// field the application later scrolls out of its layout does not turn the keyboard into a pan.
#[derive(Debug, Default)]
pub(crate) struct KeyboardFitter {
    /// The keyboard the choice was made for ([`keyboard_view`]).
    view: Option<[i32; 3]>,
    /// The field it was made for.
    field: Option<u64>,
    fit: KeyboardFit,
}

impl KeyboardFitter {
    /// The way to lay out a frame over the keyboard `view`: [`KeyboardFit::Pan`] only when
    /// that was chosen for this very keyboard.
    pub(crate) fn fit(&self, view: Option<[i32; 3]>) -> KeyboardFit {
        if view.is_some() && self.view == view {
            self.fit
        } else {
            KeyboardFit::Resize
        }
    }

    /// Learns from a frame laid out with [`Self::fit`]: `view` is the keyboard, `focused` the
    /// focused field that takes typing (its id, and its height in CSS pixels when the frame
    /// laid it out), `room` what the keyboard leaves. True when the choice changed what the
    /// next frame must lay out.
    pub(crate) fn learn(
        &mut self,
        view: Option<[i32; 3]>,
        focused: Option<(u64, Option<f64>)>,
        room: f64,
    ) -> bool {
        let before = self.fit(self.view);
        let (Some(view), Some((field, height))) = (view, focused) else {
            *self = Self::default();
            return before == KeyboardFit::Pan;
        };
        if self.view == Some(view) && (self.fit == KeyboardFit::Pan || self.field == Some(field)) {
            self.field = Some(field);
            return false;
        }
        // A new keyboard or a new field: this frame was laid out above the keyboard.
        let fit = keyboard_fit(height, room);
        *self = Self {
            view: Some(view),
            field: Some(field),
            fit,
        };
        fit == KeyboardFit::Pan
    }
}

/// Whether an element from `top` to `bottom` is as much in sight as a visual viewport from
/// `vv_top`, `vv_height` tall, can show of it: all of it, or, for one taller than the visual
/// viewport, as much as fits. All in the layout viewport's CSS pixels.
pub(crate) fn in_sight(top: f64, bottom: f64, vv_top: f64, vv_height: f64) -> bool {
    let shown = bottom.min(vv_top + vv_height) - top.max(vv_top);
    shown >= (bottom - top).min(vv_height) - 1.0
}

/// The keyboard as the page has it now ([`keyboard_view`]).
#[cfg(web)]
pub(crate) fn keyboard_view_now() -> Option<[i32; 3]> {
    let window = web_sys::window()?;
    let vv = window.visual_viewport()?;
    let number = |value: Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>| {
        value.ok().and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    keyboard_view(
        number(window.inner_width()),
        number(window.inner_height()),
        vv.height(),
        vv.scale(),
    )
}

/// What the keyboard leaves of `canvas` now ([`keyboard_room`]).
#[cfg(web)]
pub(crate) fn keyboard_room_of(canvas: &web_sys::HtmlCanvasElement) -> f64 {
    let Some(vv) = web_sys::window().and_then(|window| window.visual_viewport()) else {
        return 0.0;
    };
    keyboard_room(
        canvas.get_bounding_client_rect().top(),
        vv.offset_top(),
        vv.height(),
    )
}

/// The on-screen keyboard's cover of `canvas`, in CSS pixels, measured now.
#[cfg(web)]
pub(crate) fn keyboard_cover_of(canvas: &web_sys::HtmlCanvasElement) -> f64 {
    let Some(vv) = web_sys::window().and_then(|window| window.visual_viewport()) else {
        return 0.0;
    };
    let bottom = canvas.get_bounding_client_rect().bottom();
    // The canvas's box is in the layout viewport's coordinates, and so is the visual
    // viewport's offset.
    keyboard_cover(bottom, vv.offset_top(), vv.height(), vv.scale())
}

/// Asks `window` for a frame whenever the visual viewport changes size — the keyboard
/// coming or going, which resizes nothing a resize observer would see.
#[cfg(web)]
pub(crate) fn redraw_on_viewport_changes(window: std::sync::Arc<winit::window::Window>) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let Some(vv) = web_sys::window().and_then(|window| window.visual_viewport()) else {
        return;
    };
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        window.request_redraw();
    }) as Box<dyn FnMut(web_sys::Event)>);
    for kind in ["resize", "scroll"] {
        let _ = vv.add_event_listener_with_callback(kind, closure.as_ref().unchecked_ref());
    }
    // Kept for the life of the page.
    closure.forget();
}

/// Asks for the canvas's WebGL context back whenever the browser takes it away, and sets
/// `restored` (with a frame) when it is given back.
///
/// A browser restores a lost context only for a page that says it wants it, by cancelling the
/// `webglcontextlost` event; one that does not keeps a dark canvas for good. A WebGPU canvas
/// never hears either event: its device says it was lost.
#[cfg(web)]
pub(crate) fn recover_lost_contexts(
    window: &std::sync::Arc<winit::window::Window>,
    restored: std::rc::Rc<std::cell::Cell<bool>>,
) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowExtWebSys;
    let Some(canvas) = window.canvas() else {
        return;
    };
    let lost = Closure::wrap(Box::new(move |event: web_sys::Event| {
        log::warn!("the browser took the WebGL context away: asking for it back");
        event.prevent_default();
    }) as Box<dyn FnMut(web_sys::Event)>);
    let back = {
        let window = window.clone();
        Closure::wrap(Box::new(move |_: web_sys::Event| {
            log::warn!("the browser gave the WebGL context back: drawing again");
            restored.set(true);
            window.request_redraw();
        }) as Box<dyn FnMut(web_sys::Event)>)
    };
    let _ =
        canvas.add_event_listener_with_callback("webglcontextlost", lost.as_ref().unchecked_ref());
    let _ = canvas
        .add_event_listener_with_callback("webglcontextrestored", back.as_ref().unchecked_ref());
    // Kept for the life of the page.
    lost.forget();
    back.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_is_spelt_in_srgb_bytes() {
        assert_eq!(
            css_color(Color::rgb(245.0 / 255.0, 245.0 / 255.0, 247.0 / 255.0)),
            "rgb(245 245 247 / 1)"
        );
        assert_eq!(
            css_color(Color::rgba(1.0, 0.0, 0.0, 0.5)),
            "rgb(255 0 0 / 0.5)"
        );
        // Out of range is clamped, never wrapped.
        assert_eq!(css_color(Color::rgb(1.5, -0.2, 0.0)), "rgb(255 0 0 / 1)");
    }

    fn close(a: Color, b: Color) -> bool {
        a.to_array()
            .iter()
            .zip(b.to_array())
            .all(|(x, y)| (x - y).abs() < 1e-4)
    }

    #[test]
    fn an_opaque_bar_is_the_page_s_colour() {
        let bar = Color::rgb(0.2, 0.4, 0.6);
        assert!(close(page_background(bar, Color::rgb(1.0, 1.0, 1.0)), bar));
    }

    #[test]
    fn a_see_through_bar_shows_the_application_under_it() {
        let under = Color::rgb(0.96, 0.96, 0.97);
        assert!(close(
            page_background(Color::rgba(0.0, 0.0, 0.0, 0.0), under),
            under
        ));
        // Half black over white is composited in linear light: a linear 0.5, not sRGB 0.5.
        let half = page_background(Color::rgba(0.0, 0.0, 0.0, 0.5), Color::rgb(1.0, 1.0, 1.0));
        assert!(
            (half.r - Color::rgb(0.5, 0.5, 0.5).to_srgb().r).abs() < 1e-3,
            "{half:?}"
        );
        assert_eq!(half.a, 1.0);
    }

    #[test]
    fn a_toolbar_colour_is_an_opaque_hex_triplet() {
        assert_eq!(
            css_hex(Color::rgb(245.0 / 255.0, 245.0 / 255.0, 247.0 / 255.0)),
            "#f5f5f7"
        );
        assert_eq!(css_hex(Color::rgba(1.0, 0.0, 0.5, 0.2)), "#ff0080");
    }

    /// The bar next to the page: what a region states, then what is painted at the edge,
    /// then the theme — the gallery's scaffold paints one step away from its theme, and the
    /// bar is the scaffold's colour (milestone 751).
    #[test]
    fn a_bar_is_what_is_stated_then_what_is_painted_then_the_theme() {
        let theme = Color::rgb(245.0 / 255.0, 246.0 / 255.0, 248.0 / 255.0);
        let scaffold = Color::rgb(245.0 / 255.0, 245.0 / 255.0, 247.0 / 255.0);
        let stated = Color::rgb(0.1, 0.2, 0.3);
        assert!(close(bar_color(None, None, theme), theme));
        assert_eq!(css_hex(bar_color(None, Some(scaffold), theme)), "#f5f5f7");
        assert!(close(
            bar_color(Some(stated), Some(scaffold), theme),
            stated
        ));
        // A translucent paint is laid over the theme: the page is opaque.
        assert_eq!(
            bar_color(None, Some(Color::rgba(0.0, 0.0, 0.0, 0.5)), theme).a,
            1.0
        );
    }

    #[test]
    fn a_computed_length_is_read_in_css_pixels() {
        assert_eq!(css_px("47px"), 47.0);
        assert_eq!(css_px(" 34.5px "), 34.5);
        assert_eq!(css_px("0px"), 0.0);
        assert_eq!(css_px(""), 0.0);
        assert_eq!(css_px("auto"), 0.0);
        assert_eq!(css_px("-3px"), 0.0);
    }

    /// A notch and a home indicator, then the keyboard laid over the indicator.
    #[test]
    fn the_safe_area_and_the_keyboard_make_the_window_s_insets() {
        let portrait = SafeArea {
            top: 47.0,
            right: 0.0,
            bottom: 34.0,
            left: 0.0,
        };
        assert_eq!(window_insets(portrait, 0.0), [47.0, 0.0, 34.0, 0.0]);
        assert_eq!(window_insets(portrait, 336.0), [47.0, 0.0, 336.0, 0.0]);
        let landscape = SafeArea {
            top: 0.0,
            right: 47.0,
            bottom: 21.0,
            left: 47.0,
        };
        assert_eq!(window_insets(landscape, 0.0), [0.0, 47.0, 21.0, 47.0]);
    }

    #[test]
    fn the_keyboard_is_what_the_visual_viewport_leaves_out_at_the_bottom() {
        // The phone this was measured on: a 788-pixel page, the keyboard up.
        assert_eq!(keyboard_cover(787.84, 0.0, 321.96, 1.0), 787.84 - 321.96);
        // No keyboard.
        assert_eq!(keyboard_cover(787.84, 0.0, 787.84, 1.0), 0.0);
        // Rounding between the two boxes is not a keyboard.
        assert_eq!(keyboard_cover(788.0, 0.0, 787.5, 1.0), 0.0);
        // A pinch-zoomed page shows less of itself, and that is not a keyboard either.
        assert_eq!(keyboard_cover(787.84, 100.0, 393.92, 2.0), 0.0);
    }

    /// Chrome on Android, rotated to portrait with the keyboard up, reports a visual viewport
    /// **0 pixels tall** (offset 96.86 on a 318-pixel page), on a plain page as on a frus one,
    /// while the page is plainly shown above the keyboard (milestone 765). Taken at its word,
    /// the keyboard covered the whole canvas and the application laid itself out in nothing.
    #[test]
    fn a_visual_viewport_with_no_height_is_not_a_keyboard() {
        assert_eq!(keyboard_cover(318.04, 96.86, 0.0, 1.0), 0.0);
        assert_eq!(keyboard_cover(318.04, 38.0, 0.0, 1.0), 0.0);
    }

    /// The keyboard is what the visual viewport leaves out of the layout viewport, however the
    /// browser panned it (milestone 770).
    #[test]
    fn a_keyboard_is_a_visual_viewport_shorter_than_the_page() {
        // The phone in landscape: a 339-pixel page, the keyboard leaving 38.
        assert_eq!(keyboard_view(778.4, 339.2, 38.0, 1.0), Some([778, 339, 38]));
        // No keyboard, a pinch zoom, Chrome's visual viewport with no height.
        assert_eq!(keyboard_view(778.4, 339.2, 339.2, 1.0), None);
        assert_eq!(keyboard_view(778.4, 339.2, 169.6, 2.0), None);
        assert_eq!(keyboard_view(318.0, 318.04, 0.0, 1.0), None);
    }

    /// What the keyboard leaves of the canvas: from its top to the visual viewport's bottom.
    #[test]
    fn the_room_is_what_the_visual_viewport_shows_of_the_canvas() {
        // Portrait: the canvas under a 36-pixel header, the keyboard leaving 322.
        assert_eq!(keyboard_room(36.18, 0.0, 321.96), 321.96 - 36.18);
        // Landscape: 38 pixels, nearly all of them the header's.
        assert!((keyboard_room(36.18, 0.0, 38.0) - 1.82).abs() < 1e-9);
        // Panned past the header.
        assert_eq!(keyboard_room(36.18, 100.0, 38.0), 138.0 - 36.18);
        // Nothing of it.
        assert_eq!(keyboard_room(36.18, 0.0, 20.0), 0.0);
    }

    /// A field the layout cannot show above the keyboard makes the page pan to it
    /// (milestone 770). In landscape on the phone the keyboard left the page 38 CSS pixels:
    /// the field was not laid out, and the person typed into a field they could not see.
    #[test]
    fn a_field_with_no_room_above_the_keyboard_pans_the_page() {
        assert_eq!(keyboard_fit(Some(48.0), 285.78), KeyboardFit::Resize);
        assert_eq!(keyboard_fit(Some(48.0), 48.0), KeyboardFit::Resize);
        // Not laid out.
        assert_eq!(keyboard_fit(None, 285.78), KeyboardFit::Pan);
        // Laid out, but taller than all the keyboard leaves.
        assert_eq!(keyboard_fit(Some(48.0), 1.82), KeyboardFit::Pan);
    }

    /// The choice is made from a frame laid out above the keyboard, and kept: a pan while the
    /// keyboard stays, a resize for the field it was made for.
    #[test]
    fn the_fit_is_chosen_once_per_keyboard_and_field() {
        let landscape = Some([778, 339, 38]);
        let portrait = Some([393, 788, 322]);
        let mut fitter = KeyboardFitter::default();
        // No keyboard yet: laid out above nothing.
        assert!(!fitter.learn(None, Some((7, Some(48.0))), 303.0));
        assert_eq!(fitter.fit(None), KeyboardFit::Resize);
        // The landscape keyboard comes up; the first frame above it had no room for field 7.
        assert_eq!(fitter.fit(landscape), KeyboardFit::Resize);
        assert!(fitter.learn(landscape, Some((7, None)), 1.82));
        assert_eq!(fitter.fit(landscape), KeyboardFit::Pan);
        // Panned, the frame lays the field out at its full size: still a pan.
        assert!(!fitter.learn(landscape, Some((7, Some(48.0))), 1.82));
        // The next field, under the same keyboard: still a pan, no frame above the keyboard.
        assert!(!fitter.learn(landscape, Some((8, Some(48.0))), 1.82));
        assert_eq!(fitter.fit(landscape), KeyboardFit::Pan);
        // The keyboard goes: a resize again, and the frame must be laid out again.
        assert!(fitter.learn(None, Some((8, Some(48.0))), 303.0));
        assert_eq!(fitter.fit(landscape), KeyboardFit::Resize);

        // Portrait: room for the field, a resize, kept even when the field leaves the layout
        // later (scrolled out of a list).
        assert!(!fitter.learn(portrait, Some((7, Some(48.0))), 285.78));
        assert!(!fitter.learn(portrait, Some((7, None)), 285.78));
        assert_eq!(fitter.fit(portrait), KeyboardFit::Resize);
        // A different keyboard height is a new choice, made with a resize.
        assert_eq!(fitter.fit(landscape), KeyboardFit::Resize);
        // The focus goes: no choice left.
        assert!(!fitter.learn(portrait, None, 285.78));
        assert_eq!(fitter.fit(portrait), KeyboardFit::Resize);
    }

    /// The field is in sight when the visual viewport shows all of it, or all it can.
    #[test]
    fn a_field_is_in_sight_when_shown_as_much_as_it_can_be() {
        // A 48-pixel field in a 38-pixel visual viewport: 38 of it is all there is.
        assert!(in_sight(100.0, 148.0, 105.0, 38.0));
        assert!(!in_sight(100.0, 148.0, 0.0, 38.0));
        assert!(!in_sight(100.0, 148.0, 130.0, 38.0));
        // A small field: all of it.
        assert!(in_sight(100.0, 120.0, 90.0, 38.0));
        assert!(!in_sight(100.0, 120.0, 110.0, 38.0));
    }
}
