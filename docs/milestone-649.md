# Milestone 649 — Never and always scrollable

## Objective

Reported missing: the reference's `NeverScrollableScrollPhysics`. frus had two physics,
bouncing and clamping, and with them only the reference's default rule for when the user
may scroll an area: when there is content outside the viewport to reveal. The reference
has two more physics that change that rule, and a way of putting physics together:

| | reference | frus before |
|---|---|---|
| `NeverScrollableScrollPhysics` | the user's drag, fling and wheel never move the area; the area around it takes them; code still can | none |
| `AlwaysScrollableScrollPhysics` | the user may always scroll, even with everything in sight: a short list bounces, and keeps the drag from the page around it | none |
| putting them together | a chain, `AlwaysScrollable(parent: Bouncing())`, laid over the platform's by `applyTo` | an area's physics replaced the platform's |

The first is the one applications reach for most: a list laid out in full inside a page
that scrolls, or a carousel that only turns in code.

## What was done

- **`ScrollPhysics` is two settings**:
  - a **feel**, `ScrollFeel` — bouncing at a rate, or clamping: the old two kinds;
  - a **rule**, `UserScroll` — `WhenOverflowing` (the default), `Always` or `Never`.

  Either may be left open. `ScrollPhysics::NEVER` and `ScrollPhysics::ALWAYS` settle the
  rule and leave the feel to the platform. `BOUNCING`, `CLAMPING` and `bouncing(rate)`
  settle the feel. `always_scrollable()` and `never_scrollable()` combine the two, written
  the way round the reference's chain is: `ScrollPhysics::BOUNCING.always_scrollable()`.
- **`apply_to`**: an area's physics laid over its scroll behaviour's, as the reference's
  `applyTo` lays a widget's over the scroll configuration's. What the area settles it keeps,
  what it leaves open the platform decides. Every scrollable resolves its physics this way
  when it is registered, where before its own replaced the platform's whole.
- **The rule is asked in one place**:
  - `Scrollable::accepts_user_offset` answers for the wheel and for a drag's start;
  - the new `Scrollable::takes_user_scroll(down)` answers for a movement in a direction.

  The shell's three copies of the old test call the latter: which area a drag is for, which
  axis it claims, and whether a pan loses to a scroll. A never-scrollable area takes
  nothing, and the drag goes on to the area around it; an always-scrollable one takes a
  movement along its axis with nothing to reveal.
- **`Scrollable::axis`**: the axis an area scrolls along, which is what "always" needs when
  nothing overflows.

**Changed**: `ScrollPhysics` is a struct. `ScrollPhysics::Clamping` is
`ScrollPhysics::CLAMPING`, `ScrollPhysics::Bouncing(rate)` is
`ScrollPhysics::bouncing(rate)`, and code that matched on the kind matches on `feel()`.
`Scrollable` has `axis`.

## Decisions

- **The default rule stays frus's.** The reference makes a vertical scroll view with no
  controller of its own always scrollable by default (`ScrollView`'s `primary`). A short
  list inside a page then keeps the page from scrolling under the finger, which is the
  most common reason the reference's applications reach for `NeverScrollable`. frus lets
  such a list pass the drag on, as it always has; `ALWAYS` asks for the reference's
  behaviour. Changing every nested list's behaviour is a decision of its own, and this
  milestone does not take it.
- **Two settings rather than a chain.** The reference's chain has five links in practice,
  each answering a few questions and passing the rest up. Flattened to a feel and a rule,
  the same compositions read the same, stay `Copy`, and cannot be built in an order that
  means nothing.

## Not done here

- `RangeMaintainingScrollPhysics`: frus keeps an offset in range in its runtime instead
  (milestone 533).
- `PageScrollPhysics` and `FixedExtentScrollPhysics`: a paged view and a wheel snap through
  their own page and extent settings.

## Tests

- `frus-widgets`:
  - the four constants; `apply_to` keeping what is settled and taking what is open, both
    ways round; nothing settled being clamping and the default rule;
  - an area's own physics laid over its behaviour's in the registry: never keeping the
    platform's bouncing and taking no finger with content to reveal, always keeping the
    platform's clamping and taking one with nothing to reveal, a feel given keeping the
    default rule.
- `frus-shell`: a never-scrollable list inside a page leaving the drag to the page, and
  alone keeping nothing; an always-scrollable short list keeping a drag down from the page
  along its axis, and an always-scrollable strip across leaving it.
- The existing physics, runtime and gesture tests, with the constants renamed.

Mutation testing: 11 mutants, all killed. They cover:

- `apply_to` keeping a feel and inheriting a rule;
- the default feel, and the two combinators;
- never and always in `accepts_user_offset`;
- the axis rule of `takes_user_scroll`;
- the registry laying an area's physics over its behaviour's;
- the shell's choice of area, and its fallback refusing a never-scrollable one.
