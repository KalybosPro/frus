# Milestone 294 — Drawing in the order the scene asked for

Milestone 291 built a bottom app bar with a notch cut in it, put a filled button on
that bar, and the device showed the button as bare text. A test proved the scene had
them in the right order. The renderer was why: it drew one pass per kind of primitive,
in a fixed order.

```
rect → image → path → text → composite
```

So **every path covered every rectangle in the frame**, wherever the two sat in the
scene. Nothing had noticed because a notched bar is the first thing frus builds that
puts a path *underneath* something. The defect was never about bars: it applies to
`CustomPaint`, the charts, `ClipPath`, the overscroll glow — anything drawn as a path
with anything drawn as a rectangle on top of it.

## Not one call per primitive

The obvious fix — a draw call per primitive, in scene order — is correct and costs
about two calls per widget. A dense list would be several hundred draw calls a frame
to solve a problem that shows up in one of them.

So the renderer now does what a 2D renderer normally does, and the new `batch` module
is where it is decided. Every primitive gets a **level**, from what it covers among
the primitives before it:

- it covers something of **another** kind → one level above that, since another kind
  means another draw call and the calls run in level order;
- it covers something of its **own** kind → the same level is enough, because a batch
  draws its members in scene order and so already puts it on top;
- it covers nothing earlier → level zero.

Two primitives on one level therefore never need ordering against each other, so a
level costs one draw call per kind it holds.

The distinction between the two overlap rules is what makes this cheap, and it took a
wrong version to see it. Ordering primitive against primitive, ignoring that a batch
is internally ordered, puts a checkbox's tick above every later row's background and
costs two calls a row: a twelve-row list measured **25 draw calls**. With levels, every
tick in the scene lands on one level above every checkbox, and the same list measures
**3** — the card's rectangles, the ticks and icons above them, the buttons above those.

The painters fill their buffers in **batch order** rather than scene order, so a batch
is one contiguous range of instances or indices and therefore one call.

## What is over-estimated, and in which direction

A path's bounds include its control points rather than solving the curve, and a
rectangle's include the spread of its shadow and a stroke's half-width. All of these
over-estimate what a primitive covers, which costs a level too many and never a level
too few. That is the only safe direction: under-estimating would put something behind
what it should cover, which is the bug this exists to fix.

## Text is not in the plan, and that is now a written rule

A `Primitive::Text` records where it starts, not the box it was laid out in, so the
planner cannot know what it covers. Over-estimating it — down and right to the clip's
edge — would be correct and would break nearly every level in a frame, which is worse
than what frus already did.

So text keeps a pass of its own, above everything else in the frame, and the rule is
written down rather than accidental: **text paints above the other primitives of its
frame; covering text needs a layer.** Giving text its laid-out bounds and folding it
into the plan is on the roadmap.

The decoration quads (underline, strikethrough) are rectangles, and they move with the
text they belong to rather than into a level of their own.

### And the rule is already broken in practice

Written down and then immediately tested against the device, which is the right order
to find this out in. Opening the demo's overflow menu shows the labels *underneath* it
— "Add", "Done", "Buy milk" — reading straight through the panel. Any menu, dropdown,
dialog or sheet over text does it, because no widget in frus uses `scene.layer`: an
overlay is an ordinary container drawn late, and the text beneath it is drawn later
still.

This is not new and it is not from this milestone: text has been a final pass since the
renderer was written, and the golden `table_column_menu.png` — committed, green, and
byte-identical before and after this change — has the Score column's "5" and "3"
showing through the menu panel. It was blessed that way.

So "covering text needs a layer" is a statement of what the renderer does, not a design
anyone should keep. It is recorded on the roadmap as a defect with this evidence rather
than as an enhancement, and the fix is the same one named above: give `Primitive::Text`
the box it was laid out in, and the planner can order it like everything else.

## Verification

- `cargo test -p frus-gpu` — 22 passing, six of them new and none needing a GPU: the
  planner is pure logic. They cover the case that has to stay cheap (things that do
  not touch share a call), the case that broke (a rectangle over a path is drawn after
  it), the same rectangle moved clear (it rejoins the batch), the spread of a shadow
  and a stroke, that text and layers are left alone, and — the one that matters most —
  an invariant check that reading the batches in order never puts a primitive before
  an overlapping one that came after it.
- `frus-test` gained `a_real_screen_still_batches`: a twelve-row list of checkboxes,
  labels, icons and buttons, asserting the whole thing costs no more than four draw
  calls. It is three today. This is the guard on the trade — a correctness fix that
  quietly cost one draw call per widget would pass every other test in the repo.
- **The golden suite: byte-identical.** All 77 goldens render exactly as they did
  before the change, the 47 currently-failing ones included — the same bytes, failing
  the same way. Nothing in the suite puts a rectangle over a path, which is why the
  device found this and the tests did not.
- `frus_gpu::draw_calls(scene)` is public, so an application can measure its own
  screens.

## The goldens were already red

Which is the other thing this milestone found, and it is worth more than the fix.

Running the golden suite: **47 of 77 failed** — and they failed identically with the
change stashed, so this was not the cause. Bisecting: at `5082506`, the commit before
milestone 289, all 77 pass. **Milestone 289 broke them**, and was committed on "812
tests, 0 failures".

That milestone was right. It made text measurement round *up*, because a natural width
of 146.4 was becoming a box of 146, narrower than the text that asked for it, and the
text then wrapped when it was painted and overlapped what came below. Fixing it moves
text by up to a pixel, which moves everything laid out beside it — the data table's
sort arrow sits a few pixels over because the header it follows is now measured a
fraction wider. The goldens were stale, not wrong, and they are re-blessed here after
looking at them.

The interesting question is why five milestones passed without anyone noticing:

- the routine command is `cargo test --workspace --exclude frus-gpu --exclude frus-test`,
  and every milestone note since 276 records its test count from exactly that line;
- CI's headless job runs the same command;
- CI's GPU job **does** run the goldens — with `continue-on-error: true`, because
  lavapipe rasterises differently from real hardware. Advisory, and so unread.

An advisory check that goes red stays red. So the job is split: the GPU-backed tests
that assert on numbers rather than pixels — clipping, transforms, batching — are now
**required**, and only the golden step keeps `continue-on-error`. That is a smaller
promise than "goldens are enforced", and it is one the CI machine can actually keep;
making the goldens themselves required wants a channel tolerance validated against
lavapipe, which is a roadmap item rather than a guess made here.

`CONTRIBUTING.md` now says plainly: if you change text measurement, layout, painting or
the renderer, run the goldens yourself, look at the `.actual.png` files, and only then
accept them.
