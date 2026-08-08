# Starting a frus application

frus stays **cargo-native**: no proprietary tooling, no in-house CLI.
`cargo run` / `cargo test` / `cargo apk run` are all you need.

## The smallest application

A frus app is the Elm model in full: a state struct, a **pure** `update`, and a
`view`. The canonical example lives in
[`crates/frus-hello`](../crates/frus-hello/src/lib.rs) (~60 lines, a counter) —
copy it, or generate a fresh project with the template below.

```rust
impl Application for Counter {
    type Message = Msg;
    fn update(&mut self, m: Msg) -> Command<Msg> {
        match m { Msg::Increment => self.count += 1, Msg::Decrement => self.count -= 1 }
        Command::none()
    }
    fn view(&self, theme: &Theme, w: f32, h: f32) -> Box<dyn Widget<Msg>> { /* … */ }
}
```

Run it on the desktop:

```sh
cargo run -p frus-hello
```

## Generating a new project (`cargo generate`)

The [`templates/app`](../templates/app) template produces a frus project that
runs as-is (desktop + Android).

```sh
cargo install cargo-generate          # once
cargo generate --path templates/app --name my-app
cd my-app
cargo run
```

`cargo generate` asks for the **path to your frus checkout** (the one that
contains `crates/`) — that is where the generated `Cargo.toml` points its
dependencies, for as long as frus is unpublished. Once it is on crates.io those
dependencies become plain `frus-shell = "0.1"`.

## Android

The template includes the `android_main` entry point and the `cargo-apk`
metadata. From the generated project:

```sh
cargo install cargo-apk              # once
cargo apk run                        # build + install + launch on the device
```

Android prerequisites: SDK + NDK installed, `ANDROID_HOME`/`ANDROID_NDK_ROOT`
set, and a device connected (`adb devices`).

## Testing

Because `update` is pure, the logic is testable **without a GPU or a window**:

```sh
cargo test
```

For rendering tests (snapshots/goldens), see
[`frus-test`](../crates/frus-test/src/lib.rs).
