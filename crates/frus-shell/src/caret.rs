//! The caret's **blink** (milestone 513): when a focused field's caret is shown, and when
//! the loop has to wake to show or hide it.
//!
//! Pure, like the recogniser in `gesture.rs`: instants come in as parameters. The rule is
//! the reference's — half a second shown, half a second hidden (`editable_text.dart:109`) —
//! and every change to the field, typed or moved, starts it again from shown, so a caret
//! never blinks out under a finger that is typing (`editable_text.dart:3718`, `:4552`).
//!
//! It runs on the wall clock, not the animation clock: that one is clamped per frame and
//! stands still between frames, and a caret at rest is exactly the case where there are
//! no frames.

use std::time::Duration;

use web_time::Instant;

use frus_widgets::{Edit, WidgetId};

/// How long the caret stays shown, and then hidden.
pub(crate) const HALF_PERIOD: Duration = Duration::from_millis(500);

/// What a caret's blink starts again on: which field has it, where the caret and the
/// selection are, and a digest of what the field holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Signature {
    pub field: WidgetId,
    pub edit: Edit,
    pub value: u64,
}

/// The blink of the one caret on screen.
pub(crate) struct CaretBlink {
    /// When the blink last started, shown; `None` with no caret.
    since: Option<Instant>,
    signature: Option<Signature>,
}

impl CaretBlink {
    pub fn new() -> Self {
        Self {
            since: None,
            signature: None,
        }
    }

    /// This frame's caret, `None` when nothing with a caret has focus. A different field,
    /// caret, selection or value starts the blink again from shown.
    pub fn observe(&mut self, now: Instant, signature: Option<Signature>) {
        if signature != self.signature {
            self.since = signature.map(|_| now);
            self.signature = signature;
        }
    }

    /// Is the caret in its hidden half?
    pub fn hidden(&self, now: Instant) -> bool {
        self.since.is_some_and(|since| {
            now.saturating_duration_since(since).as_millis() / HALF_PERIOD.as_millis() % 2 == 1
        })
    }

    /// When the caret next turns on or off — the loop's next wake for it — or `None` with no
    /// caret, which is no wake at all.
    pub fn next_toggle(&self, now: Instant) -> Option<Instant> {
        let since = self.since?;
        let half = HALF_PERIOD.as_millis();
        let turns = now.saturating_duration_since(since).as_millis() / half + 1;
        Some(since + Duration::from_millis((turns * half) as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn at(cursor: usize) -> Option<Signature> {
        Some(Signature {
            field: WidgetId::from_u64(1),
            edit: Edit {
                cursor,
                anchor: None,
                composing: None,
            },
            value: 7,
        })
    }

    #[test]
    fn the_caret_is_shown_then_hidden_each_half_second() {
        let t0 = Instant::now();
        let mut blink = CaretBlink::new();
        blink.observe(t0, at(3));
        assert!(!blink.hidden(t0), "shown the moment it arrives");
        assert!(!blink.hidden(t0 + ms(499)));
        assert!(
            blink.hidden(t0 + ms(500)),
            "hidden for the second half second"
        );
        assert!(blink.hidden(t0 + ms(999)));
        assert!(!blink.hidden(t0 + ms(1000)), "and shown again");
    }

    /// Typing, or moving the caret, starts the blink again from shown — the same field
    /// looked at again, unchanged, does not.
    #[test]
    fn a_change_starts_it_again_shown() {
        let t0 = Instant::now();
        let mut blink = CaretBlink::new();
        blink.observe(t0, at(3));
        blink.observe(t0 + ms(600), at(3));
        assert!(
            blink.hidden(t0 + ms(700)),
            "nothing changed: still in its hidden half"
        );
        blink.observe(t0 + ms(700), at(4));
        assert!(!blink.hidden(t0 + ms(700)), "a key: shown");
        assert!(
            !blink.hidden(t0 + ms(1100)),
            "for a whole half second after it"
        );
        let mut typed = at(4);
        typed.as_mut().unwrap().value = 8;
        blink.observe(t0 + ms(1150), typed);
        assert!(!blink.hidden(t0 + ms(1600)), "a new value restarts it too");
    }

    /// With nothing to blink there is nothing hidden and nothing to wake for: a field
    /// losing focus must not leave the loop waking twice a second for no caret.
    #[test]
    fn with_no_caret_there_is_no_blink_and_no_wake() {
        let t0 = Instant::now();
        let mut blink = CaretBlink::new();
        assert!(!blink.hidden(t0));
        assert_eq!(blink.next_toggle(t0), None);
        blink.observe(t0, at(3));
        blink.observe(t0 + ms(200), None);
        assert!(!blink.hidden(t0 + ms(700)));
        assert_eq!(blink.next_toggle(t0 + ms(700)), None);
    }

    #[test]
    fn the_loop_wakes_at_the_next_turn() {
        let t0 = Instant::now();
        let mut blink = CaretBlink::new();
        blink.observe(t0, at(3));
        assert_eq!(blink.next_toggle(t0), Some(t0 + ms(500)));
        assert_eq!(blink.next_toggle(t0 + ms(499)), Some(t0 + ms(500)));
        assert_eq!(blink.next_toggle(t0 + ms(500)), Some(t0 + ms(1000)));
        assert_eq!(blink.next_toggle(t0 + ms(1234)), Some(t0 + ms(1500)));
    }
}
