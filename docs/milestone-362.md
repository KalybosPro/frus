# Milestone 362 — A decoration over the child

The last thing milestone 357's depth audit named on `Container`, and the one it said had
no workaround: `foregroundDecoration`. A decoration painted *over* the content rather than
behind it.

## Why it could not be written

`Container::decoration` and every part it is made of — colour, gradient, border, shadow —
go through `Widget::paint`, and the walk calls that **before** it descends into the
children. That is right for a background and useless for the cases that exist:

- an outline over a photograph, which the photograph would otherwise cover;
- a wash across a tile that is out of stock or disabled;
- a sheen, a scrim, a hairline over a card's whole surface.

The workaround was a `Stack` with a second layer the size of the first — which means
building the box twice, keeping the two in step by hand, and giving up the container's own
radius.

## The hook

`Widget::foreground(&self, theme) -> Option<BoxDecoration>`: the only point in the walk
where a widget paints after its own subtree.

It is data rather than a second `paint` on purpose. A general "paint over" would have to be
guarded by a second hook to stay cheap — the walk cannot know whether a `paint_over` body
is empty — and a decoration is what the reference means by the feature and what every use
above is. `None` costs one `Option` check per node, the same as `ink`.

Where it goes in the walk decided one thing. `walk` → `walk_scoped` → `walk_node` →
`walk_node_themed`, and only the last two have no early returns to leak past. It went in
`walk_node`, which puts it:

- **inside** the opacity group, transform and shape clip a container asks for — an opacity
  fades the foreground with everything else, which is what you want and what a `Stack`
  wrapped in `Opacity` would also do;
- **under** this node's own theme override, since it is this node's decoration;
- **inside** the repaint-boundary capture, so a cached subtree replays it.

The box is read from `rects[*index]` **before** the walk runs, because by the time the
children are done `*index` has moved past the whole subtree.

## One convenience the reference does not have

A foreground that names no radius takes the container's. `Container::radius(12)` with a
square outline over it is a mistake far more often than a request, and the reference makes
you write the radius twice. Saying `BorderRadius::ZERO` explicitly still gets a square one.

## Left on `Container`

Nothing from the audit. `transform` turned out to be a false gap — the `Transform` widget
has always existed — and `constraints` is `ConstrainedBox`.
