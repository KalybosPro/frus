# Milestone 459 — Twelve colours that promise not to move

The roadmap has carried this since the scheme was written, in red, with a reason
attached:

> Nothing here has that requirement yet, and unlike `surface_dim` they have no
> relationship to the rest of the scheme a test could hold them to.

Half of that was true. The second half was wrong, and finding out why is most of the
milestone.

## What "fixed" is for

Every accent role in a scheme is answered twice: once for a light theme and once for a
dark one. `primary` is one colour here and another there, and that is the point — a
scheme is a pair.

Three groups of four are answered **once**, and both themes give the same answer:
`primary_fixed`, `primary_fixed_dim`, `on_primary_fixed`, `on_primary_fixed_variant`,
and the same for the secondary and the tertiary. Twelve fields, none of which this
framework had.

The requirement is real and it is ordinary. A card carrying a brand colour. An
onboarding page whose illustration was drawn against one particular green. A header
somebody signed off in a design review, against a swatch, on paper. All of those need
the *emphasis* of a container and cannot accept a container's behaviour, because the
container moves when the reader turns the lights off and the illustration does not.
`primary_container` is the right emphasis for them and the wrong promise. This is the
same emphasis with the promise attached.

`*_fixed_dim` is the stronger of each pair, for what needs more weight without giving
the promise up. `on_*_fixed` is what is legible on **both** of them — one role, not
two, which is a constraint and shows up below. `on_*_fixed_variant` is the quieter
thing that can be written there: a subtitle where the other is a title.

## The relationship the roadmap said was not there

Tones 90, 80, 10 and 30 of the accent's own palette. That is all four of them, in
order, and it is what makes them testable:

- **Written schemes**: read off this crate's own `Hct`/`TonalPalette` at the hue and
  chroma of the light scheme's primary, secondary and tertiary, rather than picked by
  eye — then written **byte for byte identically** into `dark()` and `light()`.
- **Seeded schemes**: `p(90.0)`, `p(80.0)`, `p(10.0)`, `p(30.0)` in *both* branches of
  `from_seed`. The palette does not know which theme is being built, so the promise
  holds by construction there rather than by agreement.

Two things fell out of the derivation that say the numbers are the right ones: tone 90
of the tertiary lands on `rgb8(190, 234, 247)` and the hand-written light
`tertiary_container` is `rgb8(190, 234, 246)`; tone 80 of the primary is
`rgb8(111, 220, 149)`, which is *exactly* the light scheme's `inverse_primary`. The
hand-written scheme was already tonal in those places. The fixed roles did not have to
invent a palette; they had to read the one that was there.

## Two tests, where the roadmap expected none

`a_fixed_role_does_not_move_when_the_lights_go_out` walks all twelve across the written
pair and across three seeds. In the written schemes it holds two literals to each
other — the failure it is there to catch is somebody adjusting a green in `light()` and
not in `dark()`, six months from now, which is exactly the kind of thing that would
otherwise ship.

`the_dim_half_is_the_stronger_half` holds the ordering and the legibility. The
interesting assertion is this one:

```rust
for (surface, label) in [(fixed, "fixed"), (dim, "fixed_dim")] {
    let ratio = contrast(surface, on);
    assert!(ratio >= 4.5, ...);
}
```

There is **one** `on_primary_fixed` and **two** surfaces it has to sit on. A caller
that paints `primary_fixed` today and reaches for `primary_fixed_dim` tomorrow for
emphasis does not get to re-pick its text colour — that is what a single `on_` role
means — so tone 10 has to clear the bar against the darker of the pair too. The six
fixed pairs also joined the existing `from_seed` contrast sweep, so every seed the
suite tries is checked on both surfaces.

## The cost

`ColorScheme` gains twelve public fields, so a literal construction of one has to name
them. That is the third breaking change to the struct in the accelerated run —
`brightness` in milestone 453 was the first — and the same as then, the only literal
constructions in this repo are the two constructors and `lerp`; everything else uses
`..ColorScheme::light()`, which is unaffected.

`lerp` takes them like any other role. A light/dark crossing moves them nowhere,
because both ends hold the same colour — but a **palette** crossing does move them, and
there they have to travel with everything else.

**The goldens did not move**: nothing paints these yet, which is correct. The reference
leaves them for makers too.
