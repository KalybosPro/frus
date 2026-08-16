# Milestone 316 — One gesture, one axis, one area

Three things reported from a phone in one sitting, all of them the same question asked
three ways: **which scrollable is this drag for, and which way does it go?**

> une page qui se défile verticalement, ne doit pas se défiler horizontalement en même temps

> je vois encore les effets de fin de liste sur la page home

The shell had no answer to either, because it had never asked. A finger scroll took the
area under it, applied `dx` **and** `dy` to that area every frame, and let the physics sort
out what was refused. Every part of that is wrong, and the reference says so in three
different files.

## A scrollable has one axis

`Scrollable` takes an `Axis`, singular. There is no two-axis scrollable in the reference at
all: `setCanDrag` installs a **vertical** recogniser or a **horizontal** one, never both,
and a table that moves two ways is a horizontal scrollable with a vertical one inside it.
The gesture arena then does the rest — the two recognisers compete, the first past its own
slop wins, **and the loser is out of the gesture entirely**.

`Axis::Both` is worth keeping as a convenience, and it is documented now for what it is:
that nested pair collapsed into one node. What it must not do is behave like something the
reference has no word for. So the axis is claimed once, at the threshold, at the same spot
where a swipe and a scroll were already being told apart, and held to the end of the drag:

- an area that can only go one way claims that way whatever the finger did — a diagonal
  flick down a list is a scroll down, not a refusal;
- an area that can go both takes the direction the finger actually went in, ties to the
  vertical, which is the way a page reads;
- **the release follows the same claim**: the unclaimed component of the velocity is
  dropped, or a drag held to one axis would fling off along the other.

Nothing is held back for later. The loser is not a smaller movement; it is not part of the
gesture.

## The area under the finger is not always the one that means it

That is the arena's other half, and it was missing too. A strip of chips that only slides
sideways, sitting in a page that only scrolls down, was taking every drag that started on
it — including a drag straight down the page, which it then could not act on. The page
behind it stayed still. The finger had asked the only widget in reach that could not answer.

So the press no longer takes the topmost scrollable but walks the **stack** under the
finger, and the threshold picks the innermost area that can go the way the finger went.
Where nothing in the stack can, the area under the finger keeps the gesture and goes its own
way — a lone recogniser in an arena still wins. Handing over releases the loser's offset
untouched (it never moved) and catches and holds the winner exactly as the press would have.

## An edge effect where there is no edge

The third report. The home page fits on the screen; dragging it lit the end-of-content glow
anyway, top and bottom, because the drag was accepted, refused by the physics, and a refusal
is what the glow draws.

The reference is unambiguous here and states the rule in a doc comment: *the user can
manipulate the scroll offset if, and only if, there is actually content outside the viewport
to reveal*. When that fails, `setCanDrag(false)` — the recognisers are **removed**. Not a
short drag, not a refused one: no drag.

`Scrollable::accepts_user_offset` is that test, and both the finger and the wheel now ask it
before taking anything. Two exceptions, both the reference's own: content already displaced
(it shrank under an offset it can no longer reach) has to be draggable back, and a pull-to-
refresh area listening above accepts always, so a list of two items still reloads.

It buys something beyond the glow. A dead scrollable no longer **swallows** the press, so
what is behind it — the page, a dismissible row — gets its turn.

## Also here

The demo's drawer wraps its content in `SafeArea`. On the device the "frus" title sat under
the status bar, because a drawer is drawn edge to edge and nothing had removed the inset.
Worth saying plainly: this is the idiom an application writes, not what Material itself
does. There, `Scaffold` hands the drawer a `MediaQuery` that **keeps** the top inset and
`DrawerHeader` adds the status bar height to its own padding, while the side padding of the
edge the drawer does not touch is removed. The frus version pads all four edges from the
outside; in landscape with a notch a right-hand drawer would be indented on the left, where
the reference would not be.

## Verification

1056 tests (2 new in the shell), clippy silent, all 27 goldens read back green, and the
three reports checked on the device they came from.

## What this turned up, and did not fix

Scrolling is not a property of the frame in the reference: it comes from a **widget you
choose** — a list, a grid, a single scrolling child, a page view — and a `Scaffold`'s body
is not one of them. Its documentation says so in as many words: *if you have a column of
widgets that should normally fit on the screen, but may overflow and would in such cases
need to scroll, consider using a list as the body of the scaffold*. Consider — the
application's call.

Here `Scaffold` wraps every body in a `Scroll` unconditionally. That is why the home page
had a scrollable at all to light a glow: nobody ever asked for one.

The gate above makes that harmless — a body that fits now behaves as a plain box, which is
what it should have been. But the wrapper is still there, still owns an offset and a
physics, and still means an application cannot say *this screen does not scroll*. Undoing it
is an API change across every screen that exists, and the keyboard-avoidance logic hanging
off that viewport has to move with it. It is its own milestone, and it is on the roadmap.

## Left

- **No nested hand-off mid-gesture.** The area is chosen once, at the threshold. The
  reference's nested scrolling passes the *remainder* outward when an inner list hits its
  end, so a flick carries on into the page behind it. Here the inner list simply stops.
- **`Axis::Both` is still one node.** It behaves as the pair now, but it is not the pair:
  the two axes share one identity, one physics and one set of edge effects.
- **The claim is by direction, not by slop.** The reference's arena is won by whichever
  recogniser crosses **its own** axis threshold first, which is not quite the same as
  comparing the two components once a combined threshold is passed. The difference shows
  on a slow drag that starts across and turns.
