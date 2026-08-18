//! **Shortcuts and actions**: a key combination, what it means, and who answers.
//!
//! The reference splits this in two on purpose, and the split is the whole design:
//!
//! - [`Shortcuts`] maps a **keystroke to an intent** — *Ctrl+S means «save»*. It knows
//!   nothing about what saving does.
//! - [`Actions`] maps an **intent to a message** — *«save» means `Msg::Save`*. It knows
//!   nothing about which key got here.
//!
//! The point of the indirection is that the two can be replaced independently. A dialog
//! can rebind the keys without touching the handlers; a subtree can answer «save»
//! differently from the page around it, and the innermost answer wins — the same rule as
//! focus, resolved the same way.
//!
//! When there is no reason to name an intent, [`CallbackShortcuts`] binds the keystroke
//! straight to a message and skips the ceremony.
//!
//! ```ignore
//! Actions::new(child)
//!     .action(Intent("save"), Msg::Save)
//!     .action(Intent("close"), Msg::Close)
//! // …with, anywhere inside it:
//! Shortcuts::new(form)
//!     .bind(KeyStroke::new(ShortcutKey::Char('s')).ctrl(), Intent("save"))
//! ```
//!
//! ## Which scope answers
//!
//! A binding applies when **focus is inside the subtree that declared it**, and the
//! innermost such subtree wins. With nothing focused, every scope is a candidate and the
//! innermost declared still wins.
//!
//! ## Typing beats a bare letter
//!
//! A keystroke with no Ctrl, Alt or Meta goes to the focused text field first, if there is
//! one. Otherwise a `Shortcuts` binding on the letter `a` would make every field in the
//! application impossible to type in — the reference reaches the same conclusion through
//! its text-editing shortcuts taking priority in the focused scope.
//!
//! ## Not here: `ShortcutRegistrar`
//!
//! The reference has a registry a deep widget can *register into at runtime*, because its
//! tree is retained and a widget's build is not re-run on every frame. Here the view is
//! rebuilt from the state each frame, so declaring the binding where it applies is the
//! same thing with none of the bookkeeping. That is an architectural difference, not a
//! missing feature.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{sizing_of, Widget};

/// A key a shortcut can be bound to — a **general** vocabulary, unlike
/// [`Key`](crate::Key), which is the one text editing needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutKey {
    /// A printable key, matched **case-insensitively**: `Ctrl+S` and `Ctrl+s` are the
    /// same shortcut, and the caller should not have to think about Caps Lock.
    Char(char),
    /// Enter / Return.
    Enter,
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Space.
    Space,
    /// Backspace.
    Backspace,
    /// Forward delete.
    Delete,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// A function key, `F(1)` through `F(24)`.
    F(u8),
}

/// A key plus its modifiers — both the **pattern** a shortcut is bound to and the
/// **description** of what was actually pressed. One type, because they are compared to
/// each other and a second one would only be the first with the fields renamed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyStroke {
    /// The key itself.
    pub key: ShortcutKey,
    /// Control (or Command on macOS, where the shell maps it here).
    pub ctrl: bool,
    /// Shift.
    pub shift: bool,
    /// Alt / Option.
    pub alt: bool,
    /// The platform's "logo" key, when it is not already folded into `ctrl`.
    pub meta: bool,
}

impl KeyStroke {
    /// A bare key, no modifiers.
    pub const fn new(key: ShortcutKey) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        }
    }

    /// With Control held.
    pub const fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// With Shift held.
    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// With Alt held.
    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// With the logo key held.
    pub const fn meta(mut self) -> Self {
        self.meta = true;
        self
    }

    /// Does this stroke carry a modifier that makes it a **command** rather than typing?
    /// Shift alone does not: `Shift+A` is a capital A.
    pub const fn is_command(&self) -> bool {
        self.ctrl || self.alt || self.meta
    }

    /// Do a pressed stroke and a bound one match? Letters are compared without case, so a
    /// binding written `Char('s')` answers a Shift-less `S` from a keyboard with Caps Lock
    /// on.
    pub fn matches(&self, bound: &KeyStroke) -> bool {
        let same_key = match (self.key, bound.key) {
            (ShortcutKey::Char(a), ShortcutKey::Char(b)) => {
                a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase())
            }
            (a, b) => a == b,
        };
        same_key
            && self.ctrl == bound.ctrl
            && self.shift == bound.shift
            && self.alt == bound.alt
            && self.meta == bound.meta
    }
}

/// What a keystroke **means**, named rather than implemented: `Intent("save")`.
///
/// A string rather than a type, because the whole point is that the widget binding the key
/// and the widget answering it need not know about one another — and a name reads in a
/// dump, which a type id does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent(pub &'static str);

/// The shared body: one child, its box, and one of the three tables.
struct Wrapper<Msg> {
    child: Vec<Box<dyn Widget<Msg>>>,
    focusable: bool,
    shortcuts: Vec<(KeyStroke, Intent)>,
    callbacks: Vec<(KeyStroke, Msg)>,
    actions: Vec<(Intent, Msg)>,
    listeners: Vec<(Intent, Msg)>,
}

impl<Msg> Wrapper<Msg> {
    fn new(child: Box<dyn Widget<Msg>>) -> Self {
        Self {
            child: vec![child],
            focusable: false,
            shortcuts: Vec::new(),
            callbacks: Vec::new(),
            actions: Vec::new(),
            listeners: Vec::new(),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Wrapper<Msg> {
    fn style(&self) -> Style {
        sizing_of(self.child[0].style())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        sizing_of(self.child[0].style_themed(theme))
    }

    fn build_themed(&self, theme: &Theme) {
        self.child[0].build_themed(theme);
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn focusable(&self) -> bool {
        self.focusable
    }

    fn shortcut_bindings(&self) -> &[(KeyStroke, Intent)] {
        &self.shortcuts
    }

    fn shortcut_callbacks(&self) -> &[(KeyStroke, Msg)] {
        &self.callbacks
    }

    fn action_bindings(&self) -> &[(Intent, Msg)] {
        &self.actions
    }

    fn action_listeners(&self) -> &[(Intent, Msg)] {
        &self.listeners
    }
}

/// Binds **keystrokes to intents** for its subtree. See the module's documentation for
/// why that is two steps rather than one.
pub struct Shortcuts<Msg>(Wrapper<Msg>);

impl<Msg> Shortcuts<Msg> {
    /// A shortcut scope around `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self(Wrapper::new(Box::new(child)))
    }

    /// Binds a keystroke to an intent. A later binding for the same stroke replaces an
    /// earlier one, so a caller can override a table it did not write.
    pub fn bind(mut self, stroke: KeyStroke, intent: Intent) -> Self {
        self.0.shortcuts.retain(|(s, _)| !s.matches(&stroke));
        self.0.shortcuts.push((stroke, intent));
        self
    }
}

/// Binds **keystrokes straight to messages**, for when naming an intent would be
/// ceremony — the reference's `CallbackShortcuts`.
pub struct CallbackShortcuts<Msg>(Wrapper<Msg>);

impl<Msg: Clone> CallbackShortcuts<Msg> {
    /// A shortcut scope around `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self(Wrapper::new(Box::new(child)))
    }

    /// Binds a keystroke to the message it sends.
    pub fn bind(mut self, stroke: KeyStroke, msg: Msg) -> Self {
        self.0.callbacks.retain(|(s, _)| !s.matches(&stroke));
        self.0.callbacks.push((stroke, msg));
        self
    }
}

/// Answers **intents** for its subtree: the innermost `Actions` that has the intent is
/// the one that answers it.
pub struct Actions<Msg>(Wrapper<Msg>);

impl<Msg> Actions<Msg> {
    /// An action scope around `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self(Wrapper::new(Box::new(child)))
    }

    /// What this scope does about `intent`.
    pub fn action(mut self, intent: Intent, msg: Msg) -> Self {
        self.0.actions.push((intent, msg));
        self
    }
}

/// **Watches** intents without answering them: its message is sent as well as, not
/// instead of, whatever the answering [`Actions`] sends.
///
/// For the things that want to know an action happened — an undo stack, a telemetry
/// counter, a status line — without becoming the one that performs it.
pub struct ActionListener<Msg>(Wrapper<Msg>);

impl<Msg> ActionListener<Msg> {
    /// A listener around `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self(Wrapper::new(Box::new(child)))
    }

    /// Also send `msg` whenever `intent` is invoked inside this subtree.
    pub fn on(mut self, intent: Intent, msg: Msg) -> Self {
        self.0.listeners.push((intent, msg));
        self
    }
}

/// A **focus stop that carries its own keys**: focusable, with its own shortcut and action
/// tables, so a control can answer the keyboard for itself while it has focus.
///
/// The reference's `FocusableActionDetector`, and the sixth of the focus widgets — it
/// waited for this milestone because it is made of shortcuts as much as of focus. It is
/// the composition, given one name because the three are always wanted together: a menu
/// item that answers Enter, a card that answers Delete, a canvas that answers arrows.
pub struct FocusableActionDetector<Msg>(Wrapper<Msg>);

impl<Msg: Clone> FocusableActionDetector<Msg> {
    /// A focusable, key-answering wrapper around `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut w = Wrapper::new(Box::new(child));
        w.focusable = true;
        Self(w)
    }

    /// Whether it takes focus at all. A disabled control should say `false` here rather
    /// than keep an inert focus stop that Tab still lands on.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.focusable = enabled;
        self
    }

    /// Binds a keystroke to an intent, for while focus is inside.
    pub fn bind(mut self, stroke: KeyStroke, intent: Intent) -> Self {
        self.0.shortcuts.retain(|(s, _)| !s.matches(&stroke));
        self.0.shortcuts.push((stroke, intent));
        self
    }

    /// Binds a keystroke straight to a message.
    pub fn on_key(mut self, stroke: KeyStroke, msg: Msg) -> Self {
        self.0.callbacks.retain(|(s, _)| !s.matches(&stroke));
        self.0.callbacks.push((stroke, msg));
        self
    }

    /// What this control does about `intent`.
    pub fn action(mut self, intent: Intent, msg: Msg) -> Self {
        self.0.actions.push((intent, msg));
        self
    }
}

/// Hands **every keystroke** in its subtree to a closure — the reference's
/// `KeyboardListener`, and the escape hatch for anything the tables above cannot say.
pub struct KeyboardListener<Msg> {
    child: Vec<Box<dyn Widget<Msg>>>,
    /// Shared rather than borrowed: the interface the walk produces outlives the tree it
    /// was built from, so a closure it carries has to be owned.
    #[allow(clippy::type_complexity)]
    on_key: std::rc::Rc<dyn Fn(KeyStroke) -> Option<Msg>>,
}

impl<Msg> KeyboardListener<Msg> {
    /// Wraps `child`, calling `on_key` for each keystroke while focus is inside it.
    /// Returning `None` lets the keystroke carry on to the scopes around it.
    pub fn new(
        child: impl Widget<Msg> + 'static,
        on_key: impl Fn(KeyStroke) -> Option<Msg> + 'static,
    ) -> Self {
        Self {
            child: vec![Box::new(child)],
            on_key: std::rc::Rc::new(on_key),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for KeyboardListener<Msg> {
    fn style(&self) -> Style {
        sizing_of(self.child[0].style())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        sizing_of(self.child[0].style_themed(theme))
    }

    fn build_themed(&self, theme: &Theme) {
        self.child[0].build_themed(theme);
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn on_keystroke(&self) -> Option<std::rc::Rc<dyn Fn(KeyStroke) -> Option<Msg>>> {
        Some(std::rc::Rc::clone(&self.on_key))
    }
}

/// The four table-carrying wrappers are the same box with a different table filled in.
macro_rules! delegate {
    ($($ty:ident),* $(,)?) => {$(
        impl<Msg: Clone> Widget<Msg> for $ty<Msg> {
            fn style(&self) -> Style {
                Widget::<Msg>::style(&self.0)
            }
            fn style_themed(&self, theme: &Theme) -> Style {
                Widget::<Msg>::style_themed(&self.0, theme)
            }
            fn build_themed(&self, theme: &Theme) {
                Widget::<Msg>::build_themed(&self.0, theme)
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                Widget::<Msg>::children(&self.0)
            }
            fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
                Widget::<Msg>::paint(&self.0, bounds, status, theme, scene)
            }
            fn on_click(&self) -> Option<Msg> {
                Widget::<Msg>::on_click(&self.0)
            }
            fn focusable(&self) -> bool {
                Widget::<Msg>::focusable(&self.0)
            }
            fn shortcut_bindings(&self) -> &[(KeyStroke, Intent)] {
                Widget::<Msg>::shortcut_bindings(&self.0)
            }
            fn shortcut_callbacks(&self) -> &[(KeyStroke, Msg)] {
                Widget::<Msg>::shortcut_callbacks(&self.0)
            }
            fn action_bindings(&self) -> &[(Intent, Msg)] {
                Widget::<Msg>::action_bindings(&self.0)
            }
            fn action_listeners(&self) -> &[(Intent, Msg)] {
                Widget::<Msg>::action_listeners(&self.0)
            }
        }
    )*};
}

delegate!(
    Shortcuts,
    CallbackShortcuts,
    Actions,
    ActionListener,
    FocusableActionDetector,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        SavePage,
        SaveDialog,
        Edited(String),
    }

    #[test]
    fn a_letter_matches_without_case_and_a_modifier_does_not() {
        let bound = KeyStroke::new(ShortcutKey::Char('s')).ctrl();
        assert!(KeyStroke::new(ShortcutKey::Char('S'))
            .ctrl()
            .matches(&bound));
        assert!(KeyStroke::new(ShortcutKey::Char('s'))
            .ctrl()
            .matches(&bound));
        assert!(
            !KeyStroke::new(ShortcutKey::Char('s')).matches(&bound),
            "no Ctrl"
        );
        assert!(
            !KeyStroke::new(ShortcutKey::Char('s'))
                .ctrl()
                .shift()
                .matches(&bound),
            "Shift is part of the pattern"
        );
    }

    /// Shift alone is typing, not a command — otherwise a capital letter could not be
    /// typed into a field that sits under a shortcut scope.
    #[test]
    fn shift_alone_is_not_a_command() {
        assert!(!KeyStroke::new(ShortcutKey::Char('a')).shift().is_command());
        assert!(KeyStroke::new(ShortcutKey::Char('a')).ctrl().is_command());
        assert!(KeyStroke::new(ShortcutKey::Char('a')).alt().is_command());
        assert!(KeyStroke::new(ShortcutKey::Char('a')).meta().is_command());
    }

    /// The two-step resolution end to end: a key names an intent, and the innermost
    /// `Actions` that answers it supplies the message.
    #[test]
    fn a_key_names_an_intent_and_the_nearest_answer_wins() {
        use crate::runtime::Runtime;
        use crate::ui::build_ui;
        use frus_core::Size;

        let save = KeyStroke::new(ShortcutKey::Char('s')).ctrl();
        let field = |name: &str| {
            crate::TextInput::new(name.to_string())
                .label(name.to_string())
                .on_input(Msg::Edited)
        };

        // A page that saves, holding a dialog that saves differently.
        let tree = Actions::new(
            Shortcuts::new(crate::column![
                field("page"),
                Actions::new(field("dialog")).action(Intent("save"), Msg::SaveDialog),
            ])
            .bind(save, Intent("save")),
        )
        .action(Intent("save"), Msg::SavePage);

        let ui = build_ui(
            &tree,
            Size::new(400.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let stops: Vec<_> = ui.focusable_ids().collect();
        assert_eq!(stops.len(), 2);

        assert_eq!(ui.keystroke(save, Some(stops[0])), vec![Msg::SavePage]);
        assert_eq!(
            ui.keystroke(save, Some(stops[1])),
            vec![Msg::SaveDialog],
            "the innermost answer wins"
        );
        assert!(
            ui.keystroke(KeyStroke::new(ShortcutKey::Char('s')), Some(stops[0]))
                .is_empty(),
            "without Ctrl it is typing, not a command"
        );
    }

    /// An intent nobody answers is inert, not an error: a key bound to a meaning the
    /// current screen has no answer for should do nothing.
    #[test]
    fn an_unanswered_intent_does_nothing() {
        use crate::runtime::Runtime;
        use crate::ui::build_ui;
        use frus_core::Size;

        let save = KeyStroke::new(ShortcutKey::Char('s')).ctrl();
        let tree: Shortcuts<Msg> =
            Shortcuts::new(crate::Container::new().width(10.0)).bind(save, Intent("save"));
        let ui = build_ui(
            &tree,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(ui.keystroke(save, None).is_empty());
    }

    /// The sixth focus widget: a stop that answers its own keys while it has them.
    #[test]
    fn a_detector_is_a_focus_stop_that_answers_its_own_keys() {
        use crate::runtime::Runtime;
        use crate::ui::build_ui;
        use frus_core::Size;

        let del = KeyStroke::new(ShortcutKey::Delete);
        let card = FocusableActionDetector::new(crate::Container::new().width(40.0).height(40.0))
            .on_key(del, Msg::SaveDialog);
        assert!(Widget::<Msg>::focusable(&card));

        let ui = build_ui(
            &card,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let stop = ui.focusable_ids().next().expect("its own stop");
        assert_eq!(ui.keystroke(del, Some(stop)), vec![Msg::SaveDialog]);

        // Disabled, it is not a stop at all — rather than an inert one Tab still lands on.
        let off: FocusableActionDetector<Msg> =
            FocusableActionDetector::new(crate::Container::new()).enabled(false);
        assert!(!Widget::<Msg>::focusable(&off));
    }

    /// A later binding for the same stroke replaces the earlier one, so a caller can
    /// override a table it did not write rather than fighting it.
    #[test]
    fn rebinding_a_stroke_replaces_it() {
        let s: Shortcuts<()> = Shortcuts::new(crate::Container::new())
            .bind(
                KeyStroke::new(ShortcutKey::Char('s')).ctrl(),
                Intent("save"),
            )
            .bind(
                KeyStroke::new(ShortcutKey::Char('s')).ctrl(),
                Intent("save-as"),
            );
        let table = Widget::<()>::shortcut_bindings(&s);
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].1, Intent("save-as"));
    }
}
