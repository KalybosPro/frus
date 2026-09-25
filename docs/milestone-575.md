# Milestone 575 — One selection across several texts: `SelectionArea`

The second half of #24, and the half the issue calls the one people reach for. Milestone 573 made a
single `Text` selectable; a paragraph broken into three `Text`s still could not be selected as a
paragraph, and a page of labels and values could not be copied from at all. This is the subtree in
which **every text is selectable and one drag selects across them**.

## What it does

- **`SelectionArea::around(child)`** wraps a subtree. Inside it, a mouse press on words that nothing
  else has a claim on begins a selection; dragging carries its far end to the text nearest the
  pointer, in the area and never out of it; a double click selects the word. What is selected is
  highlighted in each text it covers, in the theme's selection colour.
- **Ctrl+C** copies it — **one line per text**, in reading order — and **Ctrl+A**, with a selection
  held, selects every text of the area. **Escape**, or any press that does not begin a selection,
  puts it away; so does Tab.
- What answers a press keeps it: a button, a link, a field, a text that is itself
  [`selectable`](crate::Text::selectable). A press begins a selection only where nothing else would
  have taken it.
- The area is a wrapper that is its child. It draws nothing and does not touch the layout.

## Where the selection lives

The issue asked for this to be agreed first, and it is the whole design. A field's selection is
`Edit` state in the `Runtime`, keyed by the field's identity — right for one widget, wrong for a
range that begins in one and ends three later. So:

- **The area's selection is `Runtime::region`**: two ends — a text and a character in it — and, worked
  out from them, **what each text has selected** (`RegionSelection::ranges`). The ranges are stored,
  not derived while painting: a text painted early in the walk cannot know whether the ends are before
  or after it, because the texts after it have not been walked yet. The shell, which moves an end and
  has the whole frame in hand, works them out once (`region_ranges`, a pure function with its own
  tests) and the walk only reads them, into a new `Status::region`.
- **The texts an area holds are a registry of the frame**, `Ui::text_stops`, in painted order — reading
  order for anything a tree lays out in flow. It is the shape every other registry has (`inks`,
  `focusables`), so it is captured with a repaint boundary's cache, withheld by a modal barrier and
  transformed with a scaled layer, and the highlight is part of the cache's fingerprint, so a boundary
  whose text has just been selected is painted again and not replayed. Tests hold the replay and the
  repaint.
- **What makes a text take part is the theme**, as what makes it wear an inherited size is: the area
  hands down `DefaultTextStyle::selectable`, the walk sees it turn on at the area's node — which is how
  it knows the area's identity — and every text under it registers. No new structural hook had to be
  forwarded by the dozen wrappers that fuse with what they wrap.

## Decisions

**One line per text.** Copying a selection over three texts gives three lines. A paragraph the author
broke into three texts is three lines to the reader who selected it, and the alternative — guessing
which line breaks were soft — would be wrong as often as right.

**Reading order is painted order.** It is correct for text laid out in flow, which is nearly all of it,
and wrong for a row whose visual and tree orders differ. Sorting by geometry would be wrong in other
cases (a card that overlaps another); the tree is at least predictable.

**A finger is left to scroll.** A touch press-and-drag on words does not select: a finger dragging is
a scroll, and the selection by touch is a long press with handles and a bar. That is the third piece,
below, and is not here.

**The double click needs to be close.** The field's double click asks only for time; a selection area
also asks for the two presses to be within 8 px, because words are dense and a quick second press on
the next line is not a double click.

## Verification

- **Widgets:** `region_ranges` and `region_copy` (tail, middle, head; either drag direction; both ends in
  one text; clamping; a missing end); the registry (order, area identity, two areas, empty text, the
  text outside); the hit tests (a press finds its text, a stray drag the nearest of its own area);
  a **replayed repaint boundary** still registers its texts; a selection **repaints a cached
  boundary**.
- **Through the shell, with a mouse:** a drag across three texts copies the tail of the first, the whole
  of the middle one and the head of the last, and not the label of the button between them; upward is
  the same; past the words it ends at the last text; words outside the area, and a button, do not begin
  one; a click puts it away, in the area or out; a double click selects the word; select all; a finger
  does not select; the highlight is painted in each text covered.
- **Mutations**, each failing the test that names it: the registry not captured with the cache, the
  highlight left out of the fingerprint, the distance across lines weighted like along one, a
  tappable row's press not respected, a finger allowed to select, and a press not putting the
  selection away. (The first attempt at the tappable-row one survived: a button is a focus stop and
  was kept by that before the guard was asked, so the guard had no test until a row that is only
  clickable was added.)
- **The demo:** the About page is a selection area, and a test drags from its title to its wrapped
  paragraph through the shell and reads back two lines.

## Left

- **Touch.** A long press selecting a word in an area, with handles to extend it and the bar (Copy,
  Select all) — as milestones 511 and 568 did for a field — is the next piece. Until then a phone can
  select in a single `Text::selectable()` and not across an area.
- **A right-click bar** on the desktop, and the context-menu key: the same bar, the same piece.
- **Scrolling while dragging.** A drag that leaves the top or bottom of a scrolling area selects to the
  nearest text on screen; it does not scroll to reach more.
- **`RichText`** does not take part yet: only `Text` reports words to the area.
- **Reading order** is the tree's, as above; the semantics tree does not yet say a selection exists.
- The registry is withheld by a modal barrier and moved with a scaled layer as the others are; those
  two lines have no test of their own.
