# Milestone 348 — The band learns to write

Milestone 345 painted the reference's striped band across every box whose children ran
past it, and left one thing out:

> **No label.** The reference writes `RIGHT OVERFLOWED BY 5.0 PIXELS` across the band,
> rotated for the vertical edges. The console says the same thing here, which is enough
> until the band is being read on a device — where the console is not.

That last clause is the whole milestone. A striped edge says *something is too big*. It
does not say which edge, and it does not say by how much — and the difference between a
forgotten padding and a missing wrap is exactly the number. The console has both since
milestone 335, but the console is on the developer's machine, and a photograph of a phone
is what half the bug reports in the world are made of.

So the band writes it: `RIGHT OVERFLOWED BY 86 PIXELS`. The reference's words, letter for
letter, so that searching for them finds the same answers people have already written
down. Its metrics too — 7.5 px, the heaviest weight available, a dark red on an opaque
white plate, one pixel off the outer edge, centred on the band and turned a quarter turn
on the vertical ones. A label that has to stay readable over black-and-yellow stripes
cannot afford to be subtle.

The precision rule is the reference's as well: whole pixels past ten, one decimal past
one, three significant figures below that. It reads as fussiness until the number is
`0.5` and you have to decide whether you are looking at a layout bug or at a rounding
error — which is, word for word, the question milestone 345 got wrong four times before
it read the unrounded layout.

## What the rotation cost

A quarter turn is not something a glyph does. It is something a **group** does — the
plate and the sentence turn together, and neither has to know it is turning — and the
scene had no way to say so: `Scene::layer` composites a group at an opacity and
`Scene::masked` through a shader, but a transform could only be reached by hand-building
a `Primitive::Layer`, which is what the widget walk does for a rotated widget.

`Scene::transformed` is that third one, and it is four lines longer than `layer`.

## The thing it taught about layers

The first attempt drew the label where it goes, rotated about its anchor, and lost its
last word: `RIGHT OVERFLOWED BY 86 PIX`.

A layer is **rendered flat into a window-sized texture** and then composited through the
transform — that is what makes one pass transform a whole subtree. So the flat pass is
clipped by the window, and a vertical label belongs to a box near the right edge: laid
out flat it ran off the texture, and the rotation that would have brought it back inside
happened afterwards, to pixels that no longer existed.

The fix is to stop thinking of the transform as *turning something already in place*. The
group is painted at the **origin**, where it certainly fits, and the transform is the one
that carries the origin to where the label goes: shift it half its width left, turn it,
land it on the anchor. Rotating about a pivot is the special case of that where the thing
happens to be drawn at the pivot already.

Anything that transforms a group in this framework has the same constraint, and this is
the first time it mattered.

## The golden

The fixture now shows both orientations, because the vertical one is the half that can go
wrong — and it shows one more thing that is faithful rather than pretty: on a short box
the bottom label is taller than the band and covers it. The reference does exactly that,
for the same reason (the band is a tenth of the box; the label is a fixed 7.5 px), and a
golden that hid it would be hiding the reference.

## Left

- **The band is not clipped to its box.** A vertical label is usually longer than the box
  is tall, so it is written over whatever is beside it. The reference does the same, and
  it is the right call — half a sentence is worse — but it means two overflowing boxes
  side by side produce two labels over each other.
- **The reference asks for an 800 weight** and the heaviest this framework has is bold
  (700). It is `FontWeight`'s ceiling, not this label's.
