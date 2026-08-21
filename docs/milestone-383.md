# Milestone 383 — An image cache that lets go

The store behind `Image::network` was a `HashMap<String, Fetched>` that only ever grew.

Every distinct URL an application ever showed stayed in it, decoded, for the life of the
process. A `Fetched::Ready` holds an `ImageHandle`, which holds an `ImageData`, which holds
`rgba: Vec<u8>` — the whole bitmap. A 1000×1000 picture is four megabytes.

Scroll a feed of five hundred of them and the process is holding two gigabytes it will never
give back. On a phone that is not slow, it is dead: Android kills the app.

Milestone 374 shipped the store and recorded the gap in its own notes. This closes it.

## A budget and a sweep

`DEFAULT_IMAGE_CACHE_BYTES` is 100 MB, the reference's own figure for its `ImageCache`, and
it is a default — `set_image_cache_budget` takes another. `0` is a legitimate answer for a
device with almost no memory: keep nothing nobody is holding, fetch it again when it comes
back.

The sweep runs at the one moment the store grows — when a fetch lands — rather than on every
ask. That way the cost is paid once per picture that arrives, not once per frame per picture
on screen.

## Two things it must never drop

Both exclusions are the difference between a cache and a bug.

**A picture in flight.** Dropping a `Loading` entry cancels nothing; nothing in this layer
can cancel a request. It only forgets that one was made — so the next frame starts a second
one, *and* `images_in_flight` falls to zero, *and* the redraw loop that was keeping frames
coming until the picture arrived stops. The picture then lands in a store nobody will look
at again until something else happens to force a frame. The image never appears.

**A picture somebody else is holding.** That one is on screen, or in a scene about to be
drawn. Dropping it is not *unsafe* — the `Arc` keeps the pixels alive for whoever holds
them — but the next `view` finds nothing, asks again, and the image flickers through a
placeholder on its way back to exactly where it was. A store that evicts what is visible
fetches the same picture for ever.

`Arc::strong_count(handle) == 1` is how the second question is asked: one reference, and it
is the store's own.

When everything left is one or the other, the sweep **stops** rather than spinning. Over
budget is the right answer there; the alternative is dropping something being looked at.

## A counter, not a clock

*Least recently used* needs a notion of recently. It is a monotonic `u64` bumped on every
ask, not a timestamp.

`frus-core` compiles for the Web, where `Instant::now` is not a thing. A counter needs no
platform time source, cannot go backwards when a machine's clock is corrected, and *least
recently used* only ever asks which of two numbers is smaller. A wall time would answer the
same question with more machinery and one more way to be wrong.

Asking for an image counts as using it, which is what keeps a picture on a screen nobody has
scrolled away from out of the sweep's way: `view` runs every frame, so it is asked for every
frame.

## The budget is a parameter

`evict_over_budget` takes the ceiling rather than reading the static. The rule is worth
testing on its own — which of two pictures goes first, which two never go — and a function
that reached for a process-wide value would have made those tests race each other for it,
since Rust runs them in parallel in one process.

The tests drive the sweep directly rather than through a fetcher. A network image needs a
socket, a runtime and a shell; the rule being checked needs none of the three, and a test
that stood all three up to prove the older of two pictures goes first would be testing the
shell.

## Left

The `cached` store — assets, keyed by the address of their `&'static [u8]` — is untouched
and does not need this: it is bounded by what is compiled into the binary. The reference's
`ImageCache` also caps the **count** of entries, not just their size; ours does not, so a
million failed URLs would still accumulate a million short strings.
