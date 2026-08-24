# Milestone 400 — A text style a subtree hands down

`AppBar::toolbar_text_style` was recorded as blocked in milestone 396, and deliberately not
half-shipped: the property is meaningless without a way to reach a run of words the bar
never sees. This builds that way, and the property falls out at the end of it.

## The problem the reference solves with a nullable field

The reference's `DefaultTextStyle` is an inherited widget, and its `Text` merges against it:

```dart
TextStyle? effectiveTextStyle = style;
if (style == null || style!.inherit) {
  effectiveTextStyle = defaultTextStyle.style.merge(style);
}
```

`merge` copies over only the fields the caller's style actually answered. It can do that
because every field of a `TextStyle` there is **nullable**: "unset" is a value the type can
hold.

Ours cannot. `TextStyle.size` is an `f32` and `weight` is an enum, and `Text::new` has to
put *something* in them. So a `Text` carries a size whether or not anybody chose it, and a
number cannot say which of the two it is.

That distinction is the entire feature. Without it:

- treat every field as chosen, and an inherited style wins nothing — the feature does
  nothing at all;
- treat every field as unchosen, and a subtree silently resizes the one text that had
  asked to be different.

So `Text` now carries a small `Chosen` record beside its style — one flag per question the
caller can answer — and the rule is:

> **what the caller said ?? what the subtree handed down ?? what the framework ships**,
> and a default the caller never picked does not count as having said something.

`Text::new("x")` is 16 px because nobody picked a size, so a subtree asking for 20 gets 20.
`Text::new("x").size(16.0)` picked one, and keeps it against the same subtree.

Colour and decoration colour have no flag: they are `Option`s already and say it
themselves. `decoration` does have one, because `TextDecoration::NONE` is a real answer —
it is how a caller takes an underline back off a run of words a subtree underlined.

`Text::styled(content, style)` counts size, weight and slant as answered — a whole
`TextStyle` names all three outright — and leaves colour and decoration able to inherit, so
a text built from a step of the type scale still picks up a colour the subtree set.

## Where it had to be resolved, and why that cost three signatures

The obvious implementation resolves the inherited style at paint. It is also wrong, and
wrong in the way that is hardest to see in a screenshot: the text draws at 24 px inside a
box that was measured for 16. Every row on the screen is the wrong height at once, and
nothing in the picture says which of the two numbers was the mistake.

So the resolution has to reach the **layout**, and the layout asks four questions the
`Widget` trait was answering without a theme:

- `measure` — the closure taffy calls for a widget sized by the space it is offered;
- `measure_key` — the fingerprint that keeps the relayout cache honest;
- `main_axis_floor` — the width below which a text will not be squeezed along a row;
- `main_axis_fill` — whether the widget asks for its parent's width at all.

The last one is the easiest to miss and the one that would have hurt most. Alignment is
also a *request*: a box exactly as wide as its own text has nowhere to centre it in, so a
text told to centre asks to fill the line first. Blind to the theme, the hook would leave a
handed-down `align` resolving correctly everywhere except where it takes effect — centring
a subtree's texts would silently do nothing.

All four now take `&Theme`, exactly as `style_themed` does and for the same reason. Every
call site already had one in scope — the layout walk, the relayout fingerprint — so nothing
had to be threaded anywhere new; the change is a signature, not a plumbing job. Six files
implement these hooks, and the `transparent!` macro forwards them, so a wrapper keeps
answering for the widget inside it.

`measure_key` hashes the **resolved** style rather than the written one. A cache key that
ignored half of its inputs would hand back the geometry the text had before the subtree
said anything — a stale layout, waiting for a theme to change.

Inside `Text`, one `Resolved` is built per hook and everything reads from it. That is not
tidiness: two copies of this reasoning are two places for the themed and the unthemed
answers to drift apart.

## An inherited limit also grants the squeeze

The easy half to leave out. A line limit or an overflow mode says two things — what to do
with the words that do not fit, and that this text *may be given less than it asked for*. A
flex item's automatic minimum size is its content, so a text that has not granted the
squeeze refuses to give way and pushes its siblings out instead (milestone 333, and
milestone 392 one level up).

A subtree that hands down `max_lines` and `ellipsis` therefore grants it too. Without that,
the feature ships a bar whose texts are set to ellipsise and never get the chance to.

## The property this was for

`AppBar::toolbar_text_style` — the reference's `toolbarTextStyle` — dresses **the words in
the bar that are not the title**: a label beside the back arrow, a "Save" in the actions,
anything a caller handed over already assembled. It travels the same road as
`icon_theme` from milestone 396, as a theme for the subtree, because that is the only way
to reach a `Text` nested three levels inside a button.

It is merged onto whatever an enclosing subtree already handed down rather than replacing
it: two nested subtrees each setting one field must leave a text wearing both. That has its
own test, because a whole-style handover would look identical with a single wrapper and be
wrong the moment there are two.

An application reaches the same mechanism through `DefaultTextStyle::around(child)`, which
is the reference's widget of that name — the bar is one caller of it, not the only one.

**Not the title.** The reference keeps `toolbarTextStyle` and `titleTextStyle` apart, and a
bar that let the first reach the title would quietly resize the one line in the bar that
already had an answer — and the one whose width decides how many actions still fit inline.
That half has its own assertion in the test.

## Found on the way: nine theme structs that shipped unusable

`AppBar::icon_theme` has taken an `IconTheme` since milestone 396, and `IconTheme` was
`pub` inside a **private** module and re-exported nowhere. Public and unreachable: nobody
outside the crate could name the type, so nobody could call the method. The property
shipped and could not be used.

Eight more were in the same state — `ButtonTheme`, `CheckboxTheme`, `ChipTheme`,
`IconButtonTheme`, `RadioTheme`, `SegmentedTheme`, `SliderTheme`, `SwitchTheme`. Those are
less severe, because a caller writing `theme.widgets.button.background = Some(c)` never
names the type, but the same wall is there the moment they want to build one.

All ten are exported now, `DefaultTextStyle` included. A property that ships unusable is a
property that did not ship.

## Left

- **`RichText` does not inherit.** Neither does the reference's — an explicit style on the
  root span is what it asks for, and `Text.rich` is the wrapper that merges. Ours has no
  equivalent of that wrapper yet.
- **`TextField`, `Button` and the rest of the widgets that build a `Text` internally** put a
  style on it from their own theme entry, so a handed-down style does not reach their
  labels. Correct for a button, arguable for a list tile; the reference resolves each of
  them against its own component theme first, which is what ours does — but it has not been
  checked widget by widget.
- The bar's remaining properties are unchanged from milestone 396's list: `clipBehavior`,
  `automaticallyImplyLeading`, `automaticallyImplyActions`, `animateColor`,
  `scrolledUnderElevation` (nothing carries a scroll signal to the bar), `primary` (needs
  builder-based slots), `systemOverlayStyle` (a message to the system).
