# Milestone 369 — `Icons`, because that is what it is called

Milestone 367 corrected twenty widget names against the reference's and stopped at the
widgets. `IconName` is not a widget, which is why the sweep did not reach it — and it is
the name someone types more often than most of the twenty, because every button, chip and
tile with a picture on it goes through this enum.

The reference calls the set `Icons` and reaches into it as `Icons.chevron_right`. Ours
said `IconName::ChevronRight`. Same idea, different word, and the argument from 367 applies
unchanged: somebody who knows the reference types what they know, does not find it, and
goes looking for something that was there all along.

## The stutter is the right trade

The module is `icons` and the type is now `Icons`, so the full path reads
`crate::icons::Icons`. That is a stutter, and Rust's own conventions dislike it.

It costs nothing here. The module is **private** — every name in it reaches the outside
through `frus_widgets::Icons`, which is the only path anybody writes, and it does not
stutter. The alternative was to keep a name nobody would search for in order to tidy a path
nobody sees. Milestone 367 left the twenty module files alone for the same reason: what a
reader types is the public name, and the file behind it is ours to call what we like.

## What was checked before the sweep ran

`IconName` never appears inside a string literal and is never a prefix of a longer
identifier — both checked, because a rename that reaches into quoted text changes what is
printed on a screen, and 367 had to unpick exactly that by hand four times. There was no
existing `Icons` to collide with, only the word in prose.

The historical record keeps the old name. `docs/milestone-*.md` before this one, and the
CHANGELOG and ROADMAP entries for the milestones that used it, still say `IconName` —
they describe what was true when they were written, which is the same rule 367 followed.

## One slip from 367, carried in the previous step

`grid.rs` described taffy's layout algorithm as *CSS GridView*: the sweep had caught the
name of the **CSS** algorithm, which is not our widget. A pass over all twenty names says
it was the only one. It went in with milestone 368, which was the commit in flight.
