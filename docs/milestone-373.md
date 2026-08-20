# Milestone 373 — A response is bytes

`fetch` returned a `String`. So did `Request::send`. There was no other way out of the
HTTP layer, which meant the framework could fetch a JSON document and could not fetch a
picture — and an image over the network was blocked on a missing verb rather than on a
missing widget.

That is backwards. **A response is bytes.** Text is a conversion applied to them, and the
API had it as the only shape rather than as the common one.

## `send_bytes`, and `send` on top of it

Both transports now produce a `Vec<u8>`, and `send` is that plus `String::from_utf8`. A
body that is not valid UTF-8 is a `FetchError::Decode`, which is the answer it already
gave when the transport did the conversion itself — so nothing that used `send` behaves
differently.

- **Native**: `into_reader()` instead of `into_string()`.
- **Web**: `arrayBuffer()` instead of `text()` — the same body without the browser
  deciding it is a string.

Duplicating either implementation for a second return type was the alternative, and both
are long: the web one arms an `AbortController`, a `setTimeout` and a closure that has to
outlive the request. One transport, two endings, is the version that stays right when the
next thing changes.

## A number, because a client that reads an unbounded body is a client someone else sizes

`MAX_RESPONSE_BYTES` is 32 MiB. `ureq`'s own text reader has a limit for the same reason:
a client that reads a body of unlimited length from a server it does not control can be
made to run out of memory by that server.

32 MiB is large enough for what bytes are wanted for — a photograph, a font, a document —
and small enough to be a limit rather than a formality. Anything bigger is a *download*,
which wants streaming to a file rather than a `Vec` in memory, and that is a different
tool.

The native reader takes `MAX + 1` bytes. `take(MAX)` fills exactly `MAX` for a body at the
cap and for one far over it, and there is no way to tell them apart afterwards — so the
check would pass on a body that had already been silently cut.

## The test moves bytes

The unit tests beside `net.rs` check the builder and the constant; not one of them moves a
byte, and the thing this milestone changed is precisely what a builder test cannot see.

So there is an integration test with a one-shot HTTP server on a loopback port — port 0,
so the operating system picks one and two runs cannot collide. Four assertions over a real
socket: bytes that are not text survive intact, text still arrives as text, a non-UTF-8
body fails the text path and not the byte path, and a 404 is a `Status` rather than an
error page handed back as content.

## What it found in CI

There was a step called **"Test with optional features (net + json)"**. It ran
`cargo test -p frus --features json`.

No `net`. And `-p frus`, which is the facade — the tests live in `frus-shell`. So the HTTP
helper had **no CI coverage at all**: its 55 unit tests are behind `#[cfg(feature = "net")]`
and the routine `cargo test --workspace` compiles them out, while the step whose name
promised to cover them did not.

It ran green the whole time, because a test that is compiled out passes by being absent.
The step now runs what its name says, on both crates.

## Left

`Image::network` is what this unblocks, and it needs two more things. Bytes have to reach
`frus-widgets`, which cannot call `frus-shell` — the dependency runs the other way — so it
wants a registered fetcher, the same shape the decoder took in milestone 372. And an image
in flight is the first one with **states** worth reporting, which is where `loadingBuilder`
and `errorBuilder` finally earn their place and where `Image` has to become generic over
the message type.
