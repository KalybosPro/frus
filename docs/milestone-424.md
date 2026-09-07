# Milestone 424 — The name was taken by something that was not a dialog

frus had a drawer, a bottom sheet, a menu, a popover and a snack bar. It did not have the
one that asks a question and waits for the answer — and the name was already spoken for:

```rust
pub struct AlertDialog {
    title: Option<String>,
    text: String,
    kind: AlertKind,
    …
}
```

No actions. No barrier. **No message type** — it could not have carried a button's answer
if it had one. It sits in the flow, not over it. It is a tinted message box with an accent
bar and an icon, which is a fine widget and is not a dialog, and its own source said so:

> it wears the name but it has the shape of the reference's banner […] the name is recorded
> as the thing to settle rather than papered over

Settled. It is `Alert` now, unchanged in every other respect, and `AlertDialog` means what
it means in the reference.

## `Dialog`

The surface: rounded to 28, elevated by 6, on the scheme's `surfaceContainerHigh`, held off
the window's edges by 40 across and 24 down, and **never narrower than 280**. Every one of
those is the reference's number (`dialog.dart:32`, `:275`, `:1966`, `:1967`, `:1979`), and
every one is overridable on the instance and on the theme.

Two of the defaults are worth stating because they look like omissions:

- the **shadow is transparent**, and
- the **tint is unset**.

That is the reference's Material 3 answer (`dialog.dart:1982`): the container tone carries
the height, and a shadow under it is the old look. A caller or a theme that names either
gets it, which is what `Card` already does with the same tint machinery.

It is **controlled**, like every other overlay here — `open` is the application's field,
and `on_dismiss` is what a click on the scrim sends. Saying nothing about dismissal is the
reference's `barrierDismissible: false`, and it has to be the *answer* rather than a
default: a dialog that closes itself on a stray click has not been answered.

## `AlertDialog`

Icon, title, content, actions, and the reference's conditional paddings — which are
conditional in exactly the places the reference's are (`dialog.dart:795`, `:824`, `:857`,
`:1994`):

| slot | above | below |
|---|---|---|
| icon | 24 | 16 with a title, 0 with content, 24 alone |
| title | 0 under an icon, else 24 | 0 with content, else 20 |
| content | 16 under something, else 24 | 24 |
| actions | 0 | 24 |

And the one cross-slot rule: **an icon centres the title** (`dialog.dart:844`).

## What the centring cost, and what it taught

The reference centres the title with `textAlign`, and that works there because its column
is `CrossAxisAlignment.stretch` (`dialog.dart:928`) — the title is given the full width and
the alignment moves it inside that.

Three attempts here read the same, and none of them moved the title:

1. `Flex::column().align(Align::Center)` around the slots — the slot filled anyway.
2. `Container::new().padding_each(…)` for the padded slots — a `Container` **anchors its
   child at its natural size**, which its own source says in as many words. A text laid out
   at the width of its own words has nothing for an alignment to move it within.
3. The stretch on its own — a `Text` declares the width of its words, so stretching the
   column does not stretch the text.

What finally works is a `Flex::row().justify(…)` inside the padded slot, with the text
alignment set as well: the row places a single line, the alignment places the lines of a
title that wrapped. Both, because they answer different questions.

The test that caught this had to be corrected too, and that is the part worth keeping. It
asserted on `Primitive::Text.position.x`, which is the **box's** origin and not the glyph's
— so a correctly centred title reads at the same `x` as a left-aligned one, and the test
called the fix a failure. It reads the alignment and the box's centre line now.

## What is not in it

`scrollable`, the actions' overflow family (`actionsOverflowAlignment`,
`actionsOverflowDirection`, `actionsOverflowButtonSpacing`), `buttonPadding`,
`semanticLabel` / `namesRoute`, `alignment` (the overlay places it centred and nothing
else can be asked for yet), `Dialog.fullscreen`, and the reference's `paddingScaleFactor`,
which shrinks the paddings as the reader's font grows.

No golden moved (91 + 13 + 27): the rename does not change a pixel, and nothing in the
golden scenes opens a dialog.
