//! The browser tab's icon (see [`crate::AppIcon`]).
//!
//! The PNG is handed to the page as a `Blob` and its object URL becomes the `href` of the
//! page's `<link rel="icon">`, made if there is none. Best-effort like the other bridges: with
//! no document, or a browser that refuses, the tab keeps the icon it had.

use crate::icon::{replaces_the_pages_icon, AppIcon};

/// Puts `icon` on the tab, unless the page's own icon is to be left alone.
pub(crate) fn apply(icon: &AppIcon) {
    let Some(png) = icon.png() else {
        return;
    };
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let link = document
        .query_selector("link[rel~='icon']")
        .ok()
        .flatten()
        .or_else(|| {
            let head = document.query_selector("head").ok().flatten()?;
            let link = document.create_element("link").ok()?;
            link.set_attribute("rel", "icon").ok()?;
            head.append_child(&link).ok()?;
            Some(link)
        });
    let Some(link) = link else {
        return;
    };
    if !replaces_the_pages_icon(icon, link.get_attribute("href").as_deref()) {
        return;
    }
    let bytes = js_sys::Uint8Array::from(png);
    let parts = js_sys::Array::of1(&bytes);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("image/png");
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let _ = link.set_attribute("type", "image/png");
    let _ = link.set_attribute("href", &url);
}
