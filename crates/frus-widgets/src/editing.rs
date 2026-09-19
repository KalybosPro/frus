//! [`TextEditingController`]: a text field's text, held outside the field.
//!
//! A field draws the text it is given; where that text lives is up to whoever built it.
//! A controller is that place, made once and handed to the field: what the person types
//! lands in it, and what the program puts in it appears in the field. Reading it is
//! reading what the field says, and writing it is writing to the field.
//!
//! ```
//! use frus_widgets::{TextEditingController, TextField};
//!
//! let name = TextEditingController::with_text("Ada");
//! let field: TextField = TextField::new("").label("Name").controller(&name);
//! # let _ = field;
//!
//! assert_eq!(name.text(), "Ada");
//! name.set_text("Ada Lovelace");
//! assert_eq!(name.text(), "Ada Lovelace");
//! name.clear();
//! assert!(name.is_empty());
//! ```
//!
//! It is made once and kept: in a [`State`](crate::State), or with
//! [`BuildContext::use_text_controller`].
//!
//! ```
//! use frus_widgets::{button, column, BuildContext, TextField, Widget};
//!
//! fn form(cx: &BuildContext) -> Box<dyn Widget> {
//!     let email = cx.use_text_controller("");
//!     let submit = email.clone();
//!     Box::new(column![
//!         TextField::new("").label("Email").controller(&email),
//!         button("Send", move || println!("sending to {}", submit.text())),
//!     ])
//! }
//! ```
//!
//! A controller is a [`Listenable`]: `add_listener` tells you whenever the text changes,
//! whether the person typed or the program did.

use crate::component::BuildContext;
use crate::notifier::{Listenable, Subscription, ValueNotifier};

/// A text field's text, and the changes to it.
///
/// Cloning gives another handle to the same text.
#[derive(Clone, Default)]
pub struct TextEditingController {
    text: ValueNotifier<String>,
}

impl TextEditingController {
    /// A controller holding no text.
    pub fn new() -> Self {
        Self::default()
    }

    /// A controller that starts out holding `text`.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: ValueNotifier::new(text.into()),
        }
    }

    /// The text.
    pub fn text(&self) -> String {
        self.text.get()
    }

    /// Reads the text without copying it.
    pub fn with_text_ref<R>(&self, read: impl FnOnce(&str) -> R) -> R {
        self.text.with(|text| read(text))
    }

    /// Replaces the text; a field driven by this controller shows it at once. Changing it to
    /// what it already is tells nobody.
    pub fn set_text(&self, text: impl Into<String>) {
        self.text.set(text.into());
    }

    /// Empties the text.
    pub fn clear(&self) {
        self.set_text("");
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.text.with(|text| text.is_empty())
    }

    /// How many characters the text has.
    pub fn len(&self) -> usize {
        self.text.with(|text| text.chars().count())
    }
}

impl Listenable for TextEditingController {
    fn add_listener(&self, listener: impl Fn() + 'static) -> Subscription {
        self.text.add_listener(listener)
    }
}

impl std::fmt::Debug for TextEditingController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextEditingController")
            .field(&self.text())
            .finish()
    }
}

impl BuildContext<'_> {
    /// A [`TextEditingController`] that lives as long as this component does, holding
    /// `initial` to begin with. The same controller comes back on every rebuild.
    pub fn use_text_controller(&self, initial: &str) -> TextEditingController {
        self.use_ref(|| TextEditingController::with_text(initial))
            .get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_deferred, Callback, Runtime, TextField, Theme, Widget};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn a_controller_holds_and_changes_its_text() {
        let c = TextEditingController::with_text("ab");
        assert_eq!(
            (c.text(), c.len(), c.is_empty()),
            ("ab".to_string(), 2, false)
        );
        c.set_text("héllo");
        assert_eq!(c.len(), 5, "characters, not bytes");
        c.clear();
        assert!(c.is_empty());
        assert!(c.with_text_ref(|t| t.is_empty()));
    }

    #[test]
    fn a_clone_is_the_same_text() {
        let a = TextEditingController::new();
        let b = a.clone();
        a.set_text("x");
        assert_eq!(b.text(), "x");
    }

    #[test]
    fn a_listener_hears_every_change_and_only_changes() {
        let c = TextEditingController::new();
        let heard = Rc::new(RefCell::new(Vec::new()));
        let log = heard.clone();
        let probe = c.clone();
        let _l = c.add_listener(move || log.borrow_mut().push(probe.text()));
        c.set_text("a");
        c.set_text("a");
        c.set_text("ab");
        assert_eq!(*heard.borrow(), ["a", "ab"]);
    }

    /// A field is driven by its controller: it shows the controller's text, and an edit
    /// writes to it — when the message is delivered, once.
    #[test]
    fn a_field_shows_its_controller_and_writes_back_to_it() {
        let c = TextEditingController::with_text("start");
        let field: TextField = TextField::new("ignored").controller(&c);
        assert_eq!(Widget::text_value(&field), Some("start"));

        let message: Callback = Widget::replace_value(&field, "typed".into()).expect("editable");
        assert_eq!(c.text(), "start", "asking for the message changes nothing");
        message.call();
        assert_eq!(c.text(), "typed");

        c.set_text("set from code");
        let field: TextField = TextField::new("").controller(&c);
        assert_eq!(
            Widget::text_value(&field),
            Some("set from code"),
            "and what the program writes is what the next build shows"
        );
    }

    #[test]
    fn a_hook_gives_the_same_controller_on_every_build() {
        let runtime = Runtime::default();
        let seen: Rc<RefCell<Vec<TextEditingController>>> = Rc::default();
        type Builder = Rc<dyn Fn(&BuildContext) -> Box<dyn Widget>>;
        let build: Builder = {
            let seen = seen.clone();
            Rc::new(move |cx| {
                let c = cx.use_text_controller("hello");
                seen.borrow_mut().push(c.clone());
                Box::new(TextField::new("").controller(&c))
            })
        };
        let make = || {
            let build = build.clone();
            crate::Component::stateless(move |cx: &BuildContext| build(cx))
        };
        for _ in 0..2 {
            runtime.states.begin_build();
            build_deferred(&make(), &Theme::default(), &runtime);
            runtime.states.end_frame();
            seen.borrow()[0].set_text("changed");
        }
        let seen = seen.borrow();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            seen[1].text(),
            "changed",
            "the same text, kept between builds"
        );
    }
}
