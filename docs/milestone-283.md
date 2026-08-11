# Milestone 283 — A paged view, and where a release is allowed to stop

## The whole feature is one moment

A paged view shares everything with an ordinary scrollable: the same drag, the same
edges, the same glow, the same physics on the boundaries. What it changes is a single
instant — **the one the finger lifts** — where a fling is replaced by a spring to
exactly one page.

That is why this is not a new scrolling machine but one method,
`ScrollPhysics::page_ballistic`, sitting beside the ordinary `ballistic` and chosen by
the release path when the region says it is paged. Everything else in the gesture is
code that already existed and did not have to learn about pages.

## The rule is not "nearest page"

The obvious rule — go to whichever page is nearer — is the wrong one, and every paged
view on every platform rejects it. The rule is:

```
page = current page as a fraction
if the release had speed:  page ± 0.5, the way the finger went
target = round(page)
```

So **any** release above the tolerance is a flick and turns the page, however short
the drag was. A tenth of a panel, thrown, is a page turn. Only letting go slowly falls
back to rounding.

Two consequences worth stating, because both are deliberate:

- A flick **back** after dragging most of the way forward returns to where it started.
  The velocity is the more recent statement of intent, and it wins over the distance.
- The release velocity is passed to the physics **raw**, not filtered through
  `min_fling_velocity` the way a scroll's is. A scroll asks "was this thrown hard
  enough to coast?"; a page view asks only "which way did it go", and 60 px/s answers
  that as clearly as 2000.

A target is clamped into the content before it is used. With pages narrower than the
viewport the last page does not sit on a page boundary, and rounding past the end is
not a page — it is an overscroll.

## Past an edge, the ordinary physics takes over

Dragged past the first page and released still heading out, there is no page to spring
to. The paged path hands the release back to `ballistic`, which bounces or clamps
according to the platform. Overscroll behaviour is not something a paged view should
have an opinion about.

## Two directions, one number

`PageView` has a `page(n)` and an `on_page_changed(msg)`, and the demo's walkthrough
uses both at once: the finger reports where it got to, the picker writes where it
should go, and the application holds the single number in between. Neither side owns
it, so neither can drift from the other.

Making that work needs two rules that are easy to get wrong:

- **A request is honoured when it changes**, never re-asserted. The widget is rebuilt
  every frame carrying the same number; a view that obeyed it every frame could not be
  swiped at all — the finger would move the offset and the next frame would put it
  back.
- **The first sighting is the initial page**, and it arrives without an animation. It
  is also read in the walk rather than corrected a frame later, so a view opening on
  page 3 does not show page 0 for a frame on the way.

Symmetrically, a view *appearing* is not a page change: the first sighting is recorded
silently. An application that opens on page 3 already knows it is on page 3, and being
told so on the first frame would only invite it to answer.

Changes are reported **as soon as the rounding tips** — mid-drag, not once the spring
has settled. A title above a gallery should follow the picture, not trail it.

## A page is given its box

Pages are built on demand and only the ones the viewport touches exist, which is the
virtualised list's bargain: a hundred-page walkthrough costs what a two-page one costs,
and a page has no retained state while it is off screen.

Laying one out surfaced a real gap. Content laid out in the ordinary way is *asked*
how big it wants to be; a page is *told*. A panel whose content is one centred line
would have hugged that line, leaving the rest of the panel empty and unclickable. So
`Constraints::filled` and `Layout::compute_filled` were added: both axes definite, and
an unset (`Auto`) root dimension takes the box's size. The `Auto`-patching trick was
already there for the cross axis of a single-axis scrollable (milestone 269); this is
the same idea applied to both axes at once, for content that is handed a box.

`PageView` also repeats `Scroll`'s flex-basis correction: with `flex(1)` and no
explicit size, the default dimension must not stand as a basis, or the view would need
200 px of free room before it grew at all.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **760 tests, 0
  failures** (743 at milestone 282): 6 for the snapping rule, 8 for the view, 3 for the
  runtime's paging state.
- `cargo build --workspace --all-targets` — OK, no new warning (the pre-existing
  `grid_first_error` one in `frus-demo` remains).
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK.

**Not device-verified.** No device was attached for this milestone, so nothing here
was checked on a real touchscreen. What stands in for it is a test that drives the
whole loop below the shell's pointer plumbing — build, drag, release, settle, report —
which is as close as a test without a window gets, and is not the same thing. The
on-device check of milestone 282's swipe is **still owed** as well.

## Also fixed

Four French comments left in `runtime.rs`, `image.rs` and `segmented.rs`, in a repo
that is otherwise English throughout.

## What's left

- **No page transformations.** The panels slide flat. A parallax or a depth effect
  needs each page to know how far it is from the centre at paint time, which the walk
  could pass down but does not yet.
- **`viewport_fraction` does not pad the ends.** Below 1, the first and last pages sit
  flush against the edges instead of being centred like the ones between.
- **No keyboard.** Page Up/Down and the arrow keys should turn a page; today only the
  finger and the application can.
- **The scrollbar is suppressed**, deliberately — a page count is not a distance — but
  nothing replaces it. A view without an application-supplied picker gives no clue how
  many pages there are.
