# Milestone 449 — No widget could name itself in the reader's language

A framework says a certain amount out loud on an application's behalf. The label a screen
reader announces on a back arrow. The word on the cross that dismisses a notification. The
initials over a calendar's columns, and the name of the month above them.

None of it comes from the application, so none of it could be translated by the
application. Every one was an English string constant — `const CLOSE_LABEL: &str =
"Close"`, `.label("Back")`, and **three separate copies** of the twelve month names.

## It was not only untranslated

The calendar always started its week on **Sunday**.

```rust
let lead = first_weekday(year, month);   // 0 = Sunday
```

That is the right number of blank cells only in a country whose week starts on Sunday.
Across most of Europe the week starts on Monday, in much of the Middle East on Saturday —
and in every one of them, every date in the month sat one or two columns out of place.
Not a missing translation: a wrong calendar.

The reference threads `firstDayOfWeekIndex` through both halves of the problem
(`calendar_date_picker.dart:1100` for the headings, `date.dart:356` for the offset), and
is careful about one thing that makes it work: `narrowWeekdays` is **always listed Sunday
first** (`date.dart:353`), whatever the reader's week does. That is what lets one index
serve as an index into the list *and* as the rotation.

```rust
fn lead_cells(year: i32, month: u32, first_day_of_week: usize) -> usize {
    (first_weekday(year, month) as isize - first_day_of_week as isize).rem_euclid(7) as usize
}
```

`rem_euclid`, not `%`: the difference goes negative for every locale that does not start
on Sunday, and Rust's `%` keeps the sign where Dart's does not.

## `Localizations`

A trait whose every method has an **English body**, so a table writes down only what
differs — and `English` is that trait with nothing overridden. The reference's table has
around a hundred entries; this has the ones the framework actually says today, and grows
as it says more.

It reaches widgets through this framework's ambient-scope idiom, the one `MediaQuery`
already uses: `localizations::install` for the thread, `localizations::scope` for a
closure, `localizations::of` to read.

**Not on the theme.** A theme is what an interface looks like; what it says is a different
question with a different owner. An application shipping one theme in twelve languages
would otherwise need twelve themes.

## The trap this walked into on purpose

`of()` never fails: with nothing installed it answers English, which is what every
constant it replaces already said. So nothing breaks.

That is also exactly the shape of milestone 408's bug — an ambient value that every test
passed and that never arrived in the running application, because nothing installed it.
The default makes the feature safe to add *and* would hide the wiring being missing.

So the guard is not a widget test. `a_shell_installs_the_application_s_words` drives
`install_ambient` — the shell's own reading of `Application::localizations`, extracted
into one function precisely so a test can call it, since the frame loop needs a window and
an event-loop proxy that no test in this repo can hold.

It also pins the other half: an application that answers `None` leaves whatever is in
force alone. **Saying nothing is not the same as saying English.**

## The tests

- `the_words_are_english_until_someone_says_otherwise`
- `a_table_says_only_what_differs`
- `a_scope_puts_back_what_it_found` — including through a panic.
- `a_week_starts_where_the_reader_s_week_starts` — the offsets, including the negative
  case, and the cell count in a real grid.
- `the_column_headings_rotate_and_the_month_is_named`
- `a_shell_installs_the_application_s_words`

The last three fail with the constants restored.

## Still open

The reference's table has about a hundred entries and a `LocalizationsDelegate` that
loads them per locale; this has seven and no loading at all. The next ones the framework
will need as it says more: a drawer's tooltip, a dialog's, the row-count phrases a table
speaks, and any of it that needs **plurals** — which is where a table of strings stops
being enough and the format itself has to be part of the entry.
