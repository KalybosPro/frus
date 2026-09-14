# Milestone 511 — A word held, and two handles to move it by

Towards [#23](https://github.com/KalybosPro/frus/issues/23): copy and paste without a
keyboard. The issue separates two pieces — the **gesture** that makes a selection with a
finger, and the **toolbar** that acts on it — and asks for the toolbar's shape to be agreed
before much of it is written. This is the gesture.

## What was missing

A selection could be made with a mouse (press and drag, or a double click on a word) and
with a keyboard (Shift and the arrows). With a finger, a press in a field placed the caret
and captured the gesture as a mouse drag would — so the long-press recogniser, which only
arms for a press nothing has captured, never armed there. A finger could drag out a
selection character by character, and nothing else: no way to take a word, and nothing on
screen to adjust a selection by once it existed.

## The gesture

- **A hold in a field selects the word under the finger.** A finger press in a text field
  now arms the shell's existing long-press recogniser (500 ms, the same slop) with a claim
  of its own. When it fires, the word around the caret the press already placed is
  selected — `word_at`, the rule a double click uses — and the release is swallowed, as for
  any long press. The reference does the same on Android. A hold in a field is that and
  nothing else: it is not also the long press of a card the field sits in.
- **The selection gets two handles.** Each is a 22 px disc with the corner nearest the
  text squared off, hanging below its end of the selection and pointing in at the text: the
  start one to the left of its position, the end one to the right. They are the scheme's
  `primary` unless the field (`TextFieldStyle::handle_color`) or the theme
  (`TextFieldTheme::handle_color`) says otherwise.
- **A handle is taken first.** It hangs below its line, over whatever is drawn there, so
  the shell asks for it before anything else a press could mean. A finger is bigger than a
  handle, so each one is given the platform's minimum touch target, 48 px; when a short
  selection puts both targets under the finger, the nearer handle is taken.
- **Dragging a handle moves its own end.** The finger's offset from the text position the
  handle stands for is kept, so the selection follows the handle rather than jumping to the
  fingertip the moment it moves, and the position is found on the line's centre, not below
  it where the handle hangs. The other end stays put, and **the two never meet nor cross** —
  the reference's rule on Android, which keeps a selection dragged small a selection rather
  than turning it, under the finger, into a caret or into its mirror. The end being dragged
  is the caret, which is what a field wider than its box keeps in view.
- **The handles go away** with the next press anywhere that is not a handle, with any key,
  and whenever the field is not the focused one. A selection made with a mouse or the
  keyboard never has them.
- **The keyboard is told.** On Android, the selection a hold makes and the one a handle
  leaves are reported to the keyboard (`updateSelection`, milestone 510), which then knows
  what a replacement would replace.

## One geometry, asked of the field

The paint and the shell's hit test must agree to the pixel on where a handle is, or a
finger lands on a handle and takes nothing. So there is **one function** — the field's
`handles`, answering through a new hook, `Widget::selection_handles` — and both read it: the
paint to draw, the shell to hit-test and to drag. It is the paint's own geometry: the
content's insets (a prefix icon, a label band above the box), the alignment, the sideways
scroll that follows the caret, the retained vertical scroll of a multi-line field.

A wrapper that fuses with its field shares its identity, so it is the wrapper the shell
asks. The hook is forwarded by the transparent wrapper's macro, by `Responsive` — written
by hand, and three silent bugs to date for exactly this reason (milestones 477–495) — and by
`Box<dyn Widget>`, and a test holds all three to it.

## What this does not do yet

- **The toolbar** — Cut, Copy, Paste, Select all over the selection, and the right-click
  that opens it on a desktop. The next piece of #23, and the one whose item list the issue
  asks to agree first. Until it lands, a selection made with a finger can be replaced by
  typing over it and nothing more.
- **A handle under a plain caret.** The reference shows a single handle under the caret
  after a tap, to move it by. A tap here places the caret and shows nothing.
- **Dragging on after the hold.** The reference extends the selection word by word if the
  finger moves once the hold has fired; here the hold selects and the rest of that gesture
  does nothing until the finger lifts.
- **A handle dragged past the field's edge** does not scroll the field along with it, and
  there is no magnifier.
- **A handle overhanging the field is painted in the field's own pass**, so something
  painted after the field — the widget below it — draws over the part that overhangs. The
  reference puts its handles in an overlay above everything; this framework has no such
  layer for a widget to paint into, and adding one is its own piece of work.

## Verification

- Seven tests of the rule between the geometry and the finger, in the shell
  (`selection.rs`, pure): a finger takes the handle it lands on, beside it inside its touch
  target, and neither past it nor between the two; the nearer of two in reach; dragging
  moves its own end only, whichever way the selection was made; the dragged end is the
  caret; the ends never meet nor cross; a composition does not survive a handle; no
  selection, nothing to drag.
- Three of the field: each handle stands for its own end — the point it carries, handed
  back to the field's own hit test, is that end, plain, with a label and a prefix icon,
  outlined with its label on the border, and scrolled sideways — and each sits on the line
  the paint draws, the text's top, the line's middle, then the handle. That last check is
  not decoration: a single line's hit test clamps the height to its one line, so the round
  trip alone passed with the label band left out of the geometry, and only the paint could
  say the handles had risen by it; a multi-line field scrolled down carries its handles up with the text;
  handles are painted for a touch selection only, in the scheme's primary or the field's
  own colour.
- Two of the wiring: the shell's mark on the runtime reaches the field as handles, and not
  without the mark or without the focus; the wrappers that fuse with a field pass the
  question on.
- Nine mutations, each failing the test meant for it: the mark never reaching the status;
  handles painted for every selection; the sideways scroll, the vertical scroll and the
  label band each left out of the geometry; `Responsive` swallowing the hook; the ends
  allowed to cross; the start handle always taken when both are in reach; the touch target
  shrunk to the handle. Two of them passed the first time round, and both were holes in
  the tests rather than in the code: the label band, for the reason above, and the nearer
  of two, whose test put the finger inside only one target — so it never asked the
  question it was named for.
- The long-press claim and the handle drag live in the shell's event loop, which no test
  reaches, so the device confirms them. Huawei STK-L21, Android 10, SwiftKey: `hello
  world` typed into the demo's field; a hold on the first word selected `Hello`, two
  handles under it, and the keyboard's suggestions turned to that word; the end handle
  dragged right to the end of the text selected `Hello world`, the handle following to
  the new end and the keyboard's suggestions following it.
