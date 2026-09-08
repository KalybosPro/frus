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
///
/// **A default is not agreement.** An entry left unanswered is an English string in a
/// French interface, and it compiles. So a table shipped here writes out every entry, even
/// the ones whose right answer in that language *is* the English one — "OK" is "OK" in a
/// good many languages — with the reason beside it; [`French`] does, and a test reads this
/// file to check that it still does.
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

    /// **The placeholder in a table's search field** (`searchFieldLabel`).
    fn search_field_label(&self) -> &str {
        "Search"
    }

    /// **What a table says when it has nothing to show.** A caller may say its own; this
    /// is what one that says nothing gets, and it used to be a constant.
    fn no_results_label(&self) -> &str {
        "No results"
    }

    /// **What a reader hears on the control that steps a number down**, and on the one
    /// that steps it up.
    ///
    /// The glyphs are a minus and a plus, which say nothing out loud.
    fn decrease_label(&self) -> &str {
        "Less"
    }

    /// See [`decrease_label`](Self::decrease_label).
    fn increase_label(&self) -> &str {
        "More"
    }

    /// **What a table announces when a row is ticked**, and the three sentences beside it
    /// (`selectedRowCountTitle`).
    ///
    /// Four whole sentences rather than one built from parts, for the reason
    /// [`hide_accounts_label`](Self::hide_accounts_label) is its own entry: a language
    /// that agrees its participle with the thing selected cannot be served by a formula
    /// that glues *row* to *selected*.
    fn row_selected_label(&self) -> &str {
        "Row selected"
    }

    /// See [`row_selected_label`](Self::row_selected_label).
    fn row_deselected_label(&self) -> &str {
        "Row deselected"
    }

    /// See [`row_selected_label`](Self::row_selected_label).
    fn all_rows_selected_label(&self) -> &str {
        "All rows selected"
    }

    /// See [`row_selected_label`](Self::row_selected_label).
    fn all_rows_deselected_label(&self) -> &str {
        "All rows deselected"
    }

    /// **The heading over a time picker's hours**, and the one over its minutes
    /// (`timePickerHourLabel`, `timePickerMinuteLabel`).
    fn hour_label(&self) -> &str {
        "Hour"
    }

    /// See [`hour_label`](Self::hour_label).
    fn minute_label(&self) -> &str {
        "Minute"
    }

    /// **The two halves of a twelve-hour clock** (`anteMeridiemAbbreviation`,
    /// `postMeridiemAbbreviation`).
    ///
    /// Left as the Latin abbreviations in a good many languages that never use a
    /// twelve-hour clock at all — which is not an oversight in a table that says so, and
    /// is why a test forbidding a French entry from matching the English one has to have
    /// exceptions.
    fn ante_meridiem_label(&self) -> &str {
        "AM"
    }

    /// See [`ante_meridiem_label`](Self::ante_meridiem_label).
    fn post_meridiem_label(&self) -> &str {
        "PM"
    }

    /// **The two ends of a range**: the heading over the first picker of a pair, and over
    /// the second.
    fn start_label(&self) -> &str {
        "Start"
    }

    /// See [`start_label`](Self::start_label).
    fn end_label(&self) -> &str {
        "End"
    }

    /// **What a reader hears on the arrows either side of a calendar's month.**
    fn previous_month_label(&self) -> &str {
        "Previous month"
    }

    /// See [`previous_month_label`](Self::previous_month_label).
    fn next_month_label(&self) -> &str {
        "Next month"
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

/// **The French table.**
///
/// Written rather than generated, and by someone who speaks it: a machine-translated
/// table is worse than none at all, because it silences the question for that language
/// and leaves a native reader with something subtly wrong and nobody looking at it.
///
/// Three things in here are not translation:
///
/// - **The week starts on Monday.** A calendar that always began on Sunday was not
///   untranslated; it put the days in the wrong columns.
/// - **The months are lower case**, which is the rule in French and looks like a mistake
///   to an English eye. "janvier 2026" is correct and "Janvier 2026" is not.
/// - **`AM` and `PM` stay as they are.** French tells the time on a twenty-four hour
///   clock and has no everyday words for the halves of a twelve-hour one; the reference's
///   own French table keeps the Latin abbreviations here too.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct French;

impl Localizations for French {
    fn back_button_label(&self) -> &str {
        "Retour"
    }

    fn close_button_label(&self) -> &str {
        "Fermer"
    }

    fn open_drawer_label(&self) -> &str {
        "Ouvrir le menu de navigation"
    }

    fn tab_label(&self, index: usize, count: usize) -> String {
        format!("Onglet {index} sur {count}")
    }

    fn signed_in_label(&self) -> &str {
        "Connecté"
    }

    fn show_accounts_label(&self) -> &str {
        "Afficher les comptes"
    }

    fn hide_accounts_label(&self) -> &str {
        "Masquer les comptes"
    }

    fn clear_button_label(&self) -> &str {
        "Effacer"
    }

    /// **"OK" is "OK" in French**, and a table that translated it to look busy would be
    /// wrong about the language.
    fn ok_button_label(&self) -> &str {
        "OK"
    }

    fn cancel_button_label(&self) -> &str {
        "Annuler"
    }

    fn search_field_label(&self) -> &str {
        "Rechercher"
    }

    fn no_results_label(&self) -> &str {
        "Aucun résultat"
    }

    fn decrease_label(&self) -> &str {
        "Moins"
    }

    fn increase_label(&self) -> &str {
        "Plus"
    }

    /// **Agreed with the thing selected**, which is why these are four sentences and not
    /// one formula: *ligne* is feminine, so the participle takes an `e`, and it takes an
    /// `s` as well once there are several of them.
    fn row_selected_label(&self) -> &str {
        "Ligne sélectionnée"
    }

    fn row_deselected_label(&self) -> &str {
        "Ligne désélectionnée"
    }

    fn all_rows_selected_label(&self) -> &str {
        "Toutes les lignes sélectionnées"
    }

    fn all_rows_deselected_label(&self) -> &str {
        "Toutes les lignes désélectionnées"
    }

    fn hour_label(&self) -> &str {
        "Heure"
    }

    /// The same word as the English one, which is the right answer and not a gap.
    fn minute_label(&self) -> &str {
        "Minute"
    }

    /// **Kept as they are.** French tells the time on a twenty-four hour clock; these are
    /// written out rather than left to the trait so that nothing in this table is silence.
    fn ante_meridiem_label(&self) -> &str {
        "AM"
    }

    /// See [`French::ante_meridiem_label`].
    fn post_meridiem_label(&self) -> &str {
        "PM"
    }

    fn start_label(&self) -> &str {
        "Début"
    }

    fn end_label(&self) -> &str {
        "Fin"
    }

    fn previous_month_label(&self) -> &str {
        "Mois précédent"
    }

    fn next_month_label(&self) -> &str {
        "Mois suivant"
    }

    /// Sunday first, as the trait requires, whatever day the week starts on here:
    /// dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi.
    fn narrow_weekdays(&self) -> [&str; 7] {
        ["D", "L", "M", "M", "J", "V", "S"]
    }

    /// **Monday.** Not a translation, and the thing most often left behind by one.
    fn first_day_of_week_index(&self) -> usize {
        1
    }

    fn months(&self) -> [&str; 12] {
        [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ]
    }
}

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

    /// Reads this file back, with its line endings normalised — a checkout on Windows
    /// hands it over with CRLF while the patterns below are written with `\n`.
    fn source() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/localizations.rs");
        std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("reading {path}: {e}"))
            .replace("\r\n", "\n")
    }

    /// The names of the methods declared between `start` and the end of its block.
    fn methods_in(source: &str, start: &str) -> Vec<String> {
        let body = &source[source.find(start).expect("the block") + start.len()..];
        let end = body.find("\n}\n").expect("the end of the block");
        body[..end]
            .lines()
            .filter_map(|line| line.trim().strip_prefix("fn "))
            .map(|rest| rest[..rest.find(['(', '<']).unwrap_or(rest.len())].to_string())
            .collect()
    }

    /// **A French table answers every entry the trait declares**, checked against the
    /// trait itself rather than against a list kept up to date by hand.
    ///
    /// This is the bug the whole module is about, one level down: an entry left at its
    /// default is an **English string in a French interface**, and it compiles, and no
    /// test that only checks the entries someone remembered would ever say so. An entry
    /// whose right answer in French *is* the English one — "OK", "Minute", "AM" — is
    /// written out with a reason beside it, so silence never means agreement.
    #[test]
    fn the_french_table_answers_every_entry_the_trait_declares() {
        let source = source();
        let declared = methods_in(&source, "pub trait Localizations {");
        let answered = methods_in(&source, "impl Localizations for French {");
        assert!(declared.len() >= 25, "found only {declared:?}");
        for entry in &declared {
            assert!(
                answered.contains(entry),
                "`{entry}` is not in the French table, so a French reader gets the English words: {answered:?}"
            );
        }
    }

    /// **And every one of them is a French sentence**, save the few whose right answer in
    /// French is the English one. A machine-translated table would pass the test above and
    /// fail this one on nothing at all; this is what catches an entry answered by copying.
    #[test]
    fn the_french_words_are_not_the_english_ones() {
        // "OK" is "OK" in a good many languages, "Minute" is spelt the same in both, and
        // French has no everyday words for the halves of a clock it does not use.
        let same_on_purpose = ["OK", "Minute", "AM", "PM"];
        let (en, fr) = (English, French);
        let entries: Vec<(&str, &str, &str)> = vec![
            ("back", en.back_button_label(), fr.back_button_label()),
            ("close", en.close_button_label(), fr.close_button_label()),
            ("drawer", en.open_drawer_label(), fr.open_drawer_label()),
            ("signed in", en.signed_in_label(), fr.signed_in_label()),
            (
                "show accounts",
                en.show_accounts_label(),
                fr.show_accounts_label(),
            ),
            (
                "hide accounts",
                en.hide_accounts_label(),
                fr.hide_accounts_label(),
            ),
            ("clear", en.clear_button_label(), fr.clear_button_label()),
            ("ok", en.ok_button_label(), fr.ok_button_label()),
            ("cancel", en.cancel_button_label(), fr.cancel_button_label()),
            ("search", en.search_field_label(), fr.search_field_label()),
            ("no results", en.no_results_label(), fr.no_results_label()),
            ("decrease", en.decrease_label(), fr.decrease_label()),
            ("increase", en.increase_label(), fr.increase_label()),
            (
                "row selected",
                en.row_selected_label(),
                fr.row_selected_label(),
            ),
            (
                "row deselected",
                en.row_deselected_label(),
                fr.row_deselected_label(),
            ),
            (
                "all selected",
                en.all_rows_selected_label(),
                fr.all_rows_selected_label(),
            ),
            (
                "all deselected",
                en.all_rows_deselected_label(),
                fr.all_rows_deselected_label(),
            ),
            ("hour", en.hour_label(), fr.hour_label()),
            ("minute", en.minute_label(), fr.minute_label()),
            ("am", en.ante_meridiem_label(), fr.ante_meridiem_label()),
            ("pm", en.post_meridiem_label(), fr.post_meridiem_label()),
            ("start", en.start_label(), fr.start_label()),
            ("end", en.end_label(), fr.end_label()),
            (
                "previous month",
                en.previous_month_label(),
                fr.previous_month_label(),
            ),
            ("next month", en.next_month_label(), fr.next_month_label()),
        ];
        for (what, english, french) in entries {
            if same_on_purpose.contains(&french) {
                continue;
            }
            assert_ne!(english, french, "{what} was not translated");
        }
        assert_ne!(en.tab_label(1, 3), fr.tab_label(1, 3));
        assert_ne!(en.months(), fr.months());
        assert_ne!(en.narrow_weekdays(), fr.narrow_weekdays());
    }

    /// **The parts that are not words.** A week that starts on Monday and a month written
    /// in lower case are the two things a translation of the strings alone would leave
    /// behind, and the first of them puts the days in the wrong columns.
    #[test]
    fn the_french_table_gets_the_parts_that_are_not_words_right() {
        assert_eq!(French.first_day_of_week_index(), 1, "lundi");
        assert_eq!(
            French.narrow_weekdays()[French.first_day_of_week_index()],
            "L",
            "the index is into a list that always starts on Sunday"
        );
        for month in French.months() {
            assert_eq!(
                month.chars().next().unwrap().to_lowercase().to_string(),
                month.chars().next().unwrap().to_string(),
                "{month} is capitalised, which is an English habit"
            );
        }
    }

    /// **The words reach the widgets that say them.**
    ///
    /// A table nobody reads is a table that translates nothing, and until this milestone
    /// five widgets said their own English out loud where no table could reach it. This
    /// walks the ones that were rewired and looks for French on the far side — through the
    /// scene and the accessibility tree, not through the table it already tested.
    ///
    /// Each is built **inside** the scope, which is the contract: a widget composes its
    /// children when it is constructed, so it speaks the language in force at that moment.
    /// In an application that is every frame, the shell installing the table before the
    /// view is built.
    #[test]
    fn the_words_reach_the_widgets_that_say_them() {
        use crate::theme::Theme;
        use crate::ui::build_ui;
        use frus_core::Size;

        scope(Rc::new(French), || {
            // A calendar: the month over it, and the arrows either side of it.
            let picker = crate::DatePicker::new(2026, 1, Some(15), |_: u32| (), |_: i32| ());
            let ui = build_ui(
                &picker,
                Size::new(320.0, 360.0),
                &crate::Runtime::default(),
                &Theme::default(),
            );
            let words: Vec<String> = ui
                .scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            assert!(
                words.iter().any(|w| w == "janvier 2026"),
                "the month, in lower case: {words:?}"
            );
            let labels: Vec<String> = ui
                .semantics()
                .iter()
                .filter_map(|(_, _, s)| s.label.clone())
                .collect();
            assert!(
                labels.iter().any(|l| l == "Mois précédent"),
                "the arrows: {labels:?}"
            );

            // A stepper: two buttons whose glyphs are a minus and a plus, and which say
            // nothing at all out loud without these.
            let stepper = crate::Stepper::new(3, |_: i32| ());
            let buttons: Vec<String> = crate::widget::Widget::<()>::children(&stepper)
                .iter()
                .filter_map(|child| child.semantics().and_then(|s| s.label))
                .collect();
            assert_eq!(buttons, vec!["Moins".to_string(), "Plus".to_string()]);

            // A table announcing what a tick will do.
            let table = crate::Table::<()>::new(2)
                .header(&["Nom", "Note"])
                .checkboxes(|_| (), ())
                .row(&["Ada", "5"]);
            let row = crate::widget::Widget::<()>::children(&table)[1].children()[0].announce();
            assert_eq!(row.as_deref(), Some("Ligne sélectionnée"));
        });
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
