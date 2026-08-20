# Milestone 358 — An image knows how big it is

Second of the audit's findings (milestone 357). `Image` had this shape:

```rust
Image::new(handle, width, height).fit(BoxFit::Cover)
```

Both numbers required, both at construction. Which means **you cannot show a picture
without already knowing its pixel dimensions** — and the one thing a bitmap always knows
about itself is how big it is.

## The reference's rule

`Image(image:, width:, height:)`, both optional, and `RenderImage` derives what it was not
told:

| given | box |
|---|---|
| width and height | that box |
| one of the two | the other, from the image's own ratio |
| neither | the image's own size |

All three fall out of the layout engine once the widget stops insisting: an explicit pair
is two lengths, one side is a length plus an `aspect_ratio`, and neither is the bitmap's
own size in logical pixels.

```rust
Image::new(handle)                        // its own size
Image::new(handle).width(128.0)           // 128 across, the ratio decides the rest
Image::new(handle).size(72.0, 48.0)       // exactly that
```

One caveat the tests caught and the documentation now states: a **stretched** cross axis
beats the ratio. `Flex::column()` stretches its children — that is flexbox — so an image
given only a height comes out as wide as the column. That is not a property of the image
but of the parent, and the reference does the same under
`CrossAxisAlignment.stretch`; `Column`, which centres like the reference's, gives the
ratio its say.

## `alignment`

`BoxFit::apply` centred, hard-coded, on both axes. That is the right default and the
reference's, but it is not the only answer: a portrait cropped into a banner should keep
its **top**, where a face is, rather than its middle.

`BoxFit::apply_aligned` takes the anchor, and one anchor covers both jobs — which is worth
stating because they run opposite ways. Aligning to the top means:

- the top of the **box**, for an image smaller than it (the letterbox moves up);
- the top of the **image**, for one larger (the crop keeps the low end of the UV).

Both are "keep the top", and the test asserts each direction separately.

## `opacity`

Into the tint, not into a layer. An image is a single primitive with a multiplicative
tint already, so a fade costs nothing here — where a group opacity would mean a
render-to-texture pass and the compositing rules milestone 350 spent itself on.

## Left

- **`repeat`.** `ImageRepeat` tiles a bitmap across its box. It is not a widget change but
  a sampler one — the texture would have to wrap rather than clamp — and it is the only
  item here that reaches the GPU crate.
- **`filterQuality`**, and the two builders the reference offers for an image that has not
  arrived yet or never will (`loadingBuilder`, `errorBuilder`). Both wait on image loading
  being asynchronous, which it is not here yet: an `ImageHandle` is pixels that already
  exist.
