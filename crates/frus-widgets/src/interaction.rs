//! Widget identity, keyboard keys and interaction state.
//!
//! A [`WidgetId`] identifies a widget by its **position** in the tree (the path
//! root → child indices), stable from one frame to the next for as long as the
//! structure does not change. It is the founding brick of reconciliation, and
//! what makes it possible to track hover, press and **focus**.

/// The shape of the **system cursor** a widget can request for a given sub-region
/// (milestone 205). The shell translates it to the window's cursor; widgets stay
/// independent of the windowing layer.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Cursor {
    /// The default arrow.
    #[default]
    Default,
    /// A hand (a clickable element).
    Pointer,
    /// A vertical bar (a text entry area).
    Text,
}

/// A widget's positional identity: a hash of its path in the tree.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WidgetId(u64);

impl WidgetId {
    /// The root's identity.
    pub(crate) const ROOT: WidgetId = WidgetId(0xcbf29ce484222325);

    /// The identity's raw value, used to tag primitives.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Rebuilds an identity from its raw value (the inverse of [`as_u64`]) — to
    /// route an action coming from an external layer (accessibility).
    ///
    /// [`as_u64`]: WidgetId::as_u64
    pub fn from_u64(raw: u64) -> WidgetId {
        WidgetId(raw)
    }

    /// Derives the identity of this widget's `index`-th child (positional).
    pub(crate) fn child(self, index: usize) -> WidgetId {
        let mut h = self.0 ^ (index as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
        WidgetId(h)
    }

    /// Derives a child's identity **by key** (stable whatever its position).
    /// Distinct from [`WidgetId::child`] (a different constant and shift).
    pub(crate) fn keyed(self, key: u64) -> WidgetId {
        let mut h = self.0 ^ key.wrapping_add(0x517c_c1b7_2722_0a95);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 31;
        WidgetId(h)
    }
}

/// A widget's response to a key received during the **leaf→root bubble** (the
/// focused one first, then its ancestors for as long as the result is `Ignored`).
#[derive(Clone, Debug, PartialEq)]
pub enum KeyResponse<Msg> {
    /// Not concerned: the key keeps bubbling up.
    Ignored,
    /// Consumed (with an optional message to emit): the bubbling stops.
    Handled(Option<Msg>),
    /// Not consumed, but the bubbling stops, and no fallback is attempted.
    Skip,
}

/// A key passed to the focused widget.
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    /// Typed text (one or more characters) — also used for pasting.
    Text(String),
    /// Backspace (deletes the selection, otherwise the character to the left).
    Backspace,
    /// Forward delete (the selection, otherwise the character to the right).
    Delete,
    /// Enter.
    Enter,
    /// Left arrow (`shift`: extends the selection; `word`: jumps a word, Ctrl).
    Left { shift: bool, word: bool },
    /// Right arrow (`shift`: extends it; `word`: jumps a word, Ctrl).
    Right { shift: bool, word: bool },
    /// Escape (close or cancel) — routed leaf→root, never to editing.
    Escape,
    /// Start of line (`doc`: the start of the whole **field**, Ctrl).
    Home { shift: bool, doc: bool },
    /// End of line (`doc`: the end of the whole **field**, Ctrl).
    End { shift: bool, doc: bool },
}

/// A widget's visual pointer interaction state.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Interaction {
    /// Neither hovered nor pressed.
    #[default]
    None,
    /// The pointer is over it.
    Hovered,
    /// The pointer is pressed on this widget.
    ///
    /// This is the **flag**: true from the first frame the finger is down. What a paint
    /// wants is almost always [`Status::press_progress`], the same thing as a fade.
    Pressed,
}

/// A widget's complete status for one frame: pointer interaction, focus,
/// caret and selection (for fields), animation progress and opacity.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Status {
    pub interaction: Interaction,
    pub focused: bool,
    /// The caret's (character) index, if this widget is a focused field.
    pub cursor: Option<usize>,
    /// The selected `(start, end)` range, in character indices.
    pub selection: Option<(usize, usize)>,
    /// The `(start, end)` range **being composed** by the IME (provisional,
    /// underlined text); `None` outside composition. In character indices.
    pub composing: Option<(usize, usize)>,
    /// Is a drag currently **over this widget**, and would it be accepted? Only
    /// ever true for a [`crate::DragTarget`]; it is what lets one paint the "drop it
    /// here" state itself rather than have the shell paint it from outside.
    pub drag_over: bool,
    /// The hover transition's progress (`0.0..=1.0`).
    pub hover_progress: f32,
    /// The focus transition's progress (`0.0..=1.0`).
    pub focus_progress: f32,
    /// The **press** transition's progress (`0.0..=1.0`): the animated form of
    /// [`Interaction::Pressed`], as `hover_progress` is the animated form of
    /// [`Interaction::Hovered`].
    ///
    /// A press used to be a flag and nothing else, so every state layer and every held
    /// radius in the crate reached full the instant a finger landed and vanished the
    /// instant it left. The reference fades its press highlight over 200 ms against the
    /// 50 ms it gives hover and focus (`ink_well.dart:995`) — the press is the *slower*
    /// of the two, which is the opposite of what a flag does.
    ///
    /// `interaction` stays for the decisions that really are discrete — is this the
    /// widget being held? — and this is what the **paint** reads.
    pub press_progress: f32,
    /// The opacity to apply (a fade-in); `1.0` = opaque.
    pub opacity: f32,
    /// The widget's own animated value (a switch's `0 → 1`, for instance), driven
    /// by `Widget::anim_target`.
    pub value: f32,
    /// The time elapsed (in seconds) since start-up — for continuous,
    /// time-driven animations (a `CircularProgressIndicator`, for instance).
    pub time: f32,
    /// The **interpolated** color of an animated background
    /// (`Container::animated_color`) while in transition; `None` = no animated
    /// color, and the widget uses its own.
    pub anim_color: Option<frus_core::Color>,
    /// The **interpolated** corner radius (`Container::animated_radius`) while in
    /// transition; `None` = a fixed radius, and the widget uses its own.
    pub anim_radius: Option<frus_core::BorderRadius>,
    /// This widget's **retained** vertical scroll (in px), for widgets that scroll
    /// their own content (a multi-line field); `0` otherwise.
    pub scroll_y: f32,
    /// The pointer's **absolute** position when it hovers an **interactive sub-region** of
    /// this widget (milestone 208); `None` otherwise. The widget brings it back to local
    /// coordinates through its `bounds` to highlight the targeted zone (a suffix icon…).
    /// Set by the shell from `cursor_icon`.
    pub hover_cursor: Option<frus_core::Point>,
}

impl Status {
    /// **The states this widget is in**, as a [`WidgetStateProperty`] resolves against
    /// (`widget_state.dart:168`).
    ///
    /// Three of the eight, because three are all a status honestly knows: hovered,
    /// focused and pressed. `Selected`, `Disabled`, `Error`, `Dragged` and
    /// `ScrolledUnder` are the **widget's own** to add with
    /// [`WidgetStates::set`] — nothing outside a checkbox knows whether it is ticked, and
    /// a status that guessed would be wrong for every widget that never selects anything.
    ///
    /// It reads the **flags**, not the fades: a property is a step between values, and
    /// where the change should be gradual the widget animates between two resolved values
    /// rather than resolving a fraction. See the module note.
    ///
    /// [`WidgetStateProperty`]: crate::WidgetStateProperty
    /// [`WidgetStates::set`]: crate::WidgetStates::set
    pub fn states(&self) -> crate::widgetstate::WidgetStates {
        use crate::widgetstate::{WidgetState, WidgetStates};
        WidgetStates::EMPTY
            .set(WidgetState::Hovered, self.interaction != Interaction::None)
            .set(
                WidgetState::Pressed,
                self.interaction == Interaction::Pressed,
            )
            .set(WidgetState::Focused, self.focused)
    }
}

impl Default for Status {
    fn default() -> Self {
        Self {
            interaction: Interaction::None,
            focused: false,
            cursor: None,
            selection: None,
            composing: None,
            drag_over: false,
            hover_progress: 0.0,
            focus_progress: 0.0,
            press_progress: 0.0,
            opacity: 1.0,
            value: 0.0,
            time: 0.0,
            anim_color: None,
            anim_radius: None,
            scroll_y: 0.0,
            hover_cursor: None,
        }
    }
}

/// The input state retained by the runtime, passed to the interface build.
#[derive(Copy, Clone, Debug, Default)]
pub struct InputState {
    /// The widget currently hovered.
    pub hovered: Option<WidgetId>,
    /// The widget the pointer is pressed on.
    pub pressed: Option<WidgetId>,
    /// The widget that has keyboard focus.
    pub focused: Option<WidgetId>,
    /// The pointer's absolute position when it hovers an interactive sub-region of the
    /// hovered widget (milestone 208); `None` otherwise. Set by the shell (see
    /// `Status::hover_cursor`).
    pub hover_cursor: Option<frus_core::Point>,
}

impl InputState {
    /// A given widget's status.
    pub(crate) fn status_for(&self, id: WidgetId) -> Status {
        let interaction = if self.pressed == Some(id) && self.hovered == Some(id) {
            Interaction::Pressed
        } else if self.hovered == Some(id) {
            Interaction::Hovered
        } else {
            Interaction::None
        };
        Status {
            interaction,
            focused: self.focused == Some(id),
            cursor: None,
            selection: None,
            composing: None,
            drag_over: false,
            hover_progress: 0.0,
            focus_progress: 0.0,
            press_progress: 0.0,
            opacity: 1.0,
            value: 0.0,
            time: 0.0,
            anim_color: None,
            anim_radius: None,
            scroll_y: 0.0,
            // The pointer's position only for the hovered widget (otherwise `None`).
            hover_cursor: if self.hovered == Some(id) {
                self.hover_cursor
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_yields_same_id() {
        assert_eq!(
            WidgetId::ROOT.child(0).child(2),
            WidgetId::ROOT.child(0).child(2)
        );
    }

    #[test]
    fn different_paths_differ() {
        assert_ne!(WidgetId::ROOT.child(0), WidgetId::ROOT.child(1));
        assert_ne!(
            WidgetId::ROOT.child(0).child(1),
            WidgetId::ROOT.child(1).child(0)
        );
        assert_ne!(WidgetId::ROOT, WidgetId::ROOT.child(0));
    }

    #[test]
    fn keyed_is_stable_and_distinct() {
        // The same key under the same parent → the same identity (position-independent).
        assert_eq!(WidgetId::ROOT.keyed(7), WidgetId::ROOT.keyed(7));
        // Different keys → different identities.
        assert_ne!(WidgetId::ROOT.keyed(7), WidgetId::ROOT.keyed(8));
        // A key does not collide with a positional index of the same value.
        assert_ne!(WidgetId::ROOT.keyed(0), WidgetId::ROOT.child(0));
        // Different parents → different identities for the same key.
        assert_ne!(
            WidgetId::ROOT.child(0).keyed(7),
            WidgetId::ROOT.child(1).keyed(7)
        );
    }

    #[test]
    fn status_precedence_and_focus() {
        let id = WidgetId::ROOT.child(0);
        let other = WidgetId::ROOT.child(1);

        let hovered = InputState {
            hovered: Some(id),
            ..Default::default()
        };
        assert_eq!(hovered.status_for(id).interaction, Interaction::Hovered);

        let pressed = InputState {
            hovered: Some(id),
            pressed: Some(id),
            ..Default::default()
        };
        assert_eq!(pressed.status_for(id).interaction, Interaction::Pressed);

        // Pressed, but the pointer is elsewhere → not "Pressed".
        let moved_away = InputState {
            hovered: Some(other),
            pressed: Some(id),
            ..Default::default()
        };
        assert_eq!(moved_away.status_for(id).interaction, Interaction::None);

        // Focus is independent of pointer interaction.
        let focused = InputState {
            focused: Some(id),
            ..Default::default()
        };
        assert!(focused.status_for(id).focused);
        assert!(!focused.status_for(other).focused);
    }
}
