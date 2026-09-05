# Milestone 439 — Roles nothing asked for

Milestone 429 gave the scheme the families it was missing: `tertiary` and its three
companions, `error_container` and `on_error_container`, `inverse_primary`, `surface_tint`,
`surface_dim`, `surface_bright`. Ten milestones later, a grep for six of them outside
`theme.rs` returned nothing at all.

A role that exists and is never asked for is not neutral. It is a number in a table that
nobody has checked against anything, and it stays right by luck rather than by test. This
milestone spends two of them, on the two places the reference spends them.

## The day period is not an hour

The reference selects a number on the dial with `primary` / `onPrimary`
(`time_picker.dart:3762`) and the AM/PM cell with `tertiaryContainer` /
`onTertiaryContainer` (`:3664`, `:3700`). A different family, on purpose.

The reason is not decoration. **The two are not the same kind of choice.** Picking an hour
is picking the value; picking AM or PM is saying which half of the day the value is in.
Giving both the accent makes the smaller decision shout as loudly as the larger one, and a
picker where everything selected looks equally selected is a picker you have to read twice.

`TimeCell` serves both, so it gained one flag and a `palette` that answers from it. That is
also the first thing in this framework to ask the scheme for a tertiary role at all.

The reference's day period is a rounded rectangle with an outline (`:3656`, `:3674`) where
this is a pill in a grid; the shape stays as it is. This milestone is about the role.

## An errored field deepens under the pointer

`error` at rest; `on_error_container` while hovered; `error` again once focused. That is the
reference's rule for the border, the label and the floating label
(`input_decorator.dart:5981`, `:6004`, `:6053`), and the order matters — it tests **focus
before hover**, because a focused field is already saying everything it can and has nowhere
louder to go.

Two details worth keeping:

- **The message does not deepen.** `errorStyle` is `error` in every state (`:6100`). The
  text under the field is a sentence, not a control, and a sentence that changed colour
  under the pointer would be claiming to be one.
- **It is continuous here.** The reference has a discrete `hovered` state; this framework
  has `hover_progress`, a progression, and lerps between the two colours with it — damped by
  `1 - focus_progress` so that focusing takes the deepening back off. That is the habit the
  rest of the framework already has with the pointer.

`error_hover_color` joins the field's style and the theme's slot beside `error_color`, so
the caller and the theme can each replace it.

## What is still unasked for, and why that is fine

`tertiary` and `on_tertiary` remain unused — and so do they in the reference. Grepping its
whole material library for `_colors.tertiary` finds one hit, the day period's *container*.
The plain tertiary role exists for **applications**, not for components: it is the third
accent an application reaches for, and a framework that spent it on some widget would be
taking that choice away.

`surface_tint` is a different case, and the roadmap entry that sent me here was wrong about
it. Every M3 default in the reference now sets `surfaceTintColor` to `Colors.transparent` —
`AppBar` (`app_bar.dart:2545`), `Card` (`card.dart:319`), `BottomAppBar`
(`bottom_app_bar.dart:330`), every button, every icon button. The one non-transparent
default left is the **M2** bottom app bar (`:301`). Material 3 replaced the elevation tint
with the container ladder, which this framework implemented in milestones 426 and 427. So
"nothing tints as it lifts" is not a gap: it is the current specification, and the entry has
been corrected rather than closed.

## The tests

- `the_day_period_is_not_an_hour` — both families, that they differ, and that an unselected
  cell names neither.
- `an_errored_field_deepens_under_the_pointer` — the three states of the border, the message
  unchanged through all three, and that the two error roles differ at all.

Both were run against the code without the change and both fail.
