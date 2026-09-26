# Milestone 578 — An application's say over a selection area

Milestones 575 to 577 gave `SelectionArea` a selection, a bar and handles, all fixed by the
framework: the highlight in the theme's `selection`, the handles in `primary`, the bar holding Copy
and Select all. A field has let the application change all three since milestone 568
(`TextField::selection_toolbar`, `TextFieldTheme::handle_color`); text around a field could not.
Every widget's look and slots must be the application's to change, so this closes that gap.

## What it does

- `SelectionArea::around(child)` gives a **`SelectionArea`** — a widget of its own, where it gave a
  `Themed` before — with three builders:
  - `selection_toolbar(|context, items| …)` is given the default list (Copy, and Select all unless
    everything is selected) and answers with the list to show. It may drop items, reorder them or
    add the application's own (`ToolbarItem::custom`); an empty list shows no bar, and the selection
    is still made.
  - `selection_color(color)` sets the highlight under the selected words in this area.
  - `handle_color(color)` sets the handles under a touch selection in this area.
- A theme can set both colours for every selection in text that is not a field — a
  `Text::selectable()` one's as well as an area's — with **`TextSelectionTheme`**
  (`theme.widgets.text_selection`: `selection_color`, `handle_color`). Unset, they fall back to the
  theme's `selection` and the scheme's `primary`, so nothing that sets neither looks different.

## How

- The area lays its colours over the inherited theme in its `theme_override`, as the `Themed` it
  used to be laid `selectable`; the texts below read them when they paint. An area's colours win
  over the theme's, and a nested area's over the outer one's.
- The bar is asked of the area through a new hook, **`Widget::area_toolbar`**. The walk remembers
  the widget of the area it is in, beside its identity, and when it floats the bar against the
  first text the selection covers it takes the area's bar if the area has one. The answer has three
  cases: `None` when the area was told nothing, so the text's own Copy / Select all is used;
  `Some(None)` for "no bar"; `Some(Some(bar))` for the bar. A bar is built once per context and kept,
  as a field's is.
- Like `autofill_group`, the hook is one a wrapper may answer for itself, so the transparent-wrapper
  macro leaves it out and every wrapper states it; the check that each wrapper states those hooks now
  covers it. Without that, a `Keyed` area would lose its bar.

## Checked against the reference

The reference's selection theme carries the same two colours (`selectionColor`,
`selectionHandleColor`) beside a cursor colour a text outside a field has no use for. Its selection
area takes a context-menu builder that may be null, meaning no menu. `selection_toolbar` is this
framework's version of that builder, in the list form a field's bar already uses.

## Verification

- Through the shell: with a list keeping Copy and adding the application's Share, a hold shows Copy
  and Share and no Select all, and pressing Share sends the application's message; with an empty
  list, a hold selects the word and no bar floats. The 575–577 tests still pass.
- In a frame: an area with its own colours paints the highlight in its selection colour and the
  two handles in its handle colour. Without them, it paints the theme's `selection` and `primary`.
  The area answers for its bar through a `Keyed`, and says nothing when it was told nothing.
- Mutations, each failing a test:
  - the walk ignoring the area's bar;
  - "no bar" falling back to the text's;
  - the area never remembered;
  - either colour not laid over the theme;
  - `Keyed` not forwarding the hook;
  - the highlight painted in the theme's colour regardless.
- **Not run on the phone**: no device was attached when this was done. The colours and the bar go
  through the same paint and overlay paths milestones 576 and 577 checked on the phone. Only what
  is laid over them is new.

## Left

- The right-click bar on a desktop, autoscroll while a handle is dragged, and `RichText` inside an
  area are still not done.
