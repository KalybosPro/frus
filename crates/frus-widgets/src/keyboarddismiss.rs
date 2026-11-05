// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! What takes the focus away from a field, and with it the on-screen keyboard
//! (milestone 750): [`TapOutside`], a field's answer to a tap elsewhere, and
//! [`KeyboardDismissBehavior`], a scrollable's answer to a drag.
//!
//! The rule both serve: a person who scrolls the page while typing has not finished
//! typing. On a touch screen, a press outside the focused field moves nothing when it comes
//! down; only a **tap** — a finger lifted where it landed — does what the field's
//! [`TapOutside`] says, and a press that becomes a scroll, a fling or any other drag leaves
//! the focus, and the keyboard, where they were. With a mouse, a press is already a click:
//! the field's answer is given when the button goes down, as desktop applications do.

/// What a **tap outside the focused widget** does to the focus — a field's answer, set with
/// [`TextField::tap_outside`](crate::TextField::tap_outside) or for every field with
/// [`TextFieldTheme::tap_outside`](crate::TextFieldTheme::tap_outside).
///
/// A tap on another field always moves the focus to it, whatever this says: the person
/// chose where to type next. A finger's tap is known when it lifts — a press that becomes
/// a scroll is no tap, and leaves the focus alone — a mouse's click when it presses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TapOutside {
    /// The focus goes where the tap went: to the button or the switch that was tapped, or
    /// nowhere when it landed on nothing that takes the focus. A field that loses it lets
    /// the on-screen keyboard go. The default.
    #[default]
    Unfocus,
    /// The focus stays: a tap on empty space or on a button leaves the field focused, and
    /// its keyboard up. Only another field, the keyboard's own Back or Done, or the
    /// application, takes it away.
    Keep,
}

/// What a **drag on a scrollable** does to the focus — the scrollable's
/// [`keyboard_dismiss_behavior`](crate::SingleChildScrollView::keyboard_dismiss_behavior), or the
/// [`ScrollBehavior`](crate::ScrollBehavior)'s in force where it stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyboardDismissBehavior {
    /// Scrolling leaves the focus where it is: the keyboard stays up while the page moves
    /// under it. The default.
    #[default]
    Manual,
    /// The focus is let go of as soon as a finger starts scrolling the area, and the
    /// keyboard goes with it — a list to read rather than a form to fill.
    OnDrag,
}
