# Milestone 381 — A drag with a beginning and an end

A `Slider` sent `on_change` and nothing else.

```rust
Slider::new(app.volume).on_change(Msg::Volume)
```

That message fires on every pixel of the movement — sixty times a second while the finger
is down. Which is correct: the thumb has to follow the finger, and the value has to follow
the thumb.

It is also the only thing the application ever hears. So an application that seeks a video,
writes a setting to disk, re-renders a preview or asks the network does **all of it** sixty
times a second, or does none of it, because there is no other moment to choose. There was no
way to say *do the cheap thing while it moves and the expensive one when it stops*: nothing
told anyone it had stopped.

This is not a slider problem. `Widget::draggable` is a framework-level contract and it had
exactly one hook, `on_drag(fraction)`. **Every** value-dragging widget was in the same
position.

## `on_drag_start` / `on_drag` / `on_drag_end`

Two new hooks on the trait bracket the one that was already there. The shell opens the
bracket where it creates `Drag::Widget` and closes it in `pointer_up` where it takes the
drag back.

The start goes out **before** the first value. A press on a slider's track jumps the value
immediately, and a start that arrived afterwards would hand the application a change before
it had been told a change was coming.

The end goes out even when the press never moved. A tap on a track is still a change — it
moved the thumb — and a caller deferring its work to the release would otherwise wait
forever for a release it was never told about.

## Not `on_dropped`

The trait already had a hook that fires when a drag finishes: `on_dropped(accepted)`. It
belongs to drag-and-**drop** — the family with `drag_payload`, `accepts_drag`, `on_drop`,
`drag_ghost_opacity` — and it answers a different question. That one **moves a thing** and
reports whether a target took it; this one **changes a number** and reports where it
settled.

The names do not collide, and the doc on `on_drag_start` says so out loud, because the next
person to read the trait will wonder.

## The bracket is in the same units as the stream

`Slider::on_change_start` and `on_change_end` take the **value**, not the fraction the shell
computed. A caller who set `range(0.0, 100.0)` gets a hundred from `on_change` and is owed a
hundred at each end too; a bracket that spoke in fractions inside a signature that looks
symmetrical would be a trap that type-checks.

The divisions apply at the ends for the same reason. A start or an end landing between two
stops would name a value the stream itself can never produce.

`RangeSlider` gets the pair too, in `(low, high)`. Its `on_drag` had the nearest-thumb
arithmetic inline; the three now share `interval_at`, because a start that disagreed with
the stream about which thumb the pointer was nearest would be worse than no start at all.

The shell shares its half the same way: `drag_fraction` is one function used by the start,
every move and the end. Three copies of that arithmetic would agree on every slider until
one of them did not.

## A disabled slider brackets nothing

Every one of the three checks `enabled`. `draggable` already says no, but a drag in flight
when the caller freezes the value must not land its release either — the same rule
`on_drag` has followed since it was written, applied to the two new ways in.
