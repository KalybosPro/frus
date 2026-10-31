//! [`Lazy`]: a builder that is also the widget it builds (milestone 716).
//!
//! `AppBar` and `Scaffold` are put together from their parts by `.build()`, because what
//! they assemble depends on all of their parts at once. Forgetting `.build()` used to be a
//! type error that named neither: `Scaffold` is not a widget. A `Lazy` keeps the parts
//! until the walk first asks the widget anything, assembles them then, and answers as
//! what it assembled — so the builder can be handed to anything that takes a widget, and
//! `.build()` stays for those who want the box now.

use std::cell::{OnceCell, RefCell};
use std::ops::Deref;

use crate::widget::Widget;

/// Parts kept until first asked for, then the widget they make.
pub(crate) struct Lazy<P, Msg> {
    parts: RefCell<Option<P>>,
    make: fn(P) -> Box<dyn Widget<Msg>>,
    built: OnceCell<Box<dyn Widget<Msg>>>,
}

impl<P, Msg> Lazy<P, Msg> {
    pub(crate) fn new(parts: P, make: fn(P) -> Box<dyn Widget<Msg>>) -> Self {
        Self {
            parts: RefCell::new(Some(parts)),
            make,
            built: OnceCell::new(),
        }
    }

    /// The parts, while they are still being set.
    pub(crate) fn parts_mut(&mut self) -> &mut P {
        self.parts
            .get_mut()
            .as_mut()
            .expect("a builder is not changed after it has been laid out")
    }

    /// The widget, made now if it was not yet.
    pub(crate) fn into_widget(self) -> Box<dyn Widget<Msg>> {
        match self.built.into_inner() {
            Some(built) => built,
            None => (self.make)(
                self.parts
                    .into_inner()
                    .expect("parts are kept until the widget is made"),
            ),
        }
    }
}

impl<P, Msg> Deref for Lazy<P, Msg> {
    type Target = Box<dyn Widget<Msg>>;

    fn deref(&self) -> &Self::Target {
        self.built.get_or_init(|| {
            let parts = self
                .parts
                .borrow_mut()
                .take()
                .expect("parts are kept until the widget is made");
            (self.make)(parts)
        })
    }
}
