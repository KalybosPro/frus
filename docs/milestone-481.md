# Milestone 481 — A picture that arrives instead of appearing

Closes [#42](https://github.com/KalybosPro/frus/issues/42).

`Image` knew three states — ready, on its way, never coming — and did nothing with the
middle one. A network image appeared in a single frame, cutting hard from whatever was
there to the picture, and a list of thumbnails was a screen that flickered as it loaded.

## The placeholder, and why it is not a widget slot

`Image::placeholder` takes a colour, a `Skeleton`, or another `Image` — the three things
anybody puts there, and the three the issue names.

It is deliberately **not** an arbitrary child. `Image` carries no message type
(`impl<Msg> Widget<Msg> for Image`), which is what lets one be built once and dropped into
any tree; taking a child would make it `Image<Msg>` and cost that at every call site, to
buy a fourth case nobody has asked for. All three of the cases that exist are themselves
free of a message type, so the placeholder is **painted where it stands** rather than
mounted as a child — and the whole of what a placeholder wants is the box the picture was
going to have, which is exactly what it is given.

## The crossing, and the identity question

The issue names the hard part: the fade has to run once per **source**, not per frame and
not per rebuild, and "the thing that says *this is the same image, do not restart* needs
deciding and stating."

Stating it: **the crossing has a duration in one direction only.**

Coming in it takes its time — three quarters of a second, the reference's figure. Going
back it is instant. There is nothing left to cross *from* by then: the pixels of the
picture that was there are already gone, so tweening down would fade the placeholder **in**
over three quarters of a second with nothing else on screen, which reads as a stall rather
than as a change.

That one asymmetry answers the identity question without anything recording which picture
the timeline is about:

- **A rebuild does not restart it**, because the value lives against the widget's place in
  the tree — the framework's existing answer for every implicit animation — and the target
  does not move while the source stands still. A view is rebuilt sixty times a second here,
  so a crossing that began again each time would never finish; the picture would sit at its
  first step for ever. There is a test for it.
- **A source swapped at one place in the tree does restart it**, because the swap passes
  through *not ready* — and *not ready* resets the value to nought in a single frame. The
  journey through the middle is the record.

The alternative was a per-image note of which source a timeline belonged to, in the runtime
or on the widget. Keying the widget's identity on its source was tried on paper and is
worse: `WidgetId::keyed` replaces positional identity outright, so two thumbnails of the
same picture under one parent would silently become one widget.

**An image that was never loading never fades**, and that is the rule rather than an
omission: an embedded asset, or a network image the cache already holds, is ready on the
frame it is born and there was never a placeholder to cross over from. The mount rule of
the runtime — adopt the target, do not play to it — is what delivers that, unchanged.

**An image with no placeholder is untouched.** It claims no animated value at all, so
nothing already written acquired a timeline, a frame of transparency, or a line in the
runtime's map.

## `ImageIcon`

A picture where an icon goes: a brand mark, a flag, a glyph that is artwork rather than a
path. It answers the same `caller ?? theme ?? the grid` chain `Icon` answers, so a picture
and a path are the same size in one row of actions without either being told what the other
is — and `IconButton::image` puts one in a button at the button's own glyph size.

**It is not tinted by default**, which is the one place it parts from `Icon` and the only
real decision in it. An icon is a silhouette and takes the foreground colour. A picture
usually has colours of its own, and a brand mark flattened to one grey the first time an
application themes its icons is a brand mark nobody recognises. `ImageIcon::color` is for
the case that wants it — a monochrome glyph shipped as a bitmap, which is why the reference
honours `IconTheme.color` here at all — and the documentation says how to opt into the
ambient colour in one line for anyone who wants the other default.

The tint is multiplied into the pixels, so it colours white and grey artwork and leaves a
photograph a photograph.

## Not here

- **An error slot.** `Failed` is still a state an application reads through `Image::error`,
  and a widget for it is a different question from the one this issue asks: a placeholder
  stands in for something that is coming, and a picture that will never arrive wants a
  message rather than a stand-in. Worth its own issue if anyone wants one.
- **A blur for the placeholder image.** `ImagePlaceholder::Image` takes whatever the caller
  hands it; blurring a thumbnail is `ImageFiltered`'s job and is already reachable.

## In the demo

The icon showcase row has the demo's own logo twice: once as a picture at the size it was
given, and once as an `ImageIcon`, where it answers the ambient icon size and stands level
with the five path icons beside it without being told what they are.

There is no network image in the demo to fade, and one has not been invented for the sake
of the picture: the logo is embedded and ready on the frame it is born, which is precisely
the case that correctly does not fade.

## The pictures

`image_fading_in`, in the frame-by-frame suite, because a crossing is not a function of the
widget's arguments: the first frame is the image *not ready*, which sets the crossing at
nought, and the loop is then stepped a fifth of the fade with the picture in place. The box
is deliberately **wider than the picture** so `Contain` letterboxes it and the placeholder
shows either side — the same size, the two would sit exactly on top of each other and a
crossing would photograph as a picture in slightly the wrong colours.

`image_icon_in_a_button`: a path icon in a button, the same mark as a picture untinted, the
same mark tinted, and a bare icon — all four the same size, which is the whole claim.
