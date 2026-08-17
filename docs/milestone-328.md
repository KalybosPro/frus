# Milestone 328 — Twelve per cent of what

The orange item said a slider's live rail is fainter than its disabled one, and offered a
cause: the scheme is missing `surfaceContainerHighest`, which is the role the reference
reaches for. Both halves of that turned out to be wrong, and the second one wrong in a way
worth the whole milestone.

## The role

The reference's M3 slider does not use a surface container for its inactive track. It uses
`secondaryContainer`:

```dart
Color? get inactiveTrackColor => _colors.secondaryContainer;
Color? get disabledInactiveTrackColor => _colors.onSurface.withOpacity(0.12);
```

Milestone 325 moved this rail off `outline` — right, an edge role has no business filling
a track — and onto `surface_container_high` on the strength of "a container tone". That
was half a memory. Which container matters: a *surface* container sits by definition a few
tones from the surface it lies on, which is the same neighbourhood the 12 % disabled wash
lands in; the secondary container is the role that carries a live-but-quiet fill and stays
clear of it. The rail is on it now, in both `Slider` and `RangeSlider`.

That is a correction, not a fix. It visibly helps in light — the rail is a faint blue
instead of a near-white grey, so a live slider reads differently from the disabled one
beside it — and in dark it moves the colour by two tones and changes nothing anyone would
notice. Because the cause was elsewhere.

## Twelve per cent of what

Chasing the remaining gap by arithmetic, the numbers stopped agreeing with the pictures. A
disabled fill is `on_surface` at 12 % over the surface, which in the dark scheme should
land near tone 24. Sampled out of a golden, it is **(89, 90, 93)** — tone 38.

The two models, on the dark scheme's own colours:

| | painted | model |
|---|---|---|
| linear-light blend | (89, 90, 93) | (89, 90, 93) |
| sRGB blend | | (43, 45, 49) |

Not close, and not ambiguous. The render target is `Rgba8UnormSrgb`, so the hardware
decodes the destination to linear, blends there, and re-encodes. That is the physically
correct thing to do and the ordinary wgpu pipeline — nothing is misconfigured.

But the reference's opacity tokens are a *design* language, and they assume the blend
happens in the space the colours are written in. A 12 % wash that is meant to read as a
whisper paints at roughly what 33 % would give in sRGB. Every disabled container in a dark
app is about a third of the way to `on_surface` instead of an eighth.

That is the thread behind a run of device reports, and it reframes them:

- **Milestone 324** — a disabled control that looked live. It was the live ones that were
  quiet by comparison.
- **Milestone 325** — a live outline that could not be told from a disabled one. The
  palette was genuinely off and raising it was right, cross-checked against the
  reference's own hexes. But the disabled outline it was colliding with was also brighter
  than the token says.
- **This one** — a live fill quieter than the disabled fill beside it.
- And `disabled_inputs.png` shows it without any measurement: two live sliders, two
  disabled twins, and the grey rails are plainly the louder pair.

## What is not done here

The palette change this milestone started with is **not** in it. Dark
`secondary_container` sits at tone 22 where the reference puts it at 30 — a real defect,
the only member of its family that far off, and the light scheme's is spot on. Raising it
was written, measured and reverted, because validating it means reading goldens against a
disabled fill that is itself wrong by 14 tones. Fix the blend, then the palette, then read
the pictures once.

The same reasoning killed a guard test. `a_live_container_is_never_quieter_than_a_disabled_fill`
was written, made to fail on the old palette value and pass on the new one, and then
thrown away: it modelled the disabled fill as an sRGB lerp, which is a fiction. A guard
measuring a fiction is worse than no guard, and under the true model no palette value
short of an absurd one passes — which is the proof that the palette is not where this is
fixed.

What is here instead is `crates/frus-test/tests/blending.rs`, which renders the wash and
pins the result. It asserts the current behaviour without claiming it is right, and says
so; when the blend space changes it fails loudly and points here.

## Left

- **Decide what 12 % means.** Two routes. Pre-compose the disabled tokens opaquely in sRGB
  against the surface — cheap, confined to three helpers, and arguably truer to the rule's
  own stated intent ("a disabled control flattens; it does not fade"), but it assumes the
  control sits on `surface`. Or blend in sRGB, which is a pipeline change and reaches every
  translucent thing in the framework: scrims, ink, state layers, elevation overlays. The
  first is a widget-level fix for a pipeline-level fact, and the second repaints
  everything. Neither should be chosen in the middle of a sweep.
- **Then the dark `secondary_container`**, and then the goldens, in that order.
- **Nothing else translucent has been audited.** The disabled tokens are simply where a
  device happened to look.
