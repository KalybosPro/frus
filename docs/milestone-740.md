# Milestone 740 — A clip path's shapes placed once

## Objective

A review of `frus-svg`'s clip paths raised a doubt: `collect_clip` placed each shape of a
`<clipPath>` by the shape's absolute transform, and also carried every group's transform down
as it walked. If the parser's absolute transform already holds those groups, a transformed
shape in a clip path would be moved, turned or scaled twice. The task was to prove it against
the reference renderer before changing anything, and to check the clip's own transform, the
clipped element's, `clipPathUnits="objectBoundingBox"` and masks for the same mistake.

## What was found

The doubt was right. The parser gives a clip path a root group whose absolute transform is the
identity, and every element inside it with a `transform` gets a group around it, its absolute
transform the product of all groups above. A shape's `abs_transform` is its group's. So
`<rect transform="translate(15 10)"/>` in a clip path was moved by 30 by 20, and a `<use>` with
a transform of its own, reaching a shape with another, applied both of them twice. A clip path holds
shapes, text and `<use>` only (a `<g>` in it is ignored by the parser, as the specification
says), so any `transform` on a clip path's shape hit this; one without, the common case, did
not, which is why the existing comparison passed.

The rest was right:

- **The clip path's own transform**, and with `objectBoundingBox` units the box folded into
  it by the parser, is applied once, before the clipped group's.
- **The clipped element's transform** is in the group's absolute transform, which is what the
  clip is mapped through.
- **Masks** build their content with `group_items`, which uses only absolute transforms and
  never accumulates, then map it through the masked group's: correct, in user units and in
  `objectBoundingBox` content units.

## What was done

- `collect_clip` walks a clip path's groups without applying their transforms again: each shape
  is placed by its `abs_transform`, then the clip's own transform, then the clipped group's.

## Considered

- **Accumulating the groups' own transforms and ignoring `abs_transform`**: the same pixels,
  and the way the reference renderer walks a clip. Using the absolute transform keeps the clip
  code in step with how every other shape in `frus-svg` is placed.

## Tests

- `frus-svg` unit test: a shape with `translate(5 2)` in a clip path that is itself translated,
  clipping a translated group, starts its outline where the three transforms put it once.
- On the GPU (`frus-test/tests/svg.rs`), four pictures compared with resvg:
  - shapes in a clip path translated and scaled by their own transform, and one reached
    through a `<use>` adding a transform to a turned shape: 9 724 pixels disagreed before the
    fix, none beyond the edges after;
  - a clip path with its own rotation, on an element and on a group in a scaled group, and a
    translated clip shape on a translated, turned element: 2 237 before;
  - `objectBoundingBox` clip units, a translated shape and a clip path translated on a turned
    element: 2 010 before;
  - a mask's content in nested translated and scaled groups, in user and in bounding-box
    content units, on a plain and a turned element: agreed before and after.
- A first version of the clip pictures put `<g>` elements in clip paths. Both renderers drew
  those clips empty, and the comparison agreed on nothing drawn; each reference picture was
  looked at to make sure it shows what the test names.

## Not done

- A `clip-path` on a shape inside a clip path is still ignored, and a clip path that is itself
  clipped is still drawn unclipped and named, as before.
