# Milestone 413 — Twelve widgets that decided their own type

Milestone 412 ended on a note rather than a fix. Twelve widgets set their text in a
**private constant**, with no theme entry, no builder, and no way for an application to say
anything:

```rust
const SIZE: f32 = 16.0;

fn label_style() -> ResolvedTextStyle {
    TextStyle::new(SIZE).resolved()
}
```

That is the standing rule taken backwards — themed defaults yes, hardcoded-only never — and
it was why most of milestone 412 could not be tested: a style that cannot carry a `height`
cannot demonstrate that its `height` is honoured.

## It was worse than unthemed

The reference's Material 3 snackbar sets its content in `bodyMedium`, **14 px**
(`snack_bar.dart:974`). Our `SnackBar` said `const SIZE: f32 = 16.0`.

The constant had **drifted two pixels from the reference, and nobody could see it, because
it was private**. Its action was 14 px regular where the reference's is `labelLarge` — 14 px
*medium* (`snack_bar.dart:717`).

Once every constant was read against the reference rather than against itself, eleven of the
eighteen were wrong:

| widget | was | the reference | now |
| --- | --- | --- | --- |
| `SnackBar` content | 16 | snackbar content `bodyMedium` | 14 |
| `SnackBar` action | 14 | snackbar action `labelLarge` | 14 medium |
| `PopupMenuButton` | 16 | popup menu `titleMedium` | 16 medium |
| `DropdownButton` | 18 | dropdown `titleMedium` | 16 medium |
| `Table` heading | 15 | data table heading `titleSmall` | 14 medium |
| `Table` cell | 15 | data table data `bodyMedium` | 14 |
| `DatePicker` day | 15 | M3 `dayStyle` → `bodyLarge` | 16 |
| `DatePicker` weekday | 13 | M3 `weekdayStyle` → `bodyLarge` | 16 |
| `NavigationRail` label | 12 | M3 rail `labelMedium` | 12 medium |
| `NavigationRail` badge | 10 | the step `Badge` already reads | `labelSmall`, 11 medium |
| `Alert` title | 16 | (see below) | `titleMedium`, 16 medium |
| `Alert` message | 15 | banner + dialog `bodyMedium` | 14 |
| `Steps` label | 12 | stepper title `bodyLarge` | 16 |
| `Steps` index | 14 | `_kStepStyle`, 12 px | 12 |
| `Tree` row | 16 | list tile title `bodyLarge` | 16 — unchanged |
| `Timeline` title | 16 | list tile title `bodyLarge` | 16 — unchanged |
| `Timeline` detail | 13 | list tile subtitle `bodyMedium` | 14 |
| `Autocomplete` option | 16 | list tile title `bodyLarge` | 16 — unchanged |
| `Kbd` | 13 | — | `labelMedium` + monospaced |

Three of the eighteen were already right. **Nothing here was decided by looking at the
current value**, which is the whole point: a constant that has drifted looks perfectly
reasonable from the inside.

## The chain, and the one fallback that is not a second constant

Every one of them now resolves the same way as `Button` and `Chip` already did:

> what the caller said → what `theme.widgets.<widget>` says → the step of `theme.text`.

Twelve new structs in `widgettheme.rs`, one builder per style on each widget, and no widget
naming a size any more.

The awkward term is the **third**. `Widget::style` is the un-themed path — the one the
transparent wrappers take when they ask a child how big it is — and it has no theme to read
a step from. Writing the framework's own number beside the scale would have reintroduced,
inside this very milestone, the two-constants-that-must-agree shape that the last four
milestones were about.

So the scale became a `const`:

```rust
impl TextTheme {
    pub const M3: Self = Self { /* the fifteen steps */ };
}

pub(crate) fn type_scale(theme: Option<&Theme>) -> TextTheme {
    theme.map_or(TextTheme::M3, |t| t.text)
}
```

`Default for TextTheme` returns `Self::M3`. There is one scale, and the themed and un-themed
paths read the same one.

## Two ways to carry a caller's override, and why both exist

Most widgets thread the style into the private row widget they build — `Item`, `Row`,
`Suggestion`, `NavItem`. Where a widget already had a `rebuild()`, the new builder calls it,
so `.action(…)` then `.action_text_style(…)` and the reverse describe the same widget: a
caller is entitled to assume that and should not have to discover it.

`Table` and `DatePicker` do it differently, through `Widget::theme_override`:

```rust
fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
    if self.heading_text_style.is_none() && self.data_text_style.is_none() {
        return None;
    }
    let mut theme = *inherited;
    /* … */
    Some(Box::new(theme))
}
```

A table builds cells in half a dozen places — the header row, the data rows, the virtualised
rows, the frozen columns — and a calendar has five constructors. A value carried down the
theme reaches every one of them without any of them being taught to pass it on. `None` when
nothing was said, so a widget that says nothing costs nothing.

## What a builder cannot do

`SnackBar::action` **measured its label in the builder**, before any theme existed, and kept
the width in a field:

```rust
pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
    let width = (frus_text::measure_resolved(&label, &action_style()).width + …).ceil();
    self.action_w = width + ACTION_GAP;
```

That is the same mistake in its purest form: a number about type, decided where the type is
not yet known. The label and the message are kept beside the child now, and the width is
taken in `style_themed` where there is a theme to take it with.

One measurement is still taken in a builder, and it is named in the code: `Autocomplete`'s
`max_visible` viewport, which is a `SingleChildScrollView`'s fixed height. It follows the
reader's font setting (`resolved()` applies it) and the caller's own override, but not a
theme's — an application that retypesets suggestions *and* caps them should say it on the
widget, where the same number reaches the rows and their viewport.

## Four bugs found on the way

Uncovering these constants uncovered what stood behind them.

**`Table` had a survivor of milestone 412.** The sweep looked for `line_height(style.size)`
and this one was written `frus_text::line_height(SIZE)` — against the bare constant, not
against a style. Exactly the formulation a compiler cannot find. Beside it, the sort arrow
was placed from `frus_text::measure(&self.label, SIZE)`, an unresolved size: at any text
scale above 1 the arrow landed **inside** the word.

**`Text::measure_key` never learned about milestones 409 and 410.** It hashes the resolved
size, weight and slant — and neither `height` nor `family`, both of which those milestones
put into the measurement. Two paragraphs alike but for their leading, wrapped to the same
box, and the second kept the first's geometry. `Alert::measure_key` had the same hole from
the other end: it hashed the text and the title and nothing about the styles, which was
harmless while the styles were constants and is not any more.

**`Timeline` placed its detail line at a fixed `+30.0`** and drew its dot at `ROW_H * 0.5`
rather than at the row's actual half. The second line follows the first's own line box now,
and the row's height is the floor *or what the two lines need*.

**`DatePicker` declared its width from `CELL`** while its cells sized themselves from
`cell()` — the floor grown by the reader's setting. Seven cells wide against a box sized for
seven smaller ones: the last column was clipped at any scale that grew the cells. The
container asks the cells now.

**`Autocomplete` counted its viewport in `ROW_H`** while its rows were `line_box(ROW_H, …)`.
Same shape, same fix.

None of the four had a test, and none of them could have had one, because none of the styles
involved could be set from outside. That is the argument for this milestone, not a footnote
to it.

## `TextStyle` could not say two of its own fields

Milestone 409 added `TextStyle::height` and milestone 410 added `TextStyle::family`, both as
public fields with no builder beside `size`, `weight`, `italic`, `color` and the decorations.
A caller could name a family only by writing the struct out. `Kbd` wanted one, which is how
it was noticed. Both are `const fn` builders now, like the rest.

## `Alert` is named for a widget it is not

`AlertDialog` has an accent bar, an icon and a tinted background, no actions and no barrier:
it is the reference's **banner**, wearing the name of its **dialog**. The two disagree about
the title — a dialog's is `headlineSmall`, 24 px, and a banner has no title at all — and
they agree about the message, `bodyMedium`, which is what it now uses.

A 24 px heading inside a 12 px box would have been the reference followed off a cliff. The
title takes the heading role at this scale and **the name is recorded as the thing to
settle**, rather than papered over.

## What this does not fix

`Steps` puts its label *under* the marker where the reference puts its title *beside* it.
The role is read from the reference; the placement is ours. Told apart in the code, because
the next person to compare them deserves to know which half was a decision.

Seven more widgets keep a private text constant — `Breadcrumb`, `TimePicker`, `Kanban`,
`Slider`'s tooltip, `Form`'s bullet, `TextField`'s helper line, and the `AppBar`/`NavBar`
titles. They are a **different problem**: the reference has no counterpart for them, so their
role has to be argued rather than read, and an argued role does not belong in the same commit
as eleven read ones.
