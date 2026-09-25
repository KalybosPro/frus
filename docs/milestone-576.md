# Milestone 576 — A finger selects in a selection area

Milestone 575 gave `SelectionArea` a mouse and nothing else: a finger dragging over words scrolls
the page, as it should, and there was no other way in. A phone could select in a single
`Text::selectable()` and not in an area. This is the way in for a finger — the one a field has had
since milestones 511 and 568.

## What it does

- **A finger held still on words** in an area selects the word under it, and the bar opens over
  it: **Copy** and **Select all**. Nothing else claimed the hold — a button, a tappable row, a
  widget's own long press, an item that lifts, a field — or it goes to them, as before.
- **Select all** on the bar takes every text of the area, keeps the bar, and the bar then offers
  only Copy. **Copy** puts the words on the clipboard — one line per text — and puts the selection
  and its bar away, as a platform's own selection does.
- **Back**, **Escape** and a tap anywhere else put it away.
- A finger that moves is still a scroll, and a selection made with a mouse has no bar: the bar is
  for a pointer with no Ctrl+C.

## How

- `RegionSelection` carries whether its bar shows (`bar`) and whether it already holds everything
  (`all`, which takes Select all off the list).
- The walk floats the bar against the **first text the selection covers** — the first it meets with
  a range, since it meets them in reading order — through a new hook, `Widget::region_toolbar`,
  which a `Text` answers with its part of the selection's box and a bar it builds once per context.
  The bar is a pointer-sized cell on a `Text` until a bar is first asked of it. `Status::region_bar`
  is part of the paint cache's fingerprint, so a cached text that is now carrying the bar is walked
  again.
- The shell arms the long press on a finger's press over words the area may take, and on the hold
  selects the word, releases the page the finger was about to scroll, and marks the bar. A press on
  the bar's buttons reaches the area through the same `EditAction` path the field's bar uses.

## Verification

- Through the shell, with a finger: a hold selects `second` and shows Copy and Select all; Select
  all takes the three lines and leaves Copy alone on the bar; Copy puts the selection and the bar
  away; a hold on a button selects nothing; Escape, and a tap outside, put it away; a mouse
  selection has no bar. The 575 tests still pass.
- Mutations, each failing a test: the hold never armed, the bar never pushed, Copy leaving the
  selection, `all` never true, Select all dropping the bar.
- **On the phone** (Huawei STK-L21, Android 10, a release APK of the demo), on the About page: a
  hold on `painting` in the paragraph highlighted the word and floated Copy and Select all over it;
  the system **Back** put both away; a second hold selected `typography`; **Select all** highlighted
  the title and the paragraph and left Copy alone on the bar, above the title; **Copy** put them
  away, and in the task list's field the keyboard's clipboard suggestion read `About frus…` — the
  copied text had reached the system clipboard.
- The first hold on the phone, made straight after switching to the About page, selected nothing.
  It was not seen again in the two later holds (one held with `input motionevent`, one with
  `input swipe` as the first was), and the same hold through the test driver at the phone's size
  selects the word. Not explained.
- The clipboard's full contents after Copy were not read back: only the start of it, in the
  keyboard's suggestion. In tests, the test process has no system clipboard; the words Copy takes
  are the ones `region_copy` gives, which milestone 575 tests.

## Left

- **Handles.** A touch selection in an area cannot yet be widened by dragging its ends; Select all
  is the way to take more than a word.
- The application cannot change the area's bar (a field's can, through `selection_toolbar`).
- The bar sits against the first text the selection covers, not the selection's whole box.
- On the About page, the rich-text line and the timeline's entries are not selected by Select all:
  `RichText` does not take part yet (milestone 575), and the timeline paints its entries' words itself
  (`scene.text`) rather than holding `Text`s.
