# Milestone 493 — Reading a scroll position, and moving one

Closes [#25](https://github.com/KalybosPro/frus/issues/25).

`ScrollPosition`, `Widget::on_scroll`, `ScrollTo` and `Command::scroll`. A scroll offset
could be moved by a finger, a wheel, a fling and a spring, and by nothing an application
could write.

So none of this was expressible: a "back to top" button, a chat that opens at its newest
message, load-more when the end of a long list comes into view, putting somebody back
where they were when they return to a screen, or anything that reacts to how far a page
has gone down — a bar that fades in, a progress rule, a header that changes.

## The part with the decision in it

**The shape**, which is what the issue asked for before any code. The reference splits
this into a *controller* the application owns and asks, and a *notification* that travels
up the tree. Neither crosses. There are no controllers here: the offset belongs to the
runtime, a view is rebuilt from state every frame, and an application speaks in messages.

What crosses is the *split*, and each half took the form this framework already has for
things of its kind.

### Reading is a notification, and not an ambient value

The obvious alternative was an ambient `ScrollPosition` a view could read the way it reads
`MediaQuery`. It fails on the case that matters most: **a view is rebuilt when the state
changes, and a fling changes no state.** A bar fading in as the page goes down needs a
frame the application did not cause, so an ambient read would go stale for exactly the
half-second anybody is looking at it. A message is what turns a moving offset into a
rebuilt view, and messages are how everything else here reaches an application.

So: `Widget::on_scroll`, a hook that hands back a `ScrollPosition` — offset, extents and
viewport, the three numbers the layout already knows. Everything else is derived from
them: `fraction_y` for a rule, `remaining_y` for a load-more.

`fraction_y` is **0 when there is nothing to scroll**, which is the only answer that is
not a lie. A list shorter than its window is neither at its beginning nor at its end, and
a progress rule over it must read empty rather than full.

The three scroll regions an application writes — `SingleChildScrollView`, `ListView` and
`GridView` — each take an `on_scroll`. A paged view was left out: it already says where it
is, in the vocabulary it is read in, through `on_page_changed`.

### What it costs, which is the question the issue asked

*A message per frame during a fling is a lot of messages; say what you decided about that.*

Three answers, in order of how much they matter:

- **Silent by default.** `on_scroll` returns `None` unless a widget was given a callback,
  so a region nobody is listening to reports nothing and a fling over an ordinary list is
  exactly as cheap as it was. Nothing was added to the cost of scrolling.
- **A region somebody is listening to does send one per frame while it moves**, and that
  is the honest price of a view that answers the offset. It is what a fading bar needs. A
  framework that quietly halved it would be a framework whose bar stutters and whose
  author cannot find out why.
- **A grain, for the listeners that do not need every frame.** `notify_every(px)`
  suppresses the reports in between. A load-more asking "is the end within four hundred
  pixels" gains nothing from being told at every pixel.

And the rule that makes the grain safe: **it suppresses the steps in between, never the
last one.** A region that comes to rest anywhere other than where it was last reported
reports it, whatever the distance. A coarse grain costs frames, never accuracy.

One more decision inside the grain, and it is the one a test is written against: the
distance is measured from **what was last reported**, not from the previous frame. From
the previous frame, a slow drag would creep the whole way down a list three pixels at a
time, never travel a whole grain within one frame, and never be reported at all.

### A region appearing is not a movement — usually

A region seen for the first time resting at `(0, 0)` is recorded silently. Telling an
application on the first frame of every screen that its list is at the top would only
invite it to answer, and it already knew. This is the rule `page_changes` follows for the
same reason.

A region that appears **somewhere else** is reported. Restored, or opened at a box it was
asked to keep in view — the application did not put it there and has no other way of
finding out.

### Commanding is a request, and it does not have to survive a rebuild

A scroll is not a function of the state: the same list at the same offset, and yet "go
back to the top" happens once. So it is an effect — `Command::scroll` — and not a field on
a widget. The offset it changes *is* state, and it stays exactly where it was: in the
runtime, beside every other offset.

Which answers the issue's hardest question — how a request survives a rebuild — by
removing it. **Nothing is stored.** The request is spent the moment its region is found,
and what it leaves behind is an offset in the runtime, indistinguishable from one a finger
left there. There is nothing left to survive.

A request gets **one frame**, and two attempts inside it. The first is made where a paged
view's request and a keep-visible request are made, before the springs are stepped, so the
movement happens in the frame it arrives rather than the one after; the registry there was
built last frame. A region that has only just appeared is not in it — arriving on a screen
and being put back where you left off is exactly that case — so what does not resolve is
tried once more against the registry this frame builds, and then it is gone. A key naming
nothing costs one lookup and never accumulates.

### Identity is a key, because there was already an answer

Both halves need to name a region, and the only name a widget has is the `WidgetId` the
framework derives from its position in the tree — which an application cannot know and
should not have to.

There was no need to invent anything: `Command::focus` already names a field by wrapping
it in `keyed(k, …)` and addressing `k`. `Command::scroll` uses the same hash, resolved
through the same `find_by_key`. One test asserts the two hashes agree, which is the one
thing neither side can check alone.

## Two smaller decisions

**`End` is a name, not a number.** `ScrollTo::end()` does not carry an offset, because the
number is not knowable where the request is written: `update` has the state, and the
largest offset a region may rest at is a fact about a layout that has not happened yet.
Naming the end and letting the frame resolve it is the difference between "the bottom" and
a number that was the bottom one frame ago. `Start` and `Offset` are resolved the same
way, and an offset past the end is clamped rather than refused.

`ScrollTo` names one axis or both, and `None` leaves an axis alone — which is what lets
one type serve a vertical page, a sideways strip and a surface that scrolls both ways
without a caller ever writing a coordinate it did not mean.

**A finger refuses a request.** An offset has one owner at a time — the rule the drag
handling has followed since the rubber band was built — and while a hand is on the content
that owner is the hand. A list that jumped out from under a finger because a timer fired
would be the framework overruling the reader. A fling, by contrast, is caught: it is where
the list *was* going, and the request is the more recent statement of where it should be.

## In the demo

The log screen — five thousand virtualised rows — gained a row that answers the list:
which row is at the top, how far down it is, and a "Top" button that appears once the list
is four hundred pixels down and springs it back.

Its height is fixed, and that is not a detail. The button comes and goes with the offset;
a row that grew when it appeared would shorten the list, which changes how far the list
can scroll, which moves the offset. **A view driven by a measurement of itself has to be
built so that what it draws cannot change the measurement.**

The list reports every four pixels — under a tenth of a row, so the number in the header
is never a row out, and a slow drag says so ten times less often.

## Verification

Eighteen tests on the position, the request and the two runtime rules; four in the shell
on the key resolution, including the one that asserts a command's key and a view's key are
the same hash; one on the demo, driven through `view` so that the whole loop is under it —
the list reports, the application keeps, the next build reads — since any one of the three
could be right on its own while the loop did nothing.

The grain rule has the test that fails under the wrong version of it: three pixels a frame
against a ten-pixel grain, reported on the fourth frame because the distance is measured
from the last report and not from the last frame.
