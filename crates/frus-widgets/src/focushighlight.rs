// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! Whether the focus is **shown** (milestone 752): [`FocusHighlightMode`], which follows the
//! kind of input used last, and [`FocusHighlightStrategy`], an application's say in it.
//!
//! The focus is always kept; what changes is whether a ring is drawn around the widget
//! that holds it. A person moving through the screen with a keyboard or a remote needs to
//! see where they are. A person tapping does not, and a ring left behind on a button after
//! a tap — or on the button the focus returns to when a page is popped — is noise on a
//! touch screen.

/// How the focus is highlighted now: decided by the **kind of input used last**.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusHighlightMode {
    /// The last input was a touch or a mouse press: no focus ring is drawn. Where an
    /// application starts.
    #[default]
    Touch,
    /// The last input was a key — a hardware keyboard, a remote's arrows, Tab — or a move
    /// of the focus asked for by an assistive technology: the widget with the focus shows
    /// its ring.
    Traditional,
}

/// How the [`FocusHighlightMode`] is decided — an application's
/// `focus_highlight_strategy`.
///
/// Not every key is an interaction with the keyboard. The system's **Back** (a phone's back
/// button or gesture, a browser's Back) is navigation, and the keys an **on-screen
/// keyboard** sends to the field it types into are typing on a touch screen: neither turns
/// the rings on. A focus the application moves itself, or the one a popped page hands back,
/// is shown or not as the mode already says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusHighlightStrategy {
    /// The mode follows the last input: a touch or a click turns the rings off, a key turns
    /// them on. The default.
    #[default]
    Automatic,
    /// Never a ring, whatever the input — an application that draws its own.
    AlwaysTouch,
    /// Always the ring — a television or a kiosk driven by a remote, where the focus is
    /// the only cursor there is.
    AlwaysTraditional,
}

impl FocusHighlightStrategy {
    /// The mode after an input: `keyboard` when it was a key that counts as one,
    /// `false` for a touch or a click.
    pub fn mode_after(self, keyboard: bool) -> FocusHighlightMode {
        match self {
            Self::Automatic if keyboard => FocusHighlightMode::Traditional,
            Self::Automatic | Self::AlwaysTouch => FocusHighlightMode::Touch,
            Self::AlwaysTraditional => FocusHighlightMode::Traditional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strategy_decides_what_each_input_shows() {
        use FocusHighlightMode::{Touch, Traditional};
        let automatic = FocusHighlightStrategy::Automatic;
        assert_eq!(automatic.mode_after(true), Traditional);
        assert_eq!(automatic.mode_after(false), Touch);
        for keyboard in [true, false] {
            assert_eq!(
                FocusHighlightStrategy::AlwaysTouch.mode_after(keyboard),
                Touch
            );
            assert_eq!(
                FocusHighlightStrategy::AlwaysTraditional.mode_after(keyboard),
                Traditional
            );
        }
        assert_eq!(FocusHighlightMode::default(), Touch);
    }
}
