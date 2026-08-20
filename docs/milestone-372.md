# Milestone 372 — An image an application can actually load

`Image::new` took an `ImageHandle` — decoded pixels the application already held. That is
the only constructor there was, and it meant the framework answered the second question
about images while leaving the first one entirely to the caller: *how do I show my logo?*

The demo's own answer, written because nothing better existed:

```rust
static IMG: OnceLock<ImageHandle> = OnceLock::new();
IMG.get_or_init(|| {
    frus_image::decode(include_bytes!("../assets/logo.png"))
        .map(ImageData::into_handle)
        .unwrap_or_else(|_| fallback_gradient())
}).clone()
```

Five lines, a `OnceLock`, a decoder dependency the application had to add itself, and a
fallback it had to write. **When the framework's own demo has to write the caching, the
piece is missing.** The reference says `Image.asset('logo.png')`.

## `asset!`, and why a macro is the right shape here

```rust
asset!("../assets/logo.png").width(96.0).semantic_label("frus")
```

The reference needs an asset **bundle** — a manifest, a directory convention, a loader
that finds files at run time — because its language has no way to put a file into the
program at compile time. Rust does: `include_bytes!`. So the bundle is not a missing
feature to build; it is a problem this language does not have.

That makes a macro the honest shape rather than a shortcut. `include_bytes!` takes a
literal path resolved against the file that writes it, which is exactly the ergonomics of
`Image.asset` and gives more: the bytes are in the binary, so there is no file to find at
run time, no path to get wrong on another machine, and no manifest to keep in step.

`Image::memory(bytes)` is the function underneath, carrying the reference's own name for
the same thing.

## Decoded once, keyed by where the bytes live

A view is rebuilt every frame and decoding a PNG is not free, so an image resolved in
`view` must be resolved **once**. `frus_core::cached` is that store, and it is
process-wide for the reason the font registry is: the same asset on three screens is one
image, not three.

The key is the slice's **address**, not its contents. That is exact for what this is
for — `include_bytes!` yields a `&'static [u8]` fixed for the life of the process, and two
distinct assets are two distinct statics — and it costs a pointer comparison where hashing
would cost re-reading the whole file on every frame that shows it.

The `'static` bound is what makes it sound, and it is a bound rather than a convention:
bytes that can be freed could have their address reused, and the cache would hand back the
wrong picture. Runtime bytes stay the application's business, decoded once and held as an
`ImageHandle` — which is what `Image::new` has always been for.

## A failure is a value, and it is cached too

Bytes that are not an image report it. `Image::error()` returns why, the widget paints
**nothing**, and a broken image takes no room unless it was given some — anything the
caller did say is still honoured, which keeps a page from jumping because one asset is bad.

The failure is **remembered**. A file that is not a PNG will not become one on the next
frame, and retrying every frame would turn one broken asset into a permanent cost.

There is no `errorBuilder` here, and that is deliberate for now. `Image` is not generic
over the message type, and making it so to hold a replacement widget is a change worth
making on purpose rather than in passing. `error()` lets an application `match` and build
the replacement itself, which is what the demo now does — and its fallback gradient reads
as what it is: the picture *this application* shows when the file will not decode, chosen
by the application rather than by the widget.

## The decoder is a droppable feature

`frus-widgets` did not depend on `frus-image`, and that was on purpose: `frus-core` holds
raw pixels and nothing else, so nobody inherits the decoder's dependency tree who does not
want it.

`images` keeps that true while making the common case work. It is on by default and can be
dropped, and dropping it must **not panic** — the rule the bundled fonts already follow.
Without it `Image::memory` reports a missing decoder like any other failure, which an
application that dropped the feature on purpose can see and handle. A test binary built
with `--no-default-features` is not a broken program.

## Left

`Image::network` needs bytes over HTTP, and `frus_shell::fetch` returns **text**. That is
the next step, and it brings `loadingBuilder` and `errorBuilder` with it, since an image
in flight is the first one that has states worth reporting.

`Image::file` — a path read at run time — is a smaller question with the same answer as
runtime bytes: it cannot use the address key, so it wants the cache keyed by path instead.

`repeat` and `filter_quality` still share one piece of work in `frus-gpu`: a sampler the
painter chooses per draw, where there is one hardcoded `ClampToEdge`/`Linear` today.
