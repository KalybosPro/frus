# Milestone 329 — A disabled control flattens; it does not fade

Milestone 328 established the fact and refused to act on it: our translucent tokens blend
in linear light, so the reference's 12 % and 38 % paint at roughly what 33 % and 60 %
would give. It left two routes and said neither should be chosen mid-sweep. The ground is
laid now, so this is the choosing.

## The two routes

**Blend in sRGB, pipeline-wide.** Switch the surface to a non-sRGB format and drop the
`srgb_to_linear` the shaders apply, so the hardware blends encoded values. In principle
small; in practice it reaches the surface format, four shaders, the glyph upload, the
image pipeline, the offscreen path and the MSAA resolve, and it moves all 83 goldens. It
is also, on the merits, what almost every UI toolkit does — the reference included.

**Resolve the disabled tokens here.** Two functions, no pipeline change.

The second one won, and not because it is smaller. This module's own documentation, written
long before the measurement, says:

> A disabled control **flattens**; it does not fade.

An opaque colour is the literal form of that sentence. The alpha was the part that
disagreed with the rule it was implementing. So `disabled_container` and `disabled_content`
now go through `over_surface(theme, opacity)`, which is `surface.lerp(on_surface, opacity)`
— the sRGB blend, resolved, handed to the GPU with nothing left to composite.

The cost is an assumption: that the control sits on `surface`. One on the darker
`background` lands about 11/255 light of a true blend. That is the difference between the
two backdrops, and it is why the reference specifies these tokens *over the surface* in the
first place.

## What the pictures say

Ten goldens moved, read as before/after pairs. The direction is opposite in the two schemes
and both were wrong:

- **Dark** — translucent light-on-dark was painting too **strong**. Every disabled control
  was louder than it should be, and in `disabled_inputs.png` the two disabled sliders were
  the brightest rails on the page, beating the live ones. They are now the quiet pair.
- **Light** — translucent dark-on-light was painting too **weak**. A disabled outlined
  button's label was barely there; it is a legible grey now, which is what 38 % of
  `on_surface` means.

`button_disabled.png` is the clearest single frame: a mid-grey button that read as
available is now a subdued fill under dim text.

## The palette, finally

With the blend resolved, milestone 328's other finding became checkable and is fixed in the
same step, which is the order 328 asked for — *fix the blend, then the palette, then read
the pictures once*. Dark `secondary_container` was at tone 22, below the disabled fill it
has to beat; it is at tone 30 now, its own hue and chroma, where the reference puts it and
where the light scheme's already sat. That is what lifts a slider's rail and a selected
segment clear of their disabled twins.

## The guard that could not be written

`a_live_container_is_never_quieter_than_a_disabled_fill` was written for 328, made to fail
on the old palette and pass on the new, and thrown away — it modelled the disabled fill as
an sRGB blend, which was a fiction while the GPU blended in linear, 14 tones adrift. It is
back, unchanged, because `over_surface` now *is* that arithmetic. The model is the painted
truth.

A fill passes on **tone** — measured as a distance from the surface, since what the eye
reads is how far a fill sits from what it lies on — or on **chroma**, because the disabled
fill is nearly grey and a coloured fill is told apart from it at any tone. Dark
`primary_container` is tone 24 against a disabled tone 24 and nobody would confuse them,
because one is green. What the rail had was neither.

## Five tests that were testing the implementation

The change broke five, all asserting on `.a`: *the container is the quieter of the two*
expressed as `container.a < content.a`. That worked only because the tokens happened to be
translucent. Each now measures distance from the surface, which is what quieter means and
what those tests meant. They are better tests than they were.

## A red CI step, found by walking past it

Editing this module's header meant running rustdoc, which is how the workspace's
**blocking** doc check turned out to be failing — and failing at `HEAD`, before any of
this. `disabled`'s docs link to `crate::transparent`, which is `pub(crate)`, so
`-D warnings` refuses it under `rustdoc::private_intra_doc_links`.

It is one line, in a file this milestone was already editing, so it is fixed here rather
than filed. The lesson is the same one milestone 325 wrote down about `cargo fmt`: the
routine check is clippy and the tests, and CI is stricter than the routine check in more
than one place. Neither `fmt` nor `cargo doc` is in it, and both are blocking.

## Left

- **Everything else translucent still blends in linear light** — scrims, ink, state layers,
  elevation overlays, and any colour an application fades itself. None of it has been
  audited, and no device report points at it. `frus-test/tests/blending.rs` pins the
  behaviour so the pipeline route stays available and deliberate.
- **The backdrop assumption.** A disabled control on something far from `surface` — a
  coloured card, a `primary_container` panel — gets a grey patch where a blend would tint.
  Nothing in the demo does this.
- **The rest of the dark secondary family** runs low: `secondary` is tone 69 where the
  reference puts it at 80. It collides with nothing, so it is an aesthetic call rather than
  a defect.
