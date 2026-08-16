# Milestone 321 — Scrolling belongs to the content

A report from the device, three milestones ago:

> Je vois encore les effets de fin de liste (scroll) sur la page home.

Milestone 316 answered the half of it that was a bug. The gesture arena was handing
drags to a scrollable that had nothing to scroll, so a page that fitted still lit an
end-of-list glow when it was pulled. Teaching the arena to decline stopped the glow.

It did not answer why the home page had a scrollable at all. That was the other half,
and it came with its own note:

> NB: chez Flutter, il y a des widgets spécifiques qui peuvent être scrollable.

## What the shell was deciding

`Scaffold::body` wrapped whatever it was given in a `Scroll`. Every screen in the
application, scrolling, whether or not it had anything to scroll — and with no way to
say otherwise, because the wrapper was not a parameter.

The reference does not do this, and says so in the body's own documentation: the body
is *"positioned at the top-left of the available space"*, and *"if you have a column of
widgets that should normally fit on the screen, but may overflow and would in such cases
need to scroll, consider using a `ListView` as the body of the scaffold."* Consider —
because it is the screen's decision, and the shell is not the one making it.

`NavScaffold`, in this same crate, already placed its body in a plain column. `Scaffold`
was the outlier even here.

## The cost of one wrapper

Deciding for every screen at once costs more than a stray glow:

- **a centred empty state cannot be centred.** A viewport gives its child the height it
  asks for, so "fill this screen and centre in it" has nothing to fill;
- **a screen with its own list gets a scroller inside a scroller.** Four demo screens
  already put an explicit `Scroll` in their body; they were nested, and nesting is how
  you get two offsets moving under one finger;
- **every screen registers a scrollable area** with the gesture arena, which is the
  phantom that fed the original report.

## The change

The body is placed in the room the bars leave it and nothing more. Everything else about
the slot is unchanged — it still starts below the app bar, still stops above the bottom
bar, still shortens for the keyboard. What moved is that the bottom clearance is now a
**sibling** of the body rather than padding inside a viewport, which is the same
geometry by a simpler route: the room shrinks, so a scrolling body scrolls within what
is left instead of running under the keyboard.

The body is loose and top-aligned, as the reference's is. A body that wants all of its
room says `flex(1.0)`, which is the reference's `SizedBox.expand` under another name.

## The migration, which is the point

Three screens use a `Scaffold`, and each answered differently — which is the argument
for the change stated better than any paragraph could:

- **Task** wants no scroller at all. Its content is centred in whatever room it is
  given, so it takes the room (`flex(1.0)`) and stops.
- **Sign-up wizard** wants one around the *body only*. Back and Next are in the
  persistent footer and must stay pinned while the steps move — a split that only exists
  because the two slots are different, and that the old shell got right by accident.
- **Home** wants one on two of its three sections. Tasks grows with the list and About
  is a long read; Stats is a master-detail pane sized to the size class, and wrapping it
  would give that screen a scrollable with nothing to scroll — the exact thing being
  removed.

## Verification

The test that matters asks the registry the gesture arena actually consults: a plain
body registers **no** scrollable area, a body taller than the screen registers none
either — overflowing is not the same as scrolling — and a body holding a `Scroll` gets
one. That is the device report turned into an assertion.

The scaffold's own geometry tests had to change their instrument. They used to pass an
over-tall block and read its **clip**: with the body in a viewport, what showed of an
oversized block *was* the viewport. Nothing clips now, so an over-tall block would paint
past the bottom of the screen and measure nothing. They pass a filling body instead,
which measures the room directly and is what the documentation now tells applications to
write.

## Left

- **No scroll-aware chrome.** The reference's app bar knows when content has passed
  beneath it (`scrolledUnderElevation`, carried over from milestone 318). With the
  scroller inside the body rather than around it, the bar is further from that
  information than before, not closer.
- **`extend_body` is now a weaker promise.** It still moves the bottom slots into an
  overlay so the body's room runs under them, but whether anything actually passes under
  a translucent bar depends on the screen putting a scroller there.
