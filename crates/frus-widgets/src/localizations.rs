//! **The words the framework itself puts on screen**, in the reader's language
//! (`material_localizations.dart`).
//!
//! A framework says a certain amount out loud on an application's behalf. The label a
//! screen reader announces on a back arrow. The word on the cross that dismisses a
//! notification. The initials over a calendar's columns, and the name of the month above
//! them. None of it comes from the application, so none of it can be translated by the
//! application — and until this module every one of them was an English string constant.
//!
//! The reference's answer is `MaterialLocalizations`, reached from the tree. This is the
//! same idea with this framework's ambient-scope idiom (the one [`MediaQuery`] uses): a
//! table is installed for the thread, and [`of`](crate::localizations::of) hands out whatever is in force.
//!
//! # It always answers
//!
//! [`of`](crate::localizations::of) never fails. With nothing installed it answers [`English`], which is what every
//! one of those constants said anyway — so nothing breaks, and an application that says
//! nothing is exactly where it was.
//!
//! That has a cost worth naming: a test that forgets to install a table still passes, and
//! so would a shell that forgot to install one. The guard against that is not the default
//! but `a_shell_installs_the_application_s_words`, which drives the shell and checks the
//! words actually arrive.
//!
//! # It is not on the theme
//!
//! A theme is what an interface **looks like**. What it says is a different question with
//! a different owner — the reference keeps them apart too, and an application that ships
//! one theme in twelve languages would otherwise need twelve themes.
//!
//! [`MediaQuery`]: crate::MediaQuery

use std::cell::RefCell;
use std::rc::Rc;

/// **What the framework says**, for one language.
///
/// Every method has an English body, so implementing this means writing down only what
/// differs. The reference's table has around a hundred entries; this has the ones the
/// framework actually says today, and grows as it says more.
pub trait Localizations {
    /// What a reader hears on a back arrow (`backButtonTooltip`).
    fn back_button_label(&self) -> &str {
        "Back"
    }

    /// And on the cross that dismisses something (`closeButtonTooltip`).
    fn close_button_label(&self) -> &str {
        "Close"
    }

    /// What a reader hears on the control that opens a side panel
    /// (`openAppDrawerTooltip`).
    ///
    /// **One word for both edges.** The reference says the same thing for a leading
    /// panel and a trailing one (`action_buttons.dart:331` against `:362`): a reader told
    /// which edge a panel comes in from is being told about the layout rather than about
    /// the action.
    fn open_drawer_label(&self) -> &str {
        "Open navigation menu"
    }

    /// **Where one destination sits among the rest** (`tabLabel`), for a reader who is
    /// hearing them one at a time and cannot see how many there are.
    ///
    /// `index` counts from one, as it reads: "Tab 1 of 3". It is the one entry here that
    /// takes arguments and so returns an owned string rather than a borrowed one — the
    /// numbers are the caller's, and a table cannot have written the sentence in advance.
    fn tab_label(&self, index: usize, count: usize) -> String {
        format!("Tab {index} of {count}")
    }

    /// **What a reader hears on an account header** (`signedInLabel`): the name, the
    /// address and the control for switching arrive as one thing with one name, rather
    /// than as three unrelated nodes at the top of a panel.
    fn signed_in_label(&self) -> &str {
        "Signed in"
    }

    /// And on the control that reveals the other accounts (`showAccountsLabel`).
    fn show_accounts_label(&self) -> &str {
        "Show accounts"
    }

    /// And on the same control once they are showing (`hideAccountsLabel`).
    ///
    /// **Two entries rather than one that flips**, as the reference has it: a control
    /// named for what it *will do* is the only kind a reader can act on, and the two
    /// sentences are not each other's negation in every language.
    fn hide_accounts_label(&self) -> &str {
        "Hide accounts"
    }

    /// **What a reader hears on the cross that empties a field** (`clearButtonTooltip`).
    ///
    /// Not [`close_button_label`](Self::close_button_label): the same glyph means two
    /// different things depending on what it sits in, and *Close* on a control that empties
    /// a search box would send a reader looking for the thing it closed.
    fn clear_button_label(&self) -> &str {
        "Clear"
    }

    /// The word on a confirming button (`okButtonLabel`).
    fn ok_button_label(&self) -> &str {
        "OK"
    }

    /// And on the one that backs out (`cancelButtonLabel`).
    fn cancel_button_label(&self) -> &str {
        "Cancel"
    }

    /// The single letters over a calendar's columns (`narrowWeekdays`).
    ///
    /// **Always Sunday first**, whatever day the week starts on where the reader is —
    /// the reference is explicit about this (`date.dart:353`), and it is what lets
    /// [`first_day_of_week_index`](Self::first_day_of_week_index) be an index into this
    /// list rather than a second thing to keep in step with it.
    fn narrow_weekdays(&self) -> [&str; 7] {
        ["S", "M", "T", "W", "T", "F", "S"]
    }

    /// **Which day a week starts on**, as an index into
    /// [`narrow_weekdays`](Self::narrow_weekdays) — so `0` is Sunday and `1` is Monday
    /// (`material_localizations.dart`).
    ///
    /// Sunday in the United States, Monday across most of Europe, Saturday in much of the
    /// Middle East. A calendar that always started on Sunday was not merely untranslated:
    /// it put the days in the wrong columns.
    fn first_day_of_week_index(&self) -> usize {
        0
    }

    /// The months, January first.
    fn months(&self) -> [&str; 12] {
        [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]
    }
}

/// **The English table**: every method left as the trait wrote it.
///
/// It is what [`of`] answers when nothing has been installed, and the sensible thing for
/// another table to be written against — implement [`Localizations`] and override the
/// entries that differ.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct English;

impl Localizations for English {}

thread_local! {
    /// The table in force on this thread. `RefCell` rather than `Cell`, the value being
    /// a trait object behind an `Rc` and not `Copy`.
    static AMBIENT: RefCell<Option<Rc<dyn Localizations>>> = const { RefCell::new(None) };
}

/// **The table in force**, or [`English`] when nothing has been installed.
///
/// Cheap enough to call from a paint: it is a reference count, not a table.
pub fn of() -> Rc<dyn Localizations> {
    AMBIENT.with(|ambient| {
        ambient
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(English) as Rc<dyn Localizations>)
            .clone()
    })
}

/// Installs `table` for this thread, from now on.
///
/// The shell does this every frame from [`Application::localizations`], so an application
/// that changes language while it is running is obeyed on the next frame.
///
/// [`Application::localizations`]: https://docs.rs/frus-shell
pub fn install(table: Rc<dyn Localizations>) {
    AMBIENT.with(|ambient| *ambient.borrow_mut() = Some(table));
}

/// Runs `f` with `table` in force, and puts back whatever was there before — including
/// when `f` panics, so one bad frame cannot leave a stale language installed for every
/// frame after it.
pub fn scope<R>(table: Rc<dyn Localizations>, f: impl FnOnce() -> R) -> R {
    let previous = AMBIENT.with(|ambient| ambient.borrow_mut().replace(table));
    let _restore = Restore(previous);
    f()
}

/// Puts back the previous table when dropped, panic or not.
struct Restore(Option<Rc<dyn Localizations>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let previous = self.0.take();
        AMBIENT.with(|ambient| *ambient.borrow_mut() = previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table that starts its weeks on Monday and says so in French.
    struct Fr;

    impl Localizations for Fr {
        fn back_button_label(&self) -> &str {
            "Retour"
        }

        fn first_day_of_week_index(&self) -> usize {
            1
        }

        fn narrow_weekdays(&self) -> [&str; 7] {
            ["D", "L", "M", "M", "J", "V", "S"]
        }
    }

    /// **Nothing installed still answers**, in English — which is what every string
    /// constant this replaces already said.
    #[test]
    fn the_words_are_english_until_someone_says_otherwise() {
        assert_eq!(of().back_button_label(), "Back");
        assert_eq!(of().close_button_label(), "Close");
        assert_eq!(of().first_day_of_week_index(), 0);
        assert_eq!(of().months()[0], "January");
    }

    /// A table overrides only what differs, and the rest stays as the trait wrote it.
    #[test]
    fn a_table_says_only_what_differs() {
        scope(Rc::new(Fr), || {
            assert_eq!(of().back_button_label(), "Retour");
            assert_eq!(of().first_day_of_week_index(), 1);
            assert_eq!(
                of().close_button_label(),
                "Close",
                "not said, so the trait's own answer stands"
            );
        });
    }

    /// And a scope puts back what it found, **including when what it runs panics** — one
    /// bad frame must not leave a language installed for every frame after it.
    #[test]
    fn a_scope_puts_back_what_it_found() {
        assert_eq!(of().back_button_label(), "Back");
        scope(Rc::new(Fr), || {
            assert_eq!(of().back_button_label(), "Retour");
        });
        assert_eq!(of().back_button_label(), "Back", "restored");

        let fell_over = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scope(Rc::new(Fr), || panic!("one bad frame"));
        }));
        assert!(fell_over.is_err());
        assert_eq!(
            of().back_button_label(),
            "Back",
            "restored through the unwind as well"
        );
    }
}
