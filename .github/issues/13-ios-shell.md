title: An iOS shell
labels: help wanted, design first, platform

The platform layer is one thin crate, `frus-shell`, and everything above it is
portable. Desktop and Android are done; iOS is not started. The framework's whole
architecture is a bet that this is a contained job — nobody has tested that bet.

### What it involves

- A `UIApplication` / `CAMetalLayer` host, the counterpart of the Android native
  activity.
- wgpu on Metal, which already works on macOS desktop.
- The event loop, through winit's iOS support.
- Touch, then IME, then insets (the notch and the home indicator), then the lifecycle.

### Read first

- [ARCHITECTURE.md](../../blob/master/ARCHITECTURE.md) — what belongs in the shell and
  what must never.
- `crates/frus-shell/src/app.rs` — the Android path is the closest model.
- `docs/milestone-288.md` — how insets are handled, and one way that goes wrong.

### Please open a discussion first

This is tagged **design first** deliberately. It is a large piece of work, it needs a
Mac to do at all, and the shape wants agreeing before anyone spends a weekend on it.
Say hello in the issue before starting so nobody duplicates the work.
