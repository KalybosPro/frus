title: Document multi-ABI Android packaging
labels: good first issue, documentation, android

Every example builds `aarch64-linux-android` alone. A real application shipping to a
store wants several ABIs, and either a split APK per ABI or an app bundle — otherwise
every phone downloads every architecture.

Nothing in the documentation says so, which means the first person to ship an
application with frus finds out from the Play Console.

### What to do

A section in [`docs/getting-started.md`](../../blob/master/docs/getting-started.md), or
its own short page, covering:

- which ABIs are worth shipping in 2026 and why;
- how to get there with `cargo-apk` (`build_targets` in
  `[package.metadata.android]`), and what it does *not* do for you;
- what that costs in size, measured rather than guessed;
- the signing step, which the *Shipping* section already covers for one ABI.

You will need an Android SDK and NDK to check the commands actually work. Please do
check them — a documented command that does not run is worse than no command.
