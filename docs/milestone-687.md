# Milestone 687 — Pictures decoded at the size they are shown

## Objective

Part of #221: every image was decoded at the file's own size, whatever the size it was shown
at. A twelve-megapixel photograph shown as an 80×80 thumbnail cost 48 MB of pixels, and as much
again as a GPU texture; a grid of them is how an image-heavy application gets killed for memory
on a low-end phone.

## What was done

- **`DecodeSize`** (`frus-core`): the size an image is decoded at, in physical pixels. One
  side gives the other from the image's own ratio; both give the smallest size, at the image's
  ratio, that covers both — the ratio is never changed, so a picture cropped into a square box
  keeps its shape and one letterboxed into it is not short of pixels. Nothing is decoded larger
  than the file.
- **`frus_image::decode_at`**: a JPEG is decoded straight at 1/2, 1/4 or 1/8 of its size by the
  inverse DCT (through `jpeg-decoder`, which the `image` crate's decoder cannot do), then
  downsampled the rest of the way; any other format is decoded in full and downsampled once, the
  full-size pixels dropped before it returns. The filter is as wide as the reduction, and
  translucent pixels are premultiplied for it, so a transparent edge does not turn grey. A JPEG
  the scaled path cannot take (CMYK, 16-bit) goes the general way.
- **The stores are keyed by size**: `cached_at` and `fetched_at` beside `cached` and `fetched`,
  which are their full-size cases. The same asset at two sizes is two entries, each decoded
  once; the same address at two sizes is two fetches.
- **`Image` decodes at its box**: an `Image::memory` or `Image::network` given a `width` or a
  `height` is decoded at that size times the screen's pixel ratio, rounded up to 32 physical
  pixels so a box that animates is decoded a handful of times rather than every frame.
  `cache_width` and `cache_height` ask for a size outright (for an image sized by its parent);
  `full_resolution()` turns it off, for a picture shown larger than its box says (a zoom, a
  transform). `BoxFit::None`, which paints at the file's own size, is decoded at it.
- **Resolved on the first ask**: to know its box, the widget resolves its pixels the first time
  it is laid out, painted or asked its `state`, rather than when it is built. An
  `Image::network` built and dropped without any of those fetches nothing.
- **Low memory**: `trim_image_caches()` gives back every decoded image nobody is showing, in both
  stores. The shell calls it on the system's memory warning (Android's low-memory callback, iOS's
  warning), and `frus run`'s channel takes `trim-memory` to do it on demand. Browsers have no
  such event.

## Not done yet (rest of #221)

- Decoding off the UI thread: `Image::memory` still decodes in the frame that first shows it.
- Mipmaps, for an image drawn much smaller than its texture.
- The measurement the issue asks for — 200 large photographs scrolled on the test phone.

## Measured

A 4000×3000 test card (a gradient with noise, so it compresses like a photograph: 3.4 MB as a
JPEG), decoded on the Windows laptop in a release build, three runs each:

| | held after | time |
|---|---|---|
| JPEG, full size | 46 MB | 350–390 ms |
| JPEG, 256×256 box (341×256) | 0.33 MB | 160–235 ms |
| JPEG, 1080 wide | 3.3 MB | 285–410 ms |
| PNG, full size | 46 MB | 235–455 ms |
| PNG, 256×256 box | 0.33 MB | 310–380 ms |

A JPEG's time is mostly its entropy decoding, which every scale pays; what the 1/8 scale saves
is the memory on the way — 0.56 MB of decoded pixels against 36 MB — as well as after. A PNG has
to be decoded in full first, so a smaller one costs the downsampling on top (about 80 ms here,
once halvings do most of it; the filter alone over the full image took 160 ms).

## Verified

Unit tests: the size rules (cover, one side, never larger), the scaled JPEG path against a full
decode and downsample (within 16 levels), a premultiplied edge staying white, a halving being the
mean of four, the widget decoding at its box at ratios 1 and 2, an explicit size winning, the
opt-out and `BoxFit::None` keeping the file's size, an image sized after it was asked resolving
again, a fetched image arriving at its box's size, each size decoded once, and trimming keeping
what is held. The tests of `frus-core`, `frus-image`, `frus-widgets`, `frus-shell` and `frus`;
clippy on Windows, Android and wasm32; the documentation with warnings as errors.

The low-memory callback was not seen on the test phone, which was offline; `trim-memory`
through `frus run`'s channel calls the same function.
