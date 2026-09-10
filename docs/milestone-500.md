# Milestone 500 — The batch planner, made linear without changing a single plan

Answers [#16](https://github.com/KalybosPro/frus/issues/16).

The planner gives every primitive a level from what it covers among the primitives before
it, and it found what it covers by testing every member of a level in turn. A long list
puts nearly every row's background on one level, so every primitive met every row before
it: the cost grew with the square of the scene.

After this milestone, sixteen times the primitives cost sixteen to twenty-one times the
time; before it, sixty-seven to eighty-four.

## The rules

The issue set two, and both shaped the work more than the index did:

- **The plan must not change.** Not "should agree in practice": the batches, their kinds,
  their members and their order, bit for bit.
- **Bring numbers.**

## The first number was wrong, and so was the second

Before touching anything, the baseline was measured on this machine: 403 µs to plan 1102
primitives. An hour later, the same `HEAD`, untouched, measured 579 µs; later still, 351 µs,
then 500 µs. **The same code swung by more than half across four runs**, because what else
the machine was doing — an editor re-indexing the files just edited, for one — lands in the
measurement.

So nothing in this milestone is compared with a number taken at another time. Every
comparison below is **A/B, back to back**: the old planner is put back, benched and saved as
a criterion baseline, and the new one is benched against it straight after. The first
comparisons made the other way are recorded as what they are: two samples of two machines.

## Why an index can be exact at all

What the planner asks a level is two things: *does anything here overlap this primitive?*
and *is anything that does of another kind?* Two **existence** questions. Neither depends on
which member answers or in what order the members are asked, so a structure that asks fewer
of them gets the same two booleans the full scan gets — provided it never leaves out a
member that would have said yes.

That proviso is one paragraph of argument. Two rectangles that `overlaps` says share area
have vertical extents that share a point; that point lies in one band of the index; and it
lies inside both rectangles' band ranges because the band function (`floor(y / 64)`,
clamped) is monotone. So a member the index does not offer to a query is one that cannot
overlap it. The index only ever **saves** comparisons. It never skips one that would have
answered, whatever the rectangle — a negative height from a `union`, a coordinate a hundred
million pixels off, `UNBOUNDED`, a NaN — and those go on a short list every query reads when
no band range would mean anything.

The final answer is still `overlaps`, applied to every member the index offers. The plan
cannot change because the question that decides it has not.

## What was tried first, and lost

**A two-dimensional grid**, hashed, built once a level held twenty-four members. It made the
planner nearly linear — and made a twelve-row list about **three times slower** to plan.
That one comparison was made across an hour rather than back to back, so it is worth only
its order of magnitude; but noise of the size measured above does not make three times out
of nothing.
Every row of a list was written into seven to fourteen cells, each a hashed entry with an
allocation of its own, and the rows of a scrolled list are spread down the whole content,
not the viewport: a row scrolled out of sight is clipped to no height at all, but it keeps
its place.

**The data said what to do instead.** An interface stacks vertically — the rows of a list,
the sections of a page, the cards of a feed — so height alone rejects almost everything a
query could meet. The index became **horizontal bands**: one or two insertions a row
instead of fourteen, a plain table instead of a hashed map, and the argument above in one
dimension instead of two.

## The tests did not bite, and that was found by breaking the index on purpose

The index was checked against the old planner, kept verbatim in the tests, over three
hundred random scenes of up to nine hundred footprints of every sort. Green. Then two
off-by-ones an index like this could plausibly ship with were put in on purpose — a member
written into every band it crosses *but the last*, a query that reads *only its first band*
— and **every test stayed green**.

The generator was the hole. One footprint in eight covered everything — `UNBOUNDED`, or a
backdrop — and six hundred of them meant dozens of members that overlapped every query and
answered it themselves, whatever the bands held. A test of equivalence that a half-broken
index passes is not a test of the index.

So a second generator builds **sparse pages**: small things, rows at any height so that most
straddle two bands, tiles lying exactly on band boundaries, things with no height or a
negative one, and nothing that covers everything. There, a query usually meets one member
or none, so the member the index fails to offer is the member that decides the answer. And
the white-box test now **counts the queries whose answer hung on a single member** and fails
if there are too few — so that it cannot quietly go back to being unable to fail.

Both mutations now fail two tests each. A third — anchoring the index on its first member —
fails the test written for it.

## Two more, found by measuring rather than by reading

- **Work done for an index that did not exist.** `probe` worked out the query's band range
  before looking to see whether the level had an index, which the small levels — most of
  them — do not. A division, a floor and two clamps, for every level every primitive meets.
  Measured as a 21% regression on a scene too small to build an index at all — in the
  order that favours the new code, so the bias described below could only have hidden it
  — which is what said the cost was not the index.
- **An origin taken from whichever member came first.** One footprint far off the page, if it
  arrived first, became the table's origin; every ordinary member after it was then too far
  away to be written in and went on the list every query reads. Still exact; as slow as the
  scan it replaced; and nothing would have said so. The table is now anchored on the median
  member.

## And a quadratic that was not measured yet

Turning levels into batches looked for each member's batch by walking the level's batches,
and a composited group is a batch of its own — so a level with many groups walked past all
of them for every member. At most one batch of each drawable kind exists per level, and it
is the first; a four-entry table answers the same question without the walk.

## The numbers

The final code, back to back **in both orders** on the bench the issue names — the old
planner first and then the new, and the new first and then the old:

| primitives | old, then new | new, then old |
|---|---|---|
| 68 | 8.30 → 5.41 µs | 5.20 → 4.44 µs |
| 332 | 75.7 → 36.3 µs | 37.8 → 43.6 µs |
| 1102 | 553 → 113 µs | 107 → 372 µs |

Both orders, because one is not enough. **At 68 primitives, whichever ran second was faster,
both times.** The first half of a run is the one that meets whatever the machine is still
doing after the build, so a single ordering flatters the code it puts second — by more than
the difference being measured.

Read with that in mind:

- **1102 primitives: about four times faster** — 3.5× in the order that favours the old
  planner, 4.9× in the other.
- **332: faster in both orders**, by 13% in the one that favours the old planner.
- **68: nothing can be said beyond "under a microsecond, either way".** Across the five
  comparisons made with the last three versions of the code, the new planner measured
  anywhere from 35% faster to 17% slower here, and the order moved the result by more than
  the versions did.

That last one cost a detour worth recording. Earlier runs, most of them with the new code
going first, had read as a steady small-scene regression, and two changes were tried to
close it: marking the index's methods for inlining, and moving the index out of line so that
a level without one stays small. Neither closed anything a two-order comparison could see.
**The inlining hints were taken back out** — a change that did not do what it was added for
is not one to keep. The smaller level stays, because it stands on its own: the index is the
rare case and the level is not.

**The shape is the result.** From 68 to 1102 primitives — sixteen times as many — the old
planner cost 67 to 84 times as much, and the new one costs 16 to 21 times as much: roughly
linear. The draw calls are 3, 5 and 5 on both sides, printed beside every timing so that a
bench reporting only nanoseconds could not hide the plan changing.

## Verification

- The eight original planner tests, unchanged.
- The index against the old planner, kept verbatim as `plan_linear`: three hundred scenes,
  half sparse pages and half every sort of footprint including NaN, infinities, negative
  extents and `UNBOUNDED`; and a frame-shaped scene of four hundred rows through `plan`
  itself.
- A busy level opened up: indexed, the right three members on the wide list, and four
  thousand queries answered as a scan answers them — more than four hundred of them hanging
  on a single member.
- A stray first member does not become the origin.
- Three mutations, each failing the tests it should.
- The goldens, which draw through this planner, unchanged.
