# Milestone 403 — The reader's font size, obeyed

The largest accessibility gap this framework had. A phone's *Font size* slider goes to 1.3
on Android and past 3 with iOS's larger accessibility sizes; an interface that ignores it
is one a lot of people cannot read. Milestone 399 gave `MediaQuery` a `text_scaler` and
said plainly that it was carried and not spent, because spending it needed two things to
agree that had no single place to agree in.

Milestone 402 built that place.

## Why it could not be done before, in one sentence

The hazard was never the multiplication. It was that **69 call sites read a size** and the
renderer read another one, so a scale applied in 68 of them draws text the layout never
measured — every row on the screen the wrong height at once, with nothing in the picture to
say which of the two numbers was the mistake.

After 402 there is exactly one place a size becomes a number: `TextStyle::resolved()`. So
the scale goes there, and the property holds by construction rather than by vigilance.

```rust
size: self.size.unwrap_or(DEFAULT_TEXT_SIZE) * text_scale(),
```

## Ambient, not threaded — and that is the decision

Passing a scaler down would mean every widget that measures a label remembering to apply
it. There is no diagnostic for the one that forgets: not a panic, not a warning, just a
screen that is subtly wrong. Ambient makes forgetting impossible, because the only place
that needs it already reads it.

It lives in `frus-core`, below the crate that owns `MediaQuery`, since that is where
`resolved()` is. `MediaQuery::scope` installs it — the reader's font size travels with the
description because it *is* part of the description, and installing it anywhere else would
be one more thing to remember.

`text_scale()` is public and documented as **read once**. A second reader is a second
chance to scale a size twice, or to scale one a sibling did not.

`ResolvedTextStyle::to_style` was deleted in the same breath. It was unused, and with a
scale in `resolved()` it becomes a trap: resolved size → style → resolved again is a size
scaled twice. An unused method that would be wrong the moment somebody used it is worse
than no method.

## What is deliberately not scaled

- **A scale of zero or less is disbelieved**, not obeyed. A font of no size is a screen
  with no words on it; a platform reporting one is a platform to distrust rather than a
  user to accommodate.
- **Only the size.** A reader asking for larger text has not asked for a different
  typeface, so weight and slant are untouched.
- **The debug overlays** (`inspector.rs`, the layout overlay in `ui.rs`) pass literal sizes
  and go on doing so. They are for the developer, not for the reader.
- **Outside a described surface, nothing scales.** Every test in the crate and every golden
  builds widgets with no `MediaQuery` around them; a framework that scaled by default would
  move all of them at once, which is why the 91 goldens are unchanged.

## What the probe found

Rather than assume, a throwaway test built a `Button`, a `Chip` and a `ListTile` at 1.0,
1.3 and 2.0 and printed each text's box against what its glyphs actually need:

| | 1.0 | 1.3 | 2.0 |
|---|---|---|---|
| Button "Save" | 83×40, needs 35×17 | 93×40, needs 45×22 | 117×40, needs 69×34 |
| Chip "Tag" | 56×32, needs 24×17 | 63×32, needs 31×22 | 80×32, needs 48×34 |

**Widths follow correctly** — the components grow sideways as the type does. **Heights do
not**: `BUTTON_HEIGHT` is 40 and `CHIP_HEIGHT` is 32 whatever the reader asked for, so at
2.0 a chip's glyphs need 34 px in a 32 px box. That is a real limit and it is recorded
below rather than described as fine.

The probe also turned up something that has nothing to do with scaling, and it is the
worse finding — see *Found on the way*.

## Found on the way: a `ListTile` in a column collapses

At scale 1.0, with no scaling involved at all:

```
bare:                    "A row"  box 46x20
in a column:             "A row…" box  0x20
in a stretched column:   "A row…" box  0x20
```

A list tile **on its own** lays out correctly. Put it in a column — the most ordinary thing
anybody does with a list tile — and its title box collapses to zero width and the text
ellipsises to nothing. This is the milestone 392 family again, and it is next.

It is not folded into this milestone: the causes are unrelated, and a commit that fixes two
unrelated things is a commit that cannot be reverted for one of them.

## Left

- **Fixed component heights do not follow the type.** `BUTTON_HEIGHT`, `CHIP_HEIGHT`,
  `APP_BAR_HEIGHT`, `LIST_TILE_HEIGHTS`. The reference's Material components mostly grow;
  ours are constants. Per-component work, and the measurement is now trustworthy enough to
  drive it.
- **The scale is linear.** The reference's `TextScaler` is a *function*, so a platform can
  scale non-linearly — big sizes growing less than small ones, so a headline does not leave
  the screen when the body is made readable. `MediaQuery::scaled` was already written as a
  method for this reason; the ambient holds an `f32` for now.
- **No platform reports it yet.** `Application::accessibility` and `MediaQuery::text_scaler`
  both default to *nothing asked for*; Android exposes it through `Configuration.fontScale`.
  That is the same missing wire as milestone 399's.
