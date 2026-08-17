# Milestone 330 — Every glyph in the framework was too dark

Found on a device, checking milestone 329's work rather than looking for anything. The
Settings screen has a switch that goes unavailable while notifications are off, and its
label had become unreadable: "Weekdays only", painted at `(37, 39, 43)` on a card at
`(36, 40, 48)`. Dark on dark, five values of blue apart, and only legible in a screenshot
at 3× with the brightness turned up.

## Not where it looked

The first suspicion was 329 — a disabled token resolved wrong. It was not. Driving
`Application::view` and reading the scene primitive gave the label's colour as
`(106, 109, 114)`: exactly `disabled_content`, exactly right.

So the tree was right and the **renderer** was painting something else. And `106 → 37` is
not a random corruption: 37/255 is the *linearised* value of 106/255. One sRGB→linear
conversion too many.

Checking the live label on the same screen: specified `on_surface` `(230, 232, 236)`,
painted `(202, 206, 214)`. `to_linear(230) = 202`. The same extra conversion, on every
glyph — invisible on a bright colour, fatal on a dim one.

## Not the device either

The goldens say the same thing. Across `disabled_inputs.png` and `disabled_controls.png`,
**zero** pixels at `on_surface` and 258 at its linearised value. This was never about
hardware; it is how frus has always drawn text.

## The arithmetic

`text.rs` converted each glyph colour to linear before handing it to glyphon, with a
comment explaining why:

> The target is sRGB, so we send linear values — as the quads do — to avoid encoding
> twice, which washes the text out.

The reasoning is right for the **quads**: `quad.wgsl` calls `srgb_to_linear` itself, and
the sRGB target re-encodes on write. It is wrong for text, because glyphon's own shader
already does that conversion. So a glyph went:

| step | value |
|---|---|
| specified | 0.902 |
| our `to_linear` | 0.792 |
| glyphon's shader | 0.597 |
| sRGB target re-encodes | 0.794 → **202** |

Two decodes, one encode, one left over. The comment had it exactly backwards — the danger
was never encoding twice, it was *decoding* twice.

The fix is to pass the colour through untouched.

## Verifying a change that moves everything

110 goldens move — every picture with text in it. Reading 110 pairs by eye is not a method,
so the uniformity is what gets checked, machine-wide:

- **228 477 pixels changed, and not one got darker.** That is the exact invariant of
  removing a linearisation: `to_linear(v) ≤ v` for every `v`, so undoing it can only
  raise a channel. A single darker pixel anywhere would mean something else moved too.
- **13.8 % match the un-linearisation exactly** — the glyph cores. The rest are
  antialiased edges, which blend the corrected colour with their background and so land
  in between, as they must.

Then a handful read as pictures. `validated_signup_form.png` is the one that shows the
bug's shape best: the field's error border and the error message under it are the same
`error` token, and *before* the fix they were visibly different reds — the border correct,
the text dark. They match now. One token, two renderings, which is what a colour-space slip
between the quad path and the text path looks like.

The direction flips with the scheme and both were wrong. In dark, light text was too dim.
In light, dark text was crushed toward black — which made disabled labels *more* legible
than the reference intends, so they are fainter now, at the value the token actually names.

## Left

- **Nothing checks a painted colour against its token.** Three milestones running have been
  colours that did not survive the trip to the screen — 328's blend space, 329's tokens,
  and this. A test that renders a known colour and asserts the pixel would have caught all
  three, and `frus-test` already has the harness for it. `blending.rs` is one instance of
  the idea, written for one case.
- **`image.wgsl` tints with `srgb_to_linear(in.tint.rgb)`** against a texture sample that is
  already linear. Not audited here, and no report points at it.
- **The overall weight of the UI changed**, since this is every label in every app. On the
  device the Settings screen now reads correctly — the disabled label at exactly
  `(106, 109, 114)` and the live one at exactly `(230, 232, 236)`, both to the byte — but
  a framework-wide brightening deserves a second pair of eyes on whether anything now reads
  *too* loud.
