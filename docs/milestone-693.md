# Milestone 693 — SVG pictures and Lottie animations from an address

## Objective

#303 and #304 left `SvgPicture::network` and its Lottie twin undone: artwork served by an
application's backend — a partner's logo, a celebration animation chosen on the server — could
only be shown by fetching the bytes by hand and parsing them in the view.

## What was done

- **`SvgPicture::network(url)` and `Lottie::network(url)`**, through the fetcher images already
  use (`set_image_fetcher`, which the shell installs with the `net` feature), so an
  application has one network to configure and one to stand in for in its tests.
- **`remote.rs`**, a store of fetched documents shared by the two: the first ask starts the
  fetch and every later one is a lookup, so a view asking on every frame makes one request; a
  failure is remembered, as a fetched image's is; the bytes are parsed where the fetcher hands
  them over, off the interface's thread where there are threads; a parser that panics is a
  failure rather than a picture loading for ever. Past 256 addresses, what is not in flight is
  forgotten.
- **`state()`** on both widgets, as `Image` has: `Ready`, `Loading`, `Failed(why)`. While
  loading the widget takes the room given it and draws nothing; the application puts a
  placeholder there itself with a `match`, as it does for images.
- **The frames keep coming while they load.** `frus_widgets::images_in_flight` now adds
  pictures and animations on their way to the core's images, and the shell, which already
  asks it, keeps drawing until they land. The guard on core exports names the core function
  as wrapped.
- **A fetched animation plays from its start when it arrives.** Nothing plays while it is on
  its way — the widget declares no animated value — so when it lands the runtime meets its
  value for the first time and sets off from the start (`anim_initial`, milestone 691). A
  fetch longer than the animation does not skip it.
- `frus_core::image_fetcher()` reads the registered fetcher, for the store above.
- The tests that register a fetcher — there is one per process — share one lock now, the
  image tests' included.

## Not done yet

- No cache on disk: a picture is fetched again in the next process (an HTTP cache in the
  fetcher answers that where there is one).
- Run against a real server on a device and in a browser.

## Verified

`remote.rs`: a document fetched once, a fetch failure and a parser panic remembered, a
request in flight counted and not repeated. `SvgPicture`: the room given while loading, one
request for many asks, the picture's ratio once it lands, a 404 and a file that is not an SVG
reported. `Lottie`: nothing plays during a three-second fetch of a two-second animation, then
it plays from a quarter in after half a second, through the runtime.
