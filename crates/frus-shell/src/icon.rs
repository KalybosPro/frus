//! The application's **icon**: the frus logo by default, the developer's when they say so.
//!
//! A new application should look finished the moment it runs, before anyone has drawn an
//! icon: its window has one, its browser tab has one. So every application has the frus logo
//! unless it says otherwise — and it can say otherwise, or say *none*:
//!
//! ```
//! use frus_shell::AppIcon;
//!
//! // The default: the frus logo. Nothing to write.
//! let _ = AppIcon::frus();
//! // Its own, from a PNG compiled into the binary.
//! # let png: &'static [u8] = &[];
//! let _ = AppIcon::from_png(png);
//! // No icon of the framework's at all: the platform's plain default.
//! let _ = AppIcon::none();
//! ```
//!
//! Where it shows, and where it cannot:
//!
//! - **Desktop** — the window's icon (its title bar and, on Windows, the taskbar). Wayland and
//!   macOS take their icon from the application bundle, not from the window, and ignore this.
//! - **Web** — the tab's icon. An icon the *page* declares (`<link rel="icon" href="…">`) wins
//!   over the default, since the page is the more specific author; an icon the application
//!   names explicitly wins over the page's.
//! - **Android** — the launcher icon is the manifest's, fixed when the package is built, and
//!   not the running code's to change. The template and the demo ship the frus logo as
//!   `res/mipmap-*/ic_launcher.png` and name it in `Cargo.toml`; replace those five files to
//!   change it.
//!
//! The logo is embedded at 256 px (about 22 kB) and not at the 1024 px of the artwork, which is
//! more than any of these places draws and would add most of a megabyte to every binary.

use std::borrow::Cow;

/// The frus logo at 256 px, as a PNG: the default icon. Not compiled into an Android build,
/// which has no window or tab to show it in — its launcher icon is the manifest's.
#[cfg(any(desktop, web, test))]
pub(crate) const FRUS_ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// What an application's icon is.
#[derive(Clone, Debug, Default)]
pub enum AppIcon {
    /// The frus logo. What every application has until it chooses otherwise.
    #[default]
    Frus,
    /// The developer's own, as PNG bytes — compiled in with `include_bytes!`.
    Png(Cow<'static, [u8]>),
    /// None of the framework's: the platform's own default (a plain window icon, the browser's
    /// generic tab icon), or whatever the page declares.
    None,
}

impl AppIcon {
    /// The frus logo, the default.
    pub fn frus() -> Self {
        AppIcon::Frus
    }

    /// An icon of your own, from PNG bytes. Square, and at least 64 px a side is a good size;
    /// a window on Windows draws up to 256.
    pub fn from_png(png: impl Into<Cow<'static, [u8]>>) -> Self {
        AppIcon::Png(png.into())
    }

    /// No icon from the framework.
    pub fn none() -> Self {
        AppIcon::None
    }

    /// The PNG to show, if there is one.
    #[cfg(any(desktop, web, test))]
    pub(crate) fn png(&self) -> Option<&[u8]> {
        match self {
            AppIcon::Frus => Some(FRUS_ICON_PNG),
            AppIcon::Png(bytes) => Some(bytes),
            AppIcon::None => Option::None,
        }
    }
}

/// Whether the browser tab's icon should be **replaced** by `icon`.
///
/// `declared` is the `href` of the icon the page already declares, if it does. The default
/// yields to a page that has one — an empty `data:,` counts as *none*, the way a page silences
/// the browser's request for `/favicon.ico` — but an icon the application names explicitly does
/// not: it is the more specific choice.
#[cfg(any(web, test))]
pub(crate) fn replaces_the_pages_icon(icon: &AppIcon, declared: Option<&str>) -> bool {
    let page_has_one = declared.is_some_and(|href| !href.is_empty() && href != "data:,");
    match icon {
        AppIcon::Frus => !page_has_one,
        AppIcon::Png(_) => true,
        AppIcon::None => false,
    }
}

/// The window icon for winit, decoded from the PNG — or `None` when there is no icon or the
/// bytes are not one. A bad icon is a line in the log and a window with the platform's default,
/// never a panic: an icon is not worth the application.
#[cfg(desktop)]
pub(crate) fn window_icon(icon: &AppIcon) -> Option<winit::window::Icon> {
    let png = icon.png()?;
    let image = match frus_image::decode(png) {
        Ok(image) => image,
        Err(err) => {
            log::warn!("the application's icon is not a PNG that can be read: {err}");
            return Option::None;
        }
    };
    match winit::window::Icon::from_rgba(image.rgba().to_vec(), image.width(), image.height()) {
        Ok(icon) => Some(icon),
        Err(err) => {
            log::warn!("the application's icon cannot be used as a window icon: {err}");
            Option::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default is the logo, at the size the window draws, with something in it and
    /// something round it: a logo on a transparent background, not a blank square.
    #[cfg(desktop)]
    #[test]
    fn the_default_icon_is_the_logo_at_256_pixels_with_a_transparent_background() {
        let image = frus_image::decode(FRUS_ICON_PNG).expect("the embedded icon decodes");
        assert_eq!((image.width(), image.height()), (256, 256));
        let alphas: Vec<u8> = image.rgba().chunks_exact(4).map(|p| p[3]).collect();
        assert!(alphas.contains(&255), "a drawn mark");
        assert!(alphas.contains(&0), "and a transparent corner");
        assert!(
            FRUS_ICON_PNG.len() < 64 * 1024,
            "the icon is a few tens of kilobytes, not the artwork's {} kB",
            FRUS_ICON_PNG.len() / 1024
        );
    }

    #[test]
    fn what_each_choice_shows() {
        assert_eq!(AppIcon::default().png(), Some(FRUS_ICON_PNG));
        assert_eq!(AppIcon::frus().png(), Some(FRUS_ICON_PNG));
        assert_eq!(AppIcon::from_png(&b"png"[..]).png(), Some(&b"png"[..]));
        assert_eq!(AppIcon::none().png(), Option::None);
    }

    /// An icon that cannot be read is no icon, and no panic.
    #[cfg(desktop)]
    #[test]
    fn a_window_icon_is_made_for_a_png_and_only_for_a_png() {
        assert!(window_icon(&AppIcon::frus()).is_some());
        assert!(window_icon(&AppIcon::none()).is_none());
        assert!(window_icon(&AppIcon::from_png(&b"not an image"[..])).is_none());
        assert!(window_icon(&AppIcon::from_png(FRUS_ICON_PNG)).is_some());
    }

    /// The default yields to the page's own icon, and to nothing else; an icon the application
    /// names does not yield; none touches nothing.
    #[test]
    fn the_default_yields_to_a_page_that_has_an_icon() {
        let frus = AppIcon::frus();
        assert!(
            replaces_the_pages_icon(&frus, Option::None),
            "no icon declared"
        );
        assert!(replaces_the_pages_icon(&frus, Some("")), "an empty one");
        assert!(
            replaces_the_pages_icon(&frus, Some("data:,")),
            "the page's way of saying none"
        );
        assert!(
            !replaces_the_pages_icon(&frus, Some("/favicon.png")),
            "the page's own"
        );
        let own = AppIcon::from_png(&b"png"[..]);
        assert!(
            replaces_the_pages_icon(&own, Some("/favicon.png")),
            "the application's own wins"
        );
        assert!(!replaces_the_pages_icon(&AppIcon::none(), Option::None));
        assert!(!replaces_the_pages_icon(
            &AppIcon::none(),
            Some("/favicon.png")
        ));
    }
}
