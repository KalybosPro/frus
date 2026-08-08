# Milestone 276 — Clearing the ground for iOS: named platform `cfg`s

## The goal

This milestone **does not build** the iOS shell. It removes the mine laid under its feet.

Until now, "desktop" was written in `frus-shell` **by negation**:

```rust
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
```

While only three platforms exist, that is accurate. The moment iOS is added, **iOS falls
into that branch** and inherits the `arboard` clipboard, `env_logger` and AccessKit — three
things with no UIKit backend. The code might well have compiled, and it would have been
wrong.

The flaw is structural: the phrasing encodes "the others" instead of encoding "desktop".

## Alternatives weighed

1. **Extend the list at every site** — `not(any(target_os = "android", target_os = "ios",
   target_arch = "wasm32"))`. Mechanical, no new machinery. Rejected: 66 sites, a list that
   grows with every platform, and **every site is an opportunity to silently forget one**.
   The next port (native macOS, embedded) would replay the same bug.
2. **Cargo features** (`--features desktop`). Rejected: the platform is not a user choice,
   and a forgotten feature would give a binary with no clipboard rather than a compile
   error.
3. **`build.rs` + `cfg_aliases`** — chosen. It is exactly what **winit and wgpu** do, our
   two foundation dependencies. The crate is **already in the tree** (winit uses it): zero
   extra dependency downloaded.

## The decision

`crates/frus-shell/build.rs` names four platforms:

```rust
web:     { target_arch = "wasm32" },
android: { target_os = "android" },
ios:     { target_os = "ios" },
desktop: { not(any(web, android, ios)) },
```

Adding a target now touches **that file alone**.

> `desktop` is written by reusing the preceding aliases rather than with the full
> `target_os`/`target_arch` list: in the latter form, `cfg_aliases` hits its recursion
> ceiling (`recursion limit reached while expanding $crate::cfg_aliases!`).

### Two boundaries the aliases do not cross

These are the two traps in the scheme, both documented in the code:

- **The crate boundary.** The aliases are `--cfg` flags passed to `frus-shell` alone. The
  body of the `main!` macro expands in the **application's** crate, where they do not exist:
  a `#[cfg(desktop)]` there would be *always false* and the app would have **no entry point
  at all**. So the macro keeps its explicit `target_os` / `target_arch` predicates.
- **Cargo.** `[target.'cfg(…)'.dependencies]` tables are evaluated by Cargo, which knows
  nothing of these aliases. The desktop dependency table keeps the literal list — with
  `target_os = "ios"` added by hand, and it is that table which keeps `arboard` and
  `accesskit_winit` off iOS.

### The underlying fix: `not(desktop)`, not `any(android, web)`

The code carried implementation/stub pairs of this shape:

```rust
#[cfg(desktop)]            pub struct Clipboard(Option<arboard::Clipboard>);
#[cfg(any(android, web))]  pub struct Clipboard;                    // ← iOS: neither one
```

On iOS, **neither** branch applies: the type does not exist. Those three sites move to
`#[cfg(not(desktop))]`, which is the real intent ("everything that is not desktop") and
stays correct for any future platform.

### The iOS entry point

`run()` now has two bodies: `#[cfg(desktop)]` (unchanged) and `#[cfg(ios)]`, the latter with
no `env_logger` (stderr is not readable on device), no clipboard and no AccessKit. **The
`main!` macro did not have to change**: its `not(any(android, wasm32))` predicate already
covers iOS, and winit takes care of `UIApplicationMain` itself.

## Verification

The refactor is **a no-op** on the three platforms in service — that is its safety property,
and it is verifiable here:

- `cargo build --workspace --all-targets` — OK;
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK;
- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **613 tests, 0 failures**.

**iOS itself cannot be verified from the development machine** (Windows: no Apple SDK and no
linker, so no credible cross-compilation). The new `ios` CI job (`macos-latest`) settled it —
it exists precisely to tell the truth about what was written blind. **Verdict: both targets
compile.**

```
aarch64-apple-ios      →  Finished `dev` profile in 1m 33s
aarch64-apple-ios-sim  →  Finished `dev` profile in 16.17s
```

The log confirms the arrangement beyond a bare "it compiles": `objc2-ui-kit` and `metal` are
in the tree (winit's UIKit backend, wgpu's Metal output), and **`arboard`, `accesskit_winit`
and `env_logger` are not** — the `Cargo.toml` exclusion held, and the `run()` under
`#[cfg(ios)]` type-checks.

The job was therefore made **blocking**: what has just been gained must not regress
silently. It proves that iOS *compiles*, not that iOS *runs*.

## What's left

All of the iOS shell, in fact. This milestone only made adding it possible without breaking
the rest:

- the UIKit lifecycle (`applicationDidEnterBackground`…) and the matching `Lifecycle`;
- **safe-area insets** (notch, home indicator) → `WindowInsets`, as on Android;
- IME and the soft keyboard;
- logging to `os_log` rather than stderr;
- UIKit accessibility;
- `.ipa` packaging — the one place where `cargo` has no ready-made answer.
