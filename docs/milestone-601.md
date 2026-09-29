# Milestone 601 — `CustomMultiChildLayout`

Issue #38 asked for an escape hatch: a layout flexbox, a stack and a grid cannot express, such
as a circle of buttons, a fan of cards, a bubble whose tail points at its neighbour, or nodes
placed by a solver. It is also the multi-child form that #52 left for last.

## The choice #38 asked for

The issue offered two ways:

1. **Solve the general problem**: children that can be measured, then a layout function on
   top.
2. **A function that places but does not measure**: cheap, and unable to size anything to its
   content.

This is the first, and it did not need a new pass. Milestones 592 to 600 laid children out
**apart**, in a layout of their own, from the walk. A stack's layers have always been done that
way. The function runs in the walk, where the layout's own box is known, and for each child it
asks for a separate layout under constraints it chooses. That is a real measurement: a paragraph
wraps, a list takes what it is allowed.

The one thing this rules out is a layout whose own size depends on its children. The function
runs after the layout's box is decided. **The reference rules the same thing out**: its
delegate's `getSize` sees only the constraints.

## What it does

```rust
CustomMultiChildLayout::new(|size, layout| {
    let title = layout.layout_child("title", ChildConstraints::loose(size));
    layout.position_child("title", Point::new(0.0, 0.0));
    layout.layout_child("caption", ChildConstraints::free());
    layout.position_child("caption", Point::new(title.height, title.height));
})
.child("title", Text::new("Seat map"))
.child("caption", Text::new("Row 12"))
```

- **Children are named by keys**, which can be anything hashable: strings, numbers or an
  enum. The reference wraps each child in a `LayoutId` for the same purpose.
- **`layout_child(key, constraints)`** lays the child out and returns its size. Each axis is
  `Free`, `AtMost(v)` or `Exactly(v)` (`ChildConstraints::free`, `loose`, `tight`).
- **`position_child(key, point)`** puts the child's top left corner at a point measured from the
  layout's top left.
- **`has_child(key)`** is for layouts whose children come and go.
- **Its own size** is the room it is given, or its own `width` and `height`.
- A child is painted in the order it was given. A child that is never laid out is not shown.
  A child that is laid out but never placed sits in the corner. Nothing is cut at the layout's
  edge.
- Placed children are ordinary: they take taps, focus and semantics where they are.
- Asking about a key that was never given **panics**, as the reference's assertion does. The
  function is asking about something that is not there, and a size made up for it would get
  placed.

## How

- A new hook, `Widget::custom_layout`, gives the function and the keys. Wrappers forward it,
  as they do the stack question.
- In the layout, the widget is a **leaf**, as a stack is. Its children are not in its taffy
  tree, and its fingerprint is its own style.
- In the walk, the function is handed the box. `layout_child` lays the child out through the
  layout cache under its own identity, as a stack's layer is. After the function has run, each
  child it laid out is walked at its place.
- `Exactly` **forces** the extent when no axis is `AtMost`, as a pinned stack layer is. Beside an
  `AtMost` axis, the extent is handed instead, and a child with a size of its own keeps it.
  `AtMost` uses milestone 598's rule: a paragraph wraps at it.

## Differences from the reference

- **No minimums** other than `Exactly`. The reference's constraints have a minimum and a
  maximum on each axis. Here an axis is free, bounded or exact, which is what the walk's
  layouts can express.
- **No `CustomSingleChildLayout`** as its own widget. A layout with one child is the same thing.
- **`Flow`**, which moves children at paint time, is not here either.
- **Keys are hashed.** Two keys that hash alike name the same child. With a 64-bit hash this is a
  theoretical risk.

## Verification

- A red block placed at (10, 10), and a blue one placed under it and against the right edge
  from their measured sizes.
- A child never laid out is not painted. One laid out and not placed is at the corner.
- In a 160-px column, a layout 50 px tall with a 300-px child inside: what follows starts at
  50, and the function was handed 160 × 50.
- `Exactly(70 × 30)` gives a sizeless container 70 × 30. `AtMost(120)` wraps a paragraph within
  120.
- A button placed at (100, 50) takes a tap there and not at the corner.
- Through a key, the children are still placed.
- An unknown key panics with the message.
- Mutations, each failing a test:
  - the place ignored;
  - `Exactly` taken as `AtMost`;
  - `AtMost` taken as free;
  - no request for the room;
  - the children laid out in the layout's own node;
  - a key's wrapper not passing the question on;
  - a child never laid out shown anyway.
