# Milestone 577 — Handles to widen a touch selection across texts

Milestone 576 let a finger select a word in a `SelectionArea` and gave it Copy and Select all, and
nothing in between: more than a word meant all of it. A field's touch selection has had two handles
since milestone 511. This gives an area's the same two, and lets them cross from one text to
another.

## What it does

- A selection made with a finger carries **two handles**, hanging below its two ends — the start
  one in the text where it begins, the end one in the text where it ends, which may be different
  texts.
- **A handle held and dragged carries its end** to the text position under it, in whichever text of
  the area that is; the other end stays put. The handle's line, not the fingertip, picks the
  position: the offset between the finger and the handle is kept from the press, as a field's is.
- **The two never meet nor cross.** A move that would put the end before the start, or the start
  after the end, is refused and the selection stays as the last move left it.
- The bar is put away while a handle moves and comes back, over the new selection, when it is let
  go. A press on a handle does not put the selection away, as any other press does.
- A selection made with a mouse has no handles.

## How

- `RegionSelection` knows its two ends **in reading order** (`start`, `end`), worked out with the
  ranges, and whether it carries handles (`handles`, set with the bar by `with_bar`).
- The walk hands each text the handles that hang from it, `Status::region_grips`, which is part of
  the paint cache's fingerprint; a `Text` paints them after its words. The geometry is one function
  shared with a field's handles (`handle_at`), so an area's hang exactly as a field's do.
- A new hook, `Widget::selection_grip`, answers where the handle hanging from a character boundary
  is. The shell asks it of the two ends' texts to find a handle under a press, with the same 48-px
  touch target and the same nearer-of-two rule as a field (`selection::grab`).

## Verification

- Through the shell, with a finger: a hold gives two handles, the start left of the end, both below
  the line, and the frame paints two more shapes; a mouse selection has none; the end handle carried
  down to the next text selects the rest of the word's line and the head of the next, the bar gone
  while it moved and back after; the start handle carried up widens the selection upward; a move
  straight past the other end is refused. The 575 and 576 tests still pass.
- Mutations, each failing a test: the handles not painted, never taken, allowed to cross, the bar
  not brought back, the finger's offset to the handle dropped, and handles given to a mouse
  selection. (The last survived at first: a mouse selection never went through the call that sets
  the handles, except when the keyboard selects all, which the test then did.)
- **On the phone** (Huawei STK-L21, Android 10, release APK of the demo), About page: a hold on
  `painting` showed the word highlighted with its two handles below it and the bar above; the start
  handle, carried up with the finger to the title, took the selection's start into the title — `frus`
  highlighted there, `Layout, painting` in the paragraph, the end handle unmoved — and the bar came
  back over the title when the finger lifted. Copy, then the keyboard's clipboard suggestion in the
  task list's field read `frus…`: the widened selection had reached the clipboard (only its start is
  shown there, so the rest was not read back).

## Left

- The handles are drawn in the colour of the theme's `primary`; the application cannot restyle an
  area's handles as it can a field's (`handle_color`).
- A handle dragged past the top or bottom of a scrolling area does not scroll it.
- `RichText` still takes no part.
