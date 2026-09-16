# Milestone 533 — An offset past the end, brought back when the content shrinks

A bug seen on a phone. A scroll region kept its offset when the content under it shrank, so
content that fitted again was still drawn scrolled.

## Seen

On a Huawei phone, running a release build of the demo from the owner's branch. The home
screen's filter row (a segmented button: All, Active, Done) had been wrapped in a horizontal
`SingleChildScrollView`:

1. At the normal text size the row fitted, and nothing scrolled.
2. The demo's text scale was raised twice. The row overflowed, with "Done" cut at the right. A
   swipe scrolled it so "Done" showed in full and "All" was cut at the left. Correct so far.
3. The text scale was lowered twice, back to normal. The row fitted its viewport again, **but it
   stayed scrolled**: "All" cut at the left edge, and empty room to the right of "Done".

## Reproduced

As widget tests that read what is painted, and then the runtime, in
`scroll.rs`'s `shrunk_content_tests`. A strip 100 wide holds a marked box 300 wide, and its
offset is set to 200: the end of a swipe that has settled, with nothing left owning the offset.
The strip is then built with a box 80 wide. Before the fix:

- The box was **painted at x = -200** in a viewport 100 wide: nothing of it on screen.
- With a box 150 wide, the box spanned -200 to -50, where its end belongs on the viewport's
  right edge at 100.
- A reversed strip, whose offset counts from the right, painted its box at x = 150, off to the
  right.
- **Down the screen, the same.** A `ListView` of ten 40-pixel rows, scrolled to its bottom, lost
  four rows. Its last row was painted with its bottom at 40 rather than on the floor at 200.
- A `PageView` on its fifth page that was rebuilt with two pages built and painted page four,
  which no longer exists, and never page one.
- A bouncing fling in flight when the content shrank from 400 to 100 settled at 400.2 and
  stayed there.

Six of the new tests failed that way. The two guards, a glide and a held region, passed before
and after, as they should.

## The cause

The offset lives in `Runtime::scroll`, keyed by region. Three things write it (a finger, a
fling, a glide) and each clamps against the extent while it runs. None runs once a region is at
rest:

- `Runtime::advance_scroll` (`runtime.rs`, the glide loop) visits only the regions in
  `scroll_target`. A drag's target is removed once its glide settles, so a region at rest is
  never visited again.
- The walk (`ui.rs`, the scroll branch, then the list and the paged view) read the retained
  offset as it was. It never compared it with the `max_x` and `max_y` it had just measured two
  lines above.

So a region at rest whose content shrank kept an offset past its new end, for as long as nobody
touched it.

## What the reference does

A scroll position is told the new content dimensions during layout. When they differ, it asks
its physics to adjust the pixels for the new dimensions, and corrects them in place if the
answer differs, before painting. The default physics on every platform sits over a
range-maintaining physics, which:

- leaves an **animating** position alone (a velocity that is not nought);
- keeps an **overscrolled** position the same distance past the new end;
- otherwise **clamps** a position that was in range to the new range.

It then tells the current activity. An idle one starts a ballistic at velocity nought, which
springs an out-of-range position home. A ballistic one starts again from where it is, with the
new dimensions. A drag does nothing.

## Decision

**One rule, stated once:** a region **nobody owns** keeps its offset inside `[0, max]` on both
axes. "Nobody owns" is `Runtime::scroll_at_rest`: no finger, no fling, no glide. It is the same
three writers the runtime already names as the owners of an offset.

- **Corrected at once, not sprung.** This is the reference's clamp for a position that was in
  range and is not animating, which is the reported case.
- **In the frame that shrinks it.** `Runtime::scroll_offset_within(id, max)` is what the walk now
  reads in its three places (a scroll view, a virtualised list, a paged view): the retained
  offset, clamped when nothing owns it. The frame whose content shrank already shows the content
  where it rests, as the reference corrects during layout. Correcting only on the next frame
  would draw one wrong frame, and on a screen that then goes idle, that frame stays.
- **Retained afterwards.** `Runtime::keep_scroll_in_range(regions)` writes the same answer back.
  The shell calls it right after the build, with that frame's regions, before the scroll and
  page reports and before any press reads the offset. `advance_scroll` calls it first, which is
  also how the test harness's `Stage` gets it.
- **Owners keep what they have.**
  - A **finger** is not overruled: the picture and the runtime stay where it left them. Its
    release already springs an out-of-range offset home (`fling_scroll` at velocity nought).
  - A **glide** keeps its spring, which already pulls a target past the end back inside.
  - A **fling** keeps running. Its simulation was built for the old end, so a bouncing one
    settles there, past the new end. At that moment it now **springs on** to the new end, as
    the reference restarts its ballistic on new dimensions, instead of leaving the offset to the
    correction at rest, which would jump there in one frame. A clamping fling already clamps to
    the current extent every frame.
- **No scrollbar for it.** The bar arrives when the offset moves. A correction is not a scroll,
  so the fade is told the new offset when it had seen the old one.
- **Reversed axes need nothing of their own.** An offset counts from the axis's start in both
  directions, and the range is `[0, max]` either way. A horizontal region is not mirrored by a
  right-to-left layout here; only `reverse()` flips it, and that case is tested.

**Not taken: keeping an overscroll's distance past a smaller end while it is held.** Here, a
finger that holds an overscroll while the content shrinks keeps its offset until it lets go,
and the release springs it home. The reference would shift it to stay the same distance past
the new end. That is rarer than a text size change, and it would need the previous frame's
extent kept per region. It is left.

## Verification

**Tests** — nine new.

In `scroll.rs`, `shrunk_content_tests` (6), each reading the scene's painted rectangles:

- `a_row_that_fits_again_is_drawn_from_its_start` — the phone's report. Painted at 0 in the
  frame the row fits, and retained at 0.
- `a_row_that_still_overflows_rests_at_its_new_end` — painted from -50 to 100, and retained at
  50. No scrollbar comes for it.
- `a_list_that_loses_rows_keeps_its_last_row_on_the_floor` — vertical, a `ListView`. Ten rows to
  six: the last row's bottom at 200, retained at 40. Then three rows: all three show from 0,
  retained at 0.
- `a_paged_view_that_loses_pages_shows_its_new_last_page` — five pages to two, on the last:
  page one fills the viewport, nothing past it is painted, retained at 300.
- `a_reversed_row_is_brought_back_the_same_way` — painted at 0, not at 150, and retained at 50.
- `a_held_row_is_not_yanked_from_under_the_finger` — held, it is painted at -200 and retained at
  200 for ten frames. Released, it springs home to 50.

In `runtime.rs` (3):

- `a_fling_the_content_shrank_under_springs_on_to_the_new_end` — a bouncing fling at 3000 px/s
  aimed at 400, with the content at 100, ends within a pixel of 100. No frame moves it 120
  pixels or more (a jump would be 300), and nothing is left running.
- `a_glide_the_content_shrank_under_is_not_snapped` — still above 300 one frame in, and home at
  100.
- `an_offset_nobody_owns_is_kept_inside_the_content` — both axes, both ends of the range. It
  answers whether it corrected, and it creates no entry for a region nobody scrolled.

Before the fix, six failed. `an_offset_nobody_owns_is_kept_inside_the_content` names the new
method and was gated out of that run. The glide and held tests passed, being guards.

**Mutations** — six, all killed. A script applied each one after checking its target occurred
once, ran the new tests, wrote the file's original bytes back and compared the working tree's
diff hash with the one it started from. They ran one at a time, and each was restored.

1. The scroll view's walk draws the raw offset — killed by the row that fits, the row that
   still overflows, and the reversed row.
2. The list's walk measures against no extent — killed by the list that loses rows.
3. The runtime never corrects — killed by six: every widget test that checks what is retained,
   and the runtime's own.
4. The runtime corrects an offset something owns — killed by the held row and the glide.
5. The scrollbar's fade is not told — killed by the row that still overflows, whose bar came.
6. A fling settled past the new end does not spring on — killed by the fling test, which saw
   the offset jump to the new end in one frame.

**Checks** — all clean:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p frus-widgets --lib` (1628 passed, against 1619 before)
- `cargo test -p frus-shell --lib` (108)
- `cargo test -p frus-demo --lib` (61)
- `cargo test -p frus-test --no-fail-fast` (goldens 102, widgets 47, and the rest; **no golden
  changed** and no `.actual.png` was written anywhere in the tree)
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`

The shell's call after the build has no test of its own: the shell's frame is not driven by any
test harness. The widget tests cover the rule, and the walk draws the corrected offset whether
or not the shell retains it. What the shell's call adds is that a press, the scroll reports and
the next frame read the same offset the frame was drawn with.

## On the device — the Huawei STK-L21, a release build of the demo

The filter row overflowed the text scale twice (the demo's "A+"), swiped to its end so "Done"
showed in full, then the scale was lowered back to normal (the demo's "A−"). The row snapped
back exactly to its start: "All" flush at the left edge, "Done" flush at the right, nothing cut
and no gap where the scrolled offset would have left one — the state a fresh build of the row
would draw, not a remnant of where the finger had left it. A capture right after confirmed it.

Not tried on the phone: raising the scale by less (so "Done" would land exactly on the edge
rather than fully past it), holding the row while the scale changes, and shortening a long list
mid-fling. Left for whoever picks those up next.

What was tried, for reference:

1. Raise the text scale until the row overflows, then swipe it to its end.
2. Lower the text scale until the row fits. It should snap back so "All" sits at the left edge,
   with nothing cut and no scrollbar flash.
3. Raise the text scale a little less, so the row still overflows slightly, then swipe to its
   end and lower one step. "Done" should sit on the right edge.
4. Hold the row with a finger while the text scale changes (from a second hand or a timer, if
   possible). It should not move under the finger, and should settle when released.
5. Fling a long list and, while it is still moving, delete items so the list gets shorter. It
   should come to rest on the new last row without a jump.

## What is left

- **A held overscroll is not kept the same distance past a smaller end**, as said above.
- **A fling that is not bouncing is untouched.** Under clamping physics, a fling in flight when
  the content shrinks is pinned to the new end in the frame it next samples, which can be a
  large step. That already happened before this milestone, and the reference's clamping
  simulation stops dead at an edge as well.
- **Content that grows back does not restore the offset.** A row that fitted and then overflows
  again opens at its start, as the reference's does.
- **A restored offset larger than a region's first frame** — a screen coming back whose content
  is not yet at full height, pictures still loading, say — is now clamped to that first frame's
  extent. The reference exempts a list whose extent is still unknown; nothing here has an
  unknown extent.
