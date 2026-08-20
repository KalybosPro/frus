# Milestone 371 — An image nobody could hear

`frus_core::Role::Image` has been in the accessibility vocabulary for a long time, and
`frus-shell` maps it to the platform's. Nothing in the framework ever emitted it.

Every picture in every frus application — a photograph, a chart, a logo, an avatar in a
list of people — was silent. A reader working by ear met a gap where the content was and
was told nothing at all, not even that something was there.

That is not a missing convenience. It is the widget failing at the one thing a screen
reader needs from it, and it went unnoticed because a `Role` that nobody constructs
compiles perfectly and no test asks for it.

## `semantic_label`, and why unlabelled is not silent

`Image::semantic_label` is what a reader is told instead of the picture. Say what the
picture *is* rather than that it is a picture — the role already carries that.

An image with **no** label still announces its role. That is a decision rather than a
fallback: a reader who meets it learns something is there and can move past it, which is
strictly better than a hole in the page. Being left out entirely is the application's
call, and it says so with `exclude_from_semantics`.

## `exclude_from_semantics` wins over a label

Decoration — a divider, a texture, a shape behind a heading — should not be announced at
all. Announcing it interrupts a reader with something never meant to be read, and an empty
label would still announce the role.

When both are given, the exclusion wins. That is the reference's rule and the only
unambiguous one: the alternatives are to announce something the caller asked to hide, or
to pick by order of the builder calls, which makes the same two lines mean two things.

## Proved through the walk, not at the hook

A trait method nobody calls is a shape of bug this project has already been bitten by: a
hook that answers correctly while the walk never asks it, green unit tests over a feature
that does nothing at runtime.

So the test drives `build_ui` and reads what the walk actually collected — a labelled
image present, a decoration absent — rather than calling `semantics()` and believing it.

## `match_text_direction`, and a sign

The reference's `matchTextDirection` mirrors an image horizontally in a right-to-left
reading direction. It is **off** by default there and here, which is right: an image is a
picture rather than a run of text, and a photograph of a person does not want to be
flipped because the interface is in Arabic.

Turn it on for a picture that **points** — an arrow meaning *forward*, a speech bubble
with a tail, a hand indicating the next step. Those follow the direction the reader's eye
travels, and in right-to-left that is the other way round.

The reference describes the effect as "a scaling factor of -1 in the horizontal
direction". Here it is a **sign**. The image shader reads

```wgsl
out.uv = inst.uv.xy + vert.unit_pos * inst.uv.zw;   // unit_pos runs 0..1
```

so a negative sampled width walks the same span backwards. Start at the far edge, negate
the width, and the picture is mirrored — no transform, no layer, no second copy of the
pixels, and nothing in `frus-gpu` had to change.

The **alignment** stays physical whichever way the mirror goes, and the two are separate
questions on purpose: a portrait aligned to the top of its crop wants the top of its crop
in every language.

## Left

`repeat` (tiling) and `filter_quality` (nearest for pixel art, linear for photographs) both
want a sampler the painter chooses per draw, where there is one today —
`ClampToEdge` and `Linear`, hardcoded. They share that work, so they belong to one step
rather than two.

`center_slice` (nine-patch) needs the destination split into nine quads. `color_blend_mode`
is a real deviation worth its own look: our `tint` multiplies, where the reference defaults
to `srcIn` when a colour is given.

The asynchronous builders — `loadingBuilder`, `errorBuilder`, `frameBuilder` — have no
place in this widget as it stands, and that is a design difference rather than a gap.
`ImageHandle` is an `Arc<ImageData>`: decoded pixels the application already holds. There
is no load in flight for the widget to report on. The application owns the fetch and its
states, and a `match` over them in the view is the honest shape. Should frus grow an
asynchronous image source, this is where the question comes back.
