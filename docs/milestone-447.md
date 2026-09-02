# Milestone 447 — A value that depends on what a widget is doing

The reference threads one idea through every Material component: **a property is not a
value, it is a function of the states the widget is in**. A button's foreground is one
colour at rest, another under a pointer, another while held, another when it cannot be
pressed at all — and the API takes all four in one argument.

This framework had no way to say that, and the notes have recorded wanting it since
milestone 322. What it had was `Theme::state_layer`: one lerp from a ground towards an
ink at three fixed opacities. That is the *right answer for a state layer* and no answer
at all for a caller who wants a particular colour in a particular state.

## The four pieces

`WidgetState` (`widget_state.dart:168`) — the reference's eight, unchanged: hovered,
focused, pressed, dragged, selected, scrolled-under, disabled, error.

`WidgetStates` — the reference passes a `Set<WidgetState>`. Eight states fit in eight
bits, so this is a **byte**: `Copy`, no allocation in a paint walk that runs every frame,
and `Hash`, which means the set can take part in the rebuild hash. A `HashSet` could do
neither cheaply.

`StateFilter` (`widget_state.dart:27`) — what a set has to look like for one entry to
answer. The reference spells the combinators `&`, `|` and `~`; this spells them `&`, `|`
and `!`, which is what Rust has:

```rust
let held_but_not_broken = StateFilter::from(WidgetState::Pressed)
    & !StateFilter::from(WidgetState::Error);
```

`WidgetStateProperty<T>` (`widget_state.dart:821`) — entries tried in order, **first
match wins**, exactly as `WidgetStateMapper` does (`:1009`). Order therefore matters: a
pressed widget is nearly always hovered as well, so the narrow entry goes first.

```rust
let ink = WidgetStateProperty::new()
    .when(WidgetState::Pressed, red)
    .when(WidgetState::Hovered, pink)
    .otherwise(Color::TRANSPARENT);
```

## Two deliberate departures

**No closure form.** The reference's `resolveWith` takes a callback. A map is `Clone`,
`Debug` and comparable; a boxed closure is none of the three, and this is a widget tree
that gets rebuilt and diffed every frame. The reference itself documents the map as the
form to reach for and keeps `resolveWith` as a convenience with a `TODO` about
deprecating the neighbouring one.

**Resolving to `None` is an answer.** A property that matches nothing says *nothing*, and
the widget or the theme answers instead — which is what the reference's nullable
properties do. Naming one state is not a way of silencing the rest.

## A step, not a fade

`Status::states()` reads the **flags**, not the fades. A property is a step between
values.

That is what the reference does too: where it wants the change to be gradual it animates
*between two resolved values* rather than resolving a fraction — its scrollbar lerps its
idle colour towards its hovered one with an animation controller of its own
(`scrollbar.dart:262`), having resolved both. So a property gives the endpoints and the
widget decides. `Theme::state_layer` stays the answer where the whole point is the fade.

`Status` answers for **three** of the eight — hovered, focused, pressed — because three
are all it honestly knows. Selection, disablement, error, dragging and scrolled-under are
the **widget's own** to add with `WidgetStates::set`: nothing outside a checkbox knows
whether it is ticked, and a status that guessed would be wrong for every widget that
never selects anything.

## Its first user

`NavigationRail` and `BottomBar` destinations take an `overlay_color`
(`navigation_bar.dart:232`) — the recorded 🔴 this milestone was built to answer.

The reference's overlay is a translucent colour painted over the ground. Resolved
opaquely here, that means its **alpha is how far** the ground moves and its **colour is
where** it moves to — the same arithmetic `state_layer` does, with the caller's numbers
instead of Material's. A state the property says nothing about still gets the state layer.

## The tests

Four on the primitive: the byte-sized set (including that it *is* a byte), first-match
ordering, answering nothing, and the three operators. Two on the destination: that the
caller's colour is used at the caller's strength and is not the framework's own, and that
an unnamed state still gets the framework's layer. Both of the latter fail with the one
rule restored.

## What this unblocks

Recorded, not done: `labelTextStyle` and `iconTheme` on a destination are the same shape
(`navigation_bar.dart:245`), as are a button's foreground, background and side. And the
theme structs cannot hold one yet — they derive `Copy`, and a property owns a `Vec`. That
is a question about the theme's storage, not about this.
