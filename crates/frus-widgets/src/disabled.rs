//! **One disabled state**, shared by every control that can be unavailable.
//!
//! Milestone 320 gave `Chip` and `SegmentedControl` an `enabled` flag and ended by
//! noting that each control was carrying its own copy of the rule. That undersold it:
//! five widgets had the flag with the same two colours written out by hand in each, and
//! **ten more had no way to be disabled at all**. A form could not grey out a checkbox,
//! a switch, a radio, a slider or a dropdown — the five controls a form is mostly made
//! of. That is a gap in the framework, not in any one widget.
//!
//! ## The two colours
//!
//! A disabled control **flattens**; it does not fade. Every variant collapses to
//! [`disabled_container`] under [`disabled_content`], so that unavailable reads as
//! unavailable rather than as a quieter version of whatever the control was. The
//! difference shows most on a *selected* control: fading the accent gives a pale accent,
//! which reads as *quietly selected*, and a disabled filter is not offering a dimmer
//! answer — it is not offering.
//!
//! The split is **container against content**, not widget against widget, and the switch
//! is what proves it: the reference disables its *track* at 12 % and its *thumb* at 38 %,
//! one control taking both halves of the same rule. So a checkbox's tick box and a radio's
//! dot take [`disabled_content`] even though they look like containers — they are the mark
//! itself, with nothing behind them — while a button's surface, a chip's fill and that
//! switch track take [`disabled_container`].
//!
//! The one thing that is neither is a mark drawn **on** a disabled fill: a disabled
//! selected checkbox's tick, a disabled selected switch's thumb. Another translucent
//! `on_surface` there would sink into the 38 % it sits on, so it punches through in
//! [`disabled_mark`] instead.
//!
//! ## Greying out is the easy half
//!
//! The colours are the visible quarter of the contract. A control that is disabled must
//! also be **inert**, and the three parts that are not visible are the ones that get
//! forgotten — milestone 320 found a `Chip` whose delete cross would have stayed live on
//! an inert chip, and it would have looked perfect in a screenshot. So the whole contract
//! is written down here and [checked against the source](self#the-guard):
//!
//! 1. **the press goes nowhere** — `on_click` (or `positional_click`) yields `None`;
//! 2. **no ink** — a splash promises that something is happening, and nothing is;
//! 3. **out of the tab order** — `focusable` is false, so Tab does not stop at a control
//!    that cannot be operated;
//! 4. **announced as disabled** rather than falling silent. A reader that simply stopped
//!    hearing about a filter would be told the filter had gone away, which is a different
//!    and worse fact.
//!
//! What a disabled control keeps is its **answer**: `toggled`, `checked`, the chosen
//! segment. Read-only is not invisible, and the current state is still owed to a reader
//! who cannot change it.
//!
//! ## The guard
//!
//! `every_control_with_an_enabled_flag_honours_all_four` reads the crate's own sources,
//! finds every widget carrying `enabled: bool`, and insists that each of the four hooks
//! it implements mentions that flag. It is the same instrument as
//! [`crate::transparent`]'s, written for the same reason: a rule kept by convention is
//! kept by whoever remembered it.

use frus_core::Color;

use crate::theme::Theme;

/// A disabled container's opacity over the surface — the reference's 12 %.
pub const DISABLED_CONTAINER_OPACITY: f32 = 0.12;

/// A disabled label's, glyph's or outline's opacity — the reference's 38 %.
pub const DISABLED_CONTENT_OPACITY: f32 = 0.38;

/// The surface a disabled control sits on: `on_surface` at
/// [`DISABLED_CONTAINER_OPACITY`], whatever the control's variant would have been.
pub fn disabled_container(theme: &Theme) -> Color {
    theme.scheme.on_surface.fade(DISABLED_CONTAINER_OPACITY)
}

/// What is drawn on a disabled control — its label, its glyph, its outline, and a
/// selection control's own mark: `on_surface` at [`DISABLED_CONTENT_OPACITY`].
pub fn disabled_content(theme: &Theme) -> Color {
    theme.scheme.on_surface.fade(DISABLED_CONTENT_OPACITY)
}

/// A mark drawn **on** a disabled fill — a disabled selected checkbox's tick, a disabled
/// selected switch's thumb.
///
/// Opaque `surface`, not another translucent `on_surface`: stacking 38 % on 38 % leaves
/// the two within a few percent of each other, and the tick disappears into the box it is
/// supposed to be inside.
pub fn disabled_mark(theme: &Theme) -> Color {
    theme.scheme.surface
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four-part contract, checked against the crate's own source rather than
    /// against a list someone kept up to date by hand.
    ///
    /// A widget that carries `enabled` and forgets one of the hooks is not a widget that
    /// looks wrong — it is a widget that looks right and answers a tap. That is the exact
    /// failure milestone 320 caught by hand in `Chip`, and by hand is not a method.
    #[test]
    fn every_control_with_an_enabled_flag_honours_all_four() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        // `on_click` and `positional_click` are alternatives — a control answers a tap
        // through one or the other — so they are checked as a pair.
        let hooks: [&[&str]; 6] = [
            &["fn on_click(", "fn positional_click("],
            &["fn ink("],
            &["fn focusable("],
            &["fn semantics("],
            // Not every control is operated by a tap. A slider is dragged and a field is
            // typed into, and a disabled one of either that still answered would be inert
            // only to the gesture nobody was using on it.
            &["fn draggable(", "fn on_drag(", "fn on_drag_delta("],
            &["fn on_key(", "fn on_edit("],
        ];
        let mut offenders: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(dir).expect("the crate's source") {
            let path = entry.expect("an entry").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("a module");
            // Only the widgets that claim to be disableable; the rest are not in scope.
            if !src.contains("enabled: bool,") {
                continue;
            }
            // The tests at the foot of a module are full of `.enabled(false)` and would
            // pass the check for the implementation.
            let src = src.split("\nmod tests {").next().unwrap_or(&src).to_owned();
            let name = path.file_name().expect("a name").to_string_lossy().to_string();
            for group in hooks {
                for hook in group {
                    // **Every** occurrence, not the first: a module often holds more than
                    // one widget — a chip and its delete cross, a group and its options —
                    // and checking only the first would clear the very pairing that makes
                    // a live control on an inert parent possible.
                    for (at, _) in src.match_indices(hook) {
                        // Each hook a module *implements* must consult the flag; one it
                        // does not implement is not an omission (a control with no ink has
                        // no splash to suppress).
                        let rest = &src[at + hook.len()..];
                        let body = rest.split("\n    }").next().unwrap_or(rest);
                        // A hook that is **unconditionally** inert needs no flag: a field
                        // answers taps through `positional_click` and returns a bare
                        // `None` from `on_click`, which is already the disabled answer for
                        // every state there is.
                        let inert = body.trim_end().ends_with("None");
                        if !body.contains("enabled") && !inert {
                            offenders.push(format!("{name}::{hook} (byte {at})"));
                        }
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "these look available to the tap, the tab or the reader while disabled: {offenders:?}"
        );
    }

    /// The pair is only ever right in this order: a 12 % surface under a 38 % label.
    /// Inverting them is the kind of slip that still renders something plausible, so the
    /// two constants are checked where they are written rather than at test time — the
    /// crate does not compile with them the wrong way round.
    const _: () = assert!(DISABLED_CONTAINER_OPACITY < DISABLED_CONTENT_OPACITY);

    #[test]
    fn the_container_is_quieter_than_the_content() {
        // And that the ordering survives the resolution through a theme, in both.
        for theme in [Theme::dark(), Theme::light()] {
            assert!(disabled_container(&theme).a < disabled_content(&theme).a);
        }
    }
}
