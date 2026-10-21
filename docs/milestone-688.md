# Milestone 688 — Pictures decoded off the interface's thread

## Objective

Part of #221: `Image::memory` decoded in the frame that first showed it. A photograph takes tens
of milliseconds to decode on a phone, several frames, and the interface stopped for that long; a
screen of them stopped for the sum. Network pictures were decoded on the async runtime's
threads, which are the network's: a burst of decodes held up the requests behind them, and as
many ran at once as there were pictures arriving, each a burst of memory.

## What was done

- **Decoding threads** (`frus-core`): at most two, started on the first decode and alive for the
  process, first in first out. Two whatever the machine, because a decode is a burst of memory —
  a PNG is decoded at the file's size before it is brought down — and a gallery asking for forty
  pictures must not hold forty of those at once. The Web has no threads, and decodes in place
  as before; a platform that refuses to start threads does the same.
- **`decoded_at`**: `cached_at` that decodes in the background when the caller says so —
  `Fetched::Loading` until the pixels are there, counted by `images_in_flight`, so the frames
  keep coming and the frame with the picture happens. A panic in a decoder becomes a failure
  rather than a picture loading for ever.
- **`Image::memory` decodes a large picture on a thread**: until it is there the image is
  `Loading` and shows its placeholder, as a network image does. One of at most
  `QUICK_DECODE_PIXELS` (256 × 256: an icon, a logo, an avatar) is decoded at once — read from
  the header with the new `frus_image::dimensions`, without decoding — since it takes less than
  a thread would to pick it up and a blink as it came in would be the only difference.
- **Fetched pictures are decoded on the same threads**, not on the thread the fetcher answered
  on.

## Not done yet (rest of #221)

- Mipmaps, for an image drawn much smaller than its texture.
- The upload to the GPU is still in the frame; at the size shown it is a fraction of a
  millisecond, at a photograph's own size it is not.
- The measurement on the test phone: 200 large photographs scrolled, decoded memory and frame
  times.
- A picture scrolled away before its turn is still decoded; nothing cancels a queued decode.

## Verified

Unit tests: a 700 × 700 picture loading, counted in flight and arriving at its box's size; a
200 × 200 one there at once; the header read for PNG and JPEG; and the earlier tests, rewritten
to wait as the frames would (each run three times over to shake out races between them — two
were found, a trim in one test taking back a picture another had not yet held). The tests of
`frus-core`, `frus-image`, `frus-widgets`, `frus-shell` and `frus`; clippy with CI's toolchain on
Windows, wasm32 and Android; the documentation with warnings as errors.
