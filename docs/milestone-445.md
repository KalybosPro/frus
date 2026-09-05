# Milestone 445 — A bar that never went away

Milestone 444 stopped frus drawing a scrollbar where the reference draws none: not on a
touch screen, not along a horizontal axis, and where it does draw one, a thumb rather than
a thumb over a stripe.

What it left was a bar that, once drawn, stayed drawn — for the whole life of the screen,
whether or not anything had ever scrolled. The reference's does not. It **arrives when the
area moves and goes again when the area has been still**, and that is most of what makes a
bar over content bearable to have at all.

## What the reference does

```dart
const Duration _kScrollbarFadeDuration = Duration(milliseconds: 300);   // :17
const Duration _kScrollbarTimeToFade   = Duration(milliseconds: 600);   // :18
```

- **Any movement shows it** (`scrollbar.dart:1958`) — "Any movements always makes the
  scrollbar start showing up" — and cancels whatever fade was pending.
- **The scroll ending starts the clock** (`:1969`), unless the thumb itself is being
  dragged: 600 ms, then 300 ms of going.
- **It starts closed.** Nothing opens the fade but a scroll notification, so an area
  nobody has scrolled has no bar at all. A list that has just appeared shows its content,
  not its furniture.
- **A pointer coming near it brings it back** (`:2132`): "Bring the scrollbar back into
  view if it has faded or started to fade away." Leaving restarts the clock.

And the thumb has three colours, not one (`scrollbar.dart:236`–`:248`):

| | dark | light |
|---|---|---|
| at rest | `on_surface` 30 % | 10 % |
| a pointer near it | 65 % | 50 % |
| held | 75 % | 60 % |

The first two are 200 ms apart (`:319`); the third arrives at once, because the hand is
already on it and a fade would only lag behind the grab.

## Nothing said when an area moved

The offset is written from the drag handler, from the fling, and from the spring, and none
of the three leaves word. Asking each of them to remember to say so is three chances to
forget, and a fourth the next time something else learns to scroll.

So the runtime **watches the value**: `ScrollbarFade` keeps the offset it saw last frame,
and any difference is movement, whoever caused it. That is one place instead of three, and
it cannot be forgotten by a writer that does not exist yet.

It is advanced inside `advance_scroll`, which is where the offsets settle for the frame,
and it keeps the frame loop awake while a bar is waiting to go — a wait is not a movement,
but it still needs a frame at the end of it.

## Two hit tests, not one

A pointer coming near a faded bar has to be able to find it, or the fade would make the
bar unusable rather than unobtrusive. But a bar at zero must not be **grabbable**, or a
click near the edge of a page would silently take hold of something invisible.

The reference splits exactly there — `hitTestInteractive` (`scrollbar.dart:748`) answers
for a transparent bar when a mouse is hovering and for nothing else (`:769`);
`hitTestOnlyThumbInteractive` (`:790`) refuses outright (`:798`) — and so does this:

- `Ui::scrollbar_at` — a drag. The thumb's own rectangle, and only above zero.
- `Ui::scrollbar_near` — a pointer. The track, widened to take in a tap target's worth of
  room around the thumb (`:762`), at any opacity at all.

Which makes a bar the platform does not draw and a bar that has faded out two different
things, as they should be: the first is not registered, because it is not there; the
second is registered and invisible, because it is there and waiting to be asked for.

## `thumb_visibility`

The reference's own escape hatch (`scrollbar.dart:214`), and the reason it is needed here
too: an area whose content does not look scrollable, or one a reader should be able to aim
at without scrolling first to make the bar appear. It is also what a **static frame** needs
— a golden renders one frame and advances nothing, so nothing has ever moved in it.

It is a separate question from `Scrollbars`, and stays a separate hook, because the
reference asks them in separate places: `ScrollBehavior.buildScrollbar` decides whether an
area has a bar; `Scrollbar.thumbVisibility` decides whether that bar stays.

## And the thumb was the wrong 8

`BAR_SIZE` is the **thumb's** thickness. This drew a 6-pixel thumb inside an 8-pixel slot
— the same arithmetic with the wrong number left over. The reference keeps the thumb at 8
and holds it clear of the edge with `crossAxisMargin` (`scrollbar.dart:357`, `:14`), which
is 2. So: 8 wide, 2 from the edge, fully rounded.

## The tests

- `a_bar_arrives_on_movement_and_goes_again` — untouched: nothing; moved: on its way in,
  not snapped; still for half a second: still there; and then gone.
- `a_bar_stays_for_the_pointer_that_wants_it` — a held thumb does not fade out from under
  the hand; a bar that has gone entirely comes back for a pointer, and warms.
- `an_untouched_area_shows_no_bar_but_can_still_be_reached_for` — nothing painted,
  registered all the same, not grabbable, reachable, and the reach is wider than the bar.
- `a_pinned_bar_does_not_fade`.
- `a_thumb_answers_the_pointer` — three colours, and the dragged one does not fade in.

Five of the six fail with 444's behaviour put back.

## Still open

- **A track under a pointer.** The reference thickens to 12 and paints a track
  (`scrollbar.dart:302`) — but only when `trackVisibility` resolves true, and its default
  is `false` (`:222`), so the *default* desktop bar does neither. There is no
  `trackVisibility` here yet, and until there is, there is nothing for the thickening to
  be conditional on.
- **A golden cannot show a fading bar.** A static frame advances nothing, so every
  scrollbar in the golden set is now absent unless pinned. Nine goldens lost their thumb
  in this milestone and that is correct, but it means the bar's own appearance is covered
  by unit tests alone. A test that drives frames would fix this and several other things;
  it is recorded elsewhere.
