# Milestone 292 — What an application weighs

The device said it, and it was right: the demo installed at **286 MB**. That is more
than the whole of the reference framework's equivalent, on a project whose entire
claim is that it has no runtime and no garbage collector. So this milestone is not a
feature. It is finding out where 286 MB went.

## Almost all of it was symbols

`cargo apk run` and `cargo run` build in **debug**. A debug `cdylib` keeps every
symbol, every line table and every generic it ever instantiated, and nothing strips
it on the way into the APK. The `.so` was 300,000,640 bytes and the APK was
300,007,785 — the archive is the library, plus a manifest.

The same library, built `--release` under a profile that says so:

| build                                     | `.so`       | APK       |
| ----------------------------------------- | ----------- | --------- |
| `cargo apk build` (debug)                 | 300,000,640 | 286 MB    |
| `cargo apk build --release`               | 11,209,608  | **4.85 MB** |

Fifty-nine times smaller, and it is the same code. There was no size problem to
solve; there was a **missing release profile**, and a guide that never told anyone to
use one.

```toml
[profile.release]
opt-level = 3        # frames first
lto = "fat"          # cross-crate inlining, and the dead code goes out with it
codegen-units = 1    # a single unit: better optimisation, slower build
panic = "abort"      # no unwinding tables (cargo ignores this for tests)
strip = true         # symbols and debuginfo out of the shipped binary
```

`opt-level = 3` and not `"z"` on purpose. A UI framework is measured in milliseconds
per frame, and optimising for size buys a few hundred kilobytes next to the megabytes
`lto` and `strip` already take. The web profile, added in milestone 131, still trades
the other way — there the binary is *downloaded*.

The profile lives in the workspace manifest **and** in the project template, because a
generated project is its own workspace root and inherits nothing.

## What is actually left

With the symbols gone, the remaining ~4.5 MB is worth looking at honestly. The
counter app — `frus-hello`, sixty lines — and the entire widget gallery are within
half a megabyte of each other:

| release build                        | `.so`      | compressed |
| ------------------------------------ | ---------- | ---------- |
| `frus-hello` (a counter)             | 10,012,520 | 4,593,197  |
| `frus-demo` (every widget frus has)  | 11,209,608 | ~4.85 MB   |

So the floor is the framework, and the gallery costs 5% on top of it. Which makes the
floor the only interesting number — and **1.8 MB of it, about 40%, is fonts**.

## The fonts are now a choice

frus bundles its faces for a good reason, recorded in milestone 121: Android's
`fonts.xml` aliases are not readable by fontdb, so a system "sans-serif" resolves to
nothing at all and text simply does not appear. Bundling is what makes text
deterministic everywhere.

But *all* of it, always, is not a good reason. 3.4 MB of faces:

| feature          | what it bundles                  | raw     | compressed |
| ---------------- | -------------------------------- | ------- | ---------- |
| `bundled-sans`   | the sans-serif, regular and bold | 1.47 MB | 736 kB     |
| `bundled-italic` | its oblique faces                | 1.28 MB | 659 kB     |
| `bundled-mono`   | the monospace face               | 343 kB  | 205 kB     |
| `bundled-arabic` | Arabic (Naskh), regular and bold | 357 kB  | 173 kB     |

Each is a cargo feature, all on by default — the default has to stay "text works" —
forwarded through `frus-widgets`, `frus-gpu`, `frus-shell` and the facade so that an
application can reach them from its single dependency:

```toml
frus = { version = "0.1", default-features = false, features = ["bundled-sans"] }
```

The counter, sans only: **8,031,208 bytes and 3,553,189 compressed**. A megabyte off
a 4.5 MB app, for a choice the application was always entitled to make.

### Turning one off must not be a crash

This is the part that took the work. The bundled set was not an optimisation, it was
a **precondition**: cosmic-text demands an exact match on family, weight *and* style,
and a miss goes to platform fallback lists that are empty on Android — where it
panics with "no default font found". `available_weight` already existed for exactly
that reason, snapping 500 to the 400 that is really there.

So the style got the same treatment. `available_style(italic)` answers with what the
database can actually serve, and the two call sites that used to decide for
themselves — the measurement in `frus-text`, the shaping in `frus-gpu` — now ask.
Drop `bundled-italic` and italic text comes out **upright**, which is a downgrade;
before, it would have been a crash, which is not.

The families went the same way. They were three `const &str` naming DejaVu and Noto;
they are now slots that hold what is loaded, and resolve to the generic family
(`Family::SansSerif`) when nothing is. Pointing `sans-serif` at a family nobody
loaded is worse than saying nothing and letting fontdb answer.

That last fallback is honest rather than good, and the guide says so: on Android the
platform's answer is nothing at all, so an application that drops `bundled-sans`
there has to supply a face or it draws no text. The feature lets you make that
trade; it does not pretend the trade is free.

### And an application can bring its own

Which is the other half of the same change: if you can drop the bundled faces, you
must be able to supply your own.

```rust
frus::fonts::add_font(include_bytes!("../fonts/Inter-Regular.ttf").to_vec());
frus::fonts::set_default_family("Inter");
```

Registered faces go into every `FontSystem` frus builds — the measurement one here
and the renderer's — which is why they have to be registered **before** the
application runs. That is the same contract as declaring fonts in a manifest, and it
is documented as such rather than left to be discovered.

## Signing, since a release APK needs it

`cargo apk build --release` refuses to produce anything without a release keystore;
only debug builds get the debug key for free. That is correct, and it is now in the
guide with the `keytool` line and the manifest block, because "build with --release"
is useless advice if the next command fails.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **812 tests, 0
  failures**; two new, both of which hold in *either* configuration: italic text
  measures whether or not an oblique face is bundled, and a style is only ever asked
  for when the database can answer it.
- `cargo test -p frus-text` in three configurations — everything bundled, sans only,
  and nothing at all — 20 / 16 / 16 passing.
- **On a physical device** (Huawei, Android 10): the release APK installed and run,
  every screen of the gallery drawn. `panic = "abort"` and fat LTO change code
  generation, so this is not a formality.

  This was the release build **before** the font change; the device came off the wire
  before the rebuilt APK could be installed, and the re-check is outstanding. In the
  default configuration the font work is behaviour-neutral by construction — the same
  seven faces are loaded, and `available_style` reads Italic back off a database that
  has the oblique faces in it — and the two configurations that *do* differ are
  covered by the tests above. But the device has overturned a green suite four times
  in this project, so this is recorded as owed, not as done.

## What this does not do

Not addressed, and worth naming rather than leaving as a good impression:

- **The faces are not subset.** DejaVu covers a great deal of Unicode that a given
  app never draws; a subsetting step at build time would take far more off than
  feature flags do. It needs a tool in the build, which is a milestone of its own.
- **One ABI.** The demo builds `aarch64` only. An app targeting more will want a
  split rather than a fat APK.
- **No size regression test.** Nothing in CI notices if the floor doubles.
