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

/// How many CSS pixels of a canvas whose bottom edge is at `canvas_bottom` (in the page's
/// layout coordinates) lie below what the visual viewport shows: the on-screen keyboard's
/// cover. `vv_top`, `vv_height` and `vv_scale` are the visual viewport's.
///
/// A pinch-zoomed page shows less of itself for a reason that is not a keyboard, and its
/// visual viewport is not an inset of the window: none is reported then. Less than a pixel is
/// rounding, not a keyboard.
pub(crate) fn keyboard_cover(
    canvas_bottom: f64,
    vv_top: f64,
    vv_height: f64,
    vv_scale: f64,
) -> f64 {
    if (vv_scale - 1.0).abs() > 0.01 {
        return 0.0;
    }
    let cover = canvas_bottom - (vv_top + vv_height);
    if cover < 1.0 {
        0.0
    } else {
        cover
    }
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
}
