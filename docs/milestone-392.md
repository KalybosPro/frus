# Milestone 392 — One child was deciding how wide the screen was

An application built on this framework came back with a screenshot: a shop's banner, its
search field, its filter row and its product grid, all cut off at the same edge. Every
element on the page ended at the same x, and it was not the window's.

Everything ending at the *same* edge is the tell. A single widget too wide is one widget
overflowing. A whole page cut off in a straight line is a page that has been **laid out to
the wrong width** — and something upstream decided that width.

## What was actually wrong

`Layout::allow_shrink` exists to say a lone child is bounded by the box it was given. Its
own doc comment said so. What it did was set `flex_shrink = 1.0` and stop.

That buys nothing on its own. **A flex item's automatic minimum is its min-content size.**
The item agrees to shrink, and then refuses to go below the widest thing inside it. The
idiom that makes it real is the one every CSS layout eventually learns — `min-width: 0` —
and we had never written it.

So: one over-wide row inside a column. The column came out at that row's width. The padded
box around the column came out at the same. And `Align::Stretch`, doing exactly what it is
for, stretched every sibling to match. One segmented control four segments wide had decided
how wide the screen was, and the window clipped the result.

Reproduced away from the application, at 300 px:

| | before | after |
|---|---|---|
| an ordinary sibling | **600** | **252** |
| a child that asked for 600 | 600 | 600 |

252 is 300 less the padding either side. The over-wide child still overflows — that is the
reference's behaviour and the honest one — but it no longer drags its parent and its
siblings out with it.

## The width, and only the width

The first version bounded whichever axis the parent ran. It shortened a two-pane golden
until the list's background no longer covered its own content, which is the argument
against: **a box is bounded by the width it is given; a height is what scrolling is for.**
Squeezing a height squashes text nobody asked to squash.

The second version stamped a zero minimum over every child. That broke two tests that
existed to defend the opposite rule — a box that names a floor keeps it. So the rule reads
only `Dimension::Auto`: the *automatic* minimum, the one nobody asked for. A tight box, or
one with a floor of its own, is left alone.

## What it turned silent into loud

The change converts growth that was silent into overflow that is reported. Our own demo —
which has had an overflow guard on every screen since milestone 335 — immediately reported
eight pixels on every phone screen. The guard had been **green while asserting the bug**:
each parent grew to fit, so nothing ever measured as overflowing.

Three places in the demo were counting pixels by hand, and all three were wrong the same
way:

```rust
let content_width = (width - 88.0).clamp(240.0, 560.0);   // 24×2 + 20×2
```

24 for the body's padding, 20 for the card's — and nothing for the card's own **margin**,
which the caller never set and had no reason to know about. Eight pixels, on every phone,
every time.

The fix was not to change 88 to 96. It was to stop subtracting:

```rust
Card::new().padding(20.0).child(
    ConstrainedBox::new(column![ … ]).max_width(560.0),
)
```

The column fills the card. The only number left is the one a designer would give — a
measure no wider than 560, because a line of prose across a desktop is unreadable — and it
is a ceiling, not a measurement of the screen. The same for the card itself (it fills the
body on a phone and is capped and centred on a wide window), the text field (`Expanded`,
so it takes the room the button leaves), the progress bar and the horizontal showcase
(they stretch).

**This is the point of the whole change.** An application should not be computing the
screen's width minus a chain of paddings it has to know by heart. It gets one of them
wrong, and the failure is invisible until something reports it.

## A pin came down

Settings carried a **known** 4.5 px overflow, measured in milestone 335, pinned so it could
not grow, and left on the roadmap with a guess at its cause that milestone 345 disproved.
Under the corrected rule it measured 9, and the diagnosis took one look: a slider asking for
a fixed 220 beside a label measuring 108, in a card of 331.

```rust
Expanded::new(Slider::new(app.volume).width(220.0).on_change(Msg::SetVolume)).loose()
```

220 is now what the slider *would like*. A loose flex child — the reference's `Flexible` —
takes that or the room left, whichever is smaller. The allowance is gone from the guard:
**every screen, at a phone's width and at a desktop's, draws inside itself.**

## What this does not fix

The developer still hands `Scaffold`, `AppBar` and `Navigator` the screen's width and
height. Nothing above forces a wrong number any more, but the framework should not be
asking for the number at all. That is the next step in this series.
