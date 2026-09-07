//! **A value that depends on what a widget is doing**: [`WidgetState`], [`WidgetStates`],
//! [`StateFilter`] and [`WidgetStateProperty`] (`widget_state.dart`).
//!
//! The reference threads one idea through every Material component: a property is not a
//! value but a *function of the states the widget is in*. A button's foreground is one
//! colour at rest, another under a pointer, another while held, another when it cannot be
//! pressed at all — and the API takes all four in one argument rather than four.
//!
//! This framework had no way to say that. Its state rule is [`Theme::state_layer`], one
//! lerp from a ground towards an ink at three fixed opacities, which is the *right answer
//! for a state layer* and no answer at all for a caller who wants a particular colour in
//! a particular state. Since milestone 322 the notes have recorded wanting this.
//!
//! # A step, not a fade
//!
//! A property resolves from a **set of flags**, so it steps: the value for `Hovered` is
//! the value the moment the pointer arrives. That is what the reference does too, and
//! where it wants the change to be gradual it animates *between* two resolved values
//! rather than resolving a fraction (its scrollbar does exactly this,
//! `scrollbar.dart:262`). So a property gives the endpoints, and the widget decides
//! whether to step between them or fade. [`Theme::state_layer`] remains the answer where
//! the whole point is the fade.
//!
//! [`Theme::state_layer`]: crate::Theme::state_layer

/// **One thing a widget can be doing** (`widget_state.dart:168`).
///
/// The reference's eight, unchanged. Not all of them are answerable from outside a
/// widget — see [`Status::states`](crate::Status::states).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WidgetState {
    /// A pointer is over it.
    Hovered,
    /// The keyboard has reached it.
    Focused,
    /// It is being held down.
    Pressed,
    /// **It** is being dragged from one place to another — not that something is being
    /// dragged over it, which is the drop target's business.
    Dragged,
    /// It is one of a set and it is the chosen one: a tab, a radio, a checked box.
    Selected,
    /// It overlaps the content of a scrollable passing beneath it — what an app bar goes
    /// by to decide it is no longer flush with the page.
    ScrolledUnder,
    /// It cannot be interacted with, and so is in none of the four states above.
    Disabled,
    /// It holds something invalid.
    Error,
}

impl WidgetState {
    /// This state's bit in a [`WidgetStates`].
    const fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// **The set of states a widget is in**, as a byte rather than a hash set.
///
/// The reference passes a `Set<WidgetState>`. Eight states fit in eight bits, and a
/// `Copy` set with no allocation is what a paint walk that runs every frame wants — it
/// also means the set can take part in the rebuild hash, which a `HashSet` could not do
/// cheaply.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WidgetStates(u8);

impl WidgetStates {
    /// In no state at all: at rest, reachable, valid, unselected.
    pub const EMPTY: Self = Self(0);

    /// The set holding `state` alone.
    pub const fn of(state: WidgetState) -> Self {
        Self(state.bit())
    }

    /// This set, plus `state`.
    #[must_use]
    pub const fn with(self, state: WidgetState) -> Self {
        Self(self.0 | state.bit())
    }

    /// This set, less `state`.
    #[must_use]
    pub const fn without(self, state: WidgetState) -> Self {
        Self(self.0 & !state.bit())
    }

    /// This set, with `state` present or not according to `on` — the shape a widget
    /// answering for its own selection wants.
    #[must_use]
    pub const fn set(self, state: WidgetState, on: bool) -> Self {
        if on {
            self.with(state)
        } else {
            self.without(state)
        }
    }

    /// Is the widget in this state?
    pub const fn contains(self, state: WidgetState) -> bool {
        self.0 & state.bit() != 0
    }

    /// Is it in none at all?
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<WidgetState> for WidgetStates {
    fn from(state: WidgetState) -> Self {
        Self::of(state)
    }
}

impl std::ops::BitOr for WidgetState {
    type Output = WidgetStates;

    fn bitor(self, other: WidgetState) -> WidgetStates {
        WidgetStates::of(self).with(other)
    }
}

impl std::ops::BitOr<WidgetState> for WidgetStates {
    type Output = WidgetStates;

    fn bitor(self, state: WidgetState) -> WidgetStates {
        self.with(state)
    }
}

/// **What a set of states has to look like** for one entry of a
/// [`WidgetStateProperty`] to answer (`widget_state.dart:27`).
///
/// A bare state is satisfied when the set contains it; the three operators build the rest.
/// The reference spells them `&`, `|` and `~`, and so does this:
///
/// ```
/// use frus_widgets::{StateFilter, WidgetState, WidgetStates};
///
/// let held_but_not_broken =
///     StateFilter::from(WidgetState::Pressed) & !StateFilter::from(WidgetState::Error);
/// assert!(held_but_not_broken.matches(WidgetStates::of(WidgetState::Pressed)));
/// assert!(!held_but_not_broken.matches(
///     WidgetState::Pressed | WidgetState::Error
/// ));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateFilter {
    /// Satisfied by anything at all, including nothing — the reference's
    /// `WidgetState.any` (`widget_state.dart:225`), and what a last entry uses to mean
    /// *otherwise*.
    Any,
    /// Satisfied when the set contains this state.
    Is(WidgetState),
    /// Both.
    And(Box<StateFilter>, Box<StateFilter>),
    /// Either.
    Or(Box<StateFilter>, Box<StateFilter>),
    /// The opposite.
    Not(Box<StateFilter>),
}

impl StateFilter {
    /// Do these states satisfy this filter?
    pub fn matches(&self, states: WidgetStates) -> bool {
        match self {
            StateFilter::Any => true,
            StateFilter::Is(state) => states.contains(*state),
            StateFilter::And(a, b) => a.matches(states) && b.matches(states),
            StateFilter::Or(a, b) => a.matches(states) || b.matches(states),
            StateFilter::Not(inner) => !inner.matches(states),
        }
    }
}

impl From<WidgetState> for StateFilter {
    fn from(state: WidgetState) -> Self {
        StateFilter::Is(state)
    }
}

impl std::ops::BitAnd for StateFilter {
    type Output = StateFilter;

    fn bitand(self, other: StateFilter) -> StateFilter {
        StateFilter::And(Box::new(self), Box::new(other))
    }
}

impl std::ops::BitOr for StateFilter {
    type Output = StateFilter;

    fn bitor(self, other: StateFilter) -> StateFilter {
        StateFilter::Or(Box::new(self), Box::new(other))
    }
}

impl std::ops::Not for StateFilter {
    type Output = StateFilter;

    fn not(self) -> StateFilter {
        StateFilter::Not(Box::new(self))
    }
}

/// **A value chosen by the states a widget is in** (`widget_state.dart:821`).
///
/// Entries are tried in order and the **first match wins**, which is the reference's
/// `WidgetStateMapper` (`widget_state.dart:1009`) and the reason order is worth
/// thinking about: put the narrow cases first.
///
/// ```
/// use frus_core::Color;
/// use frus_widgets::{WidgetState, WidgetStateProperty, WidgetStates};
///
/// let ink = WidgetStateProperty::new()
///     .when(WidgetState::Pressed, Color::rgb8(200, 0, 0))
///     .when(WidgetState::Hovered, Color::rgb8(240, 120, 120))
///     .otherwise(Color::TRANSPARENT);
///
/// // Held *and* hovered, as a pressed widget always is: the narrower entry answers.
/// let held = WidgetState::Pressed | WidgetState::Hovered;
/// assert_eq!(ink.resolve(held), Some(&Color::rgb8(200, 0, 0)));
/// assert_eq!(ink.resolve(WidgetStates::EMPTY), Some(&Color::TRANSPARENT));
/// ```
///
/// A property built from **closures** — the reference's `resolveWith` — is deliberately
/// not offered. A map is `Clone`, `Debug` and comparable, which is what a widget tree
/// that is rebuilt and diffed every frame needs; a boxed closure is none of the three,
/// and the reference itself documents the map as the form to reach for.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetStateProperty<T> {
    entries: Vec<(StateFilter, T)>,
}

impl<T> Default for WidgetStateProperty<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> WidgetStateProperty<T> {
    /// A property that answers nothing yet.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// **The same value whatever the widget is doing** — the reference's
    /// `WidgetStatePropertyAll` (`widget_state.dart:1077`), and what a caller who has one
    /// colour and an API that wants a property reaches for.
    pub fn all(value: T) -> Self {
        Self::new().otherwise(value)
    }

    /// The value to use when `filter` is satisfied, tried **after** everything already
    /// added and **before** everything added later.
    #[must_use]
    pub fn when(mut self, filter: impl Into<StateFilter>, value: T) -> Self {
        self.entries.push((filter.into(), value));
        self
    }

    /// The value to use when nothing above matched. Anything added after it is
    /// unreachable, `Any` being satisfied by every set.
    #[must_use]
    pub fn otherwise(self, value: T) -> Self {
        self.when(StateFilter::Any, value)
    }

    /// The first value whose filter these states satisfy, or `None` when none does —
    /// which means *say nothing*, and leaves the widget or the theme to answer, exactly
    /// as the reference's nullable properties do.
    pub fn resolve(&self, states: WidgetStates) -> Option<&T> {
        self.entries
            .iter()
            .find(|(filter, _)| filter.matches(states))
            .map(|(_, value)| value)
    }

    /// Has anything been said at all?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The set is a byte, and it behaves like the reference's set.
    #[test]
    fn a_set_of_states_is_eight_bits() {
        let states = WidgetState::Hovered | WidgetState::Focused;
        assert!(states.contains(WidgetState::Hovered));
        assert!(states.contains(WidgetState::Focused));
        assert!(!states.contains(WidgetState::Pressed));
        assert!(WidgetStates::EMPTY.is_empty());
        assert!(!states.is_empty());
        assert_eq!(
            states.without(WidgetState::Focused),
            WidgetState::Hovered.into()
        );
        assert_eq!(
            WidgetStates::EMPTY.set(WidgetState::Selected, true),
            WidgetStates::of(WidgetState::Selected)
        );
        assert_eq!(
            states.set(WidgetState::Hovered, false),
            WidgetStates::of(WidgetState::Focused)
        );
        assert_eq!(
            std::mem::size_of::<WidgetStates>(),
            1,
            "a set that costs a byte can go in a paint walk and in a hash"
        );
    }

    /// **The first match wins** (`widget_state.dart:1009`), so a narrow entry has to come
    /// before a wide one — a pressed widget is nearly always hovered as well.
    #[test]
    fn the_first_entry_that_matches_answers() {
        let property = WidgetStateProperty::new()
            .when(WidgetState::Pressed, "held")
            .when(WidgetState::Hovered, "under a pointer")
            .otherwise("at rest");

        let held = WidgetState::Pressed | WidgetState::Hovered;
        assert_eq!(property.resolve(held), Some(&"held"));
        assert_eq!(
            property.resolve(WidgetStates::of(WidgetState::Hovered)),
            Some(&"under a pointer")
        );
        assert_eq!(property.resolve(WidgetStates::EMPTY), Some(&"at rest"));
    }

    /// And a property with nothing to say says **nothing**, rather than a default that
    /// would silently outrank the widget's own answer.
    #[test]
    fn a_property_may_answer_nothing() {
        let only_held = WidgetStateProperty::new().when(WidgetState::Pressed, 1);
        assert_eq!(only_held.resolve(WidgetStates::EMPTY), None);
        assert_eq!(
            only_held.resolve(WidgetStates::of(WidgetState::Pressed)),
            Some(&1)
        );
        assert!(WidgetStateProperty::<u8>::new().is_empty());
        assert_eq!(
            WidgetStateProperty::all(7).resolve(WidgetStates::of(WidgetState::Error)),
            Some(&7),
            "and one that says the same thing always says it in every state"
        );
    }

    /// The three operators, which are the reference's three (`widget_state.dart:49`).
    #[test]
    fn filters_combine_the_way_the_reference_spells_them() {
        let f = |state| StateFilter::from(state);
        let both = f(WidgetState::Hovered) & f(WidgetState::Focused);
        assert!(both.matches(WidgetState::Hovered | WidgetState::Focused));
        assert!(!both.matches(WidgetStates::of(WidgetState::Hovered)));

        let either = f(WidgetState::Hovered) | f(WidgetState::Focused);
        assert!(either.matches(WidgetStates::of(WidgetState::Focused)));
        assert!(!either.matches(WidgetStates::of(WidgetState::Pressed)));

        let reachable = !f(WidgetState::Disabled);
        assert!(reachable.matches(WidgetStates::EMPTY));
        assert!(!reachable.matches(WidgetStates::of(WidgetState::Disabled)));

        assert!(StateFilter::Any.matches(WidgetStates::EMPTY));
    }
}
