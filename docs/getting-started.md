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

The template's manifest metadata asks for a theme **without** a title bar:

```toml
[package.metadata.android.application]
theme = "@android:style/Theme.DeviceDefault.NoActionBar"
```

Keep it. The safe area is derived from the space the system leaves the activity,
and a theme that reserves an action bar makes that space 56dp shorter than the
window — which the app then pads away as if it were a system bar, leaving a wide
empty band above its own app bar.

## Shipping

`cargo run` and `cargo apk run` build in **debug**, and a debug build keeps every
symbol it ever generated. On Android that is a ~300 MB `.so`. It is not what you
install on anyone's phone:

```sh
cargo apk build --release      # the APK to install or upload
cargo build --release          # the desktop binary
```

| what you built                       | `.so`   | in the APK |
| ------------------------------------ | ------- | ---------- |
| `cargo apk build` (debug)            | 300 MB  | 286 MB     |
| `cargo apk build --release`          | 10.0 MB | 4.4 MB     |
| …with only the sans face bundled     | 8.0 MB  | 3.5 MB     |

Those are the numbers for a counter app. The whole widget gallery — every screen in
`frus-demo` — is a 4.9 MB APK, because almost all of it is the framework and the
framework is the same size either way.

The generated project already carries the `[profile.release]` that gets you there:
link-time optimisation, one codegen unit, no unwinding tables, and the symbols
stripped. Keep it.

### Signing

A release APK has to be signed with a key you own — the debug key is only wired up
for debug builds. Generate one, keep it out of version control, and name it in the
manifest:

```sh
keytool -genkey -v -keystore release.keystore -alias mykey \
        -keyalg RSA -keysize 2048 -validity 10000
```

```toml
[package.metadata.android.signing.release]
path = "/absolute/path/to/release.keystore"
keystore_password = "…"
```

### Fonts, and what they weigh

frus bundles its own faces so text renders identically everywhere — Android has no
system font list an application can resolve — and they are the single biggest thing
it puts in your binary: about 3.4 MB, ~1.8 MB once the APK compresses them.

Each group is a feature, all on by default. An application that ships its own faces,
or simply never draws italics or Arabic, can turn off what it does not need:

```toml
frus = { version = "0.1", default-features = false, features = ["bundled-sans"] }
```

| feature          | what it bundles                  | cost   |
| ---------------- | -------------------------------- | ------ |
| `bundled-sans`   | the sans-serif, regular and bold | 1.5 MB |
| `bundled-italic` | its oblique faces                | 1.3 MB |
| `bundled-mono`   | the monospace face               | 343 kB |
| `bundled-arabic` | Arabic (Naskh), regular and bold | 357 kB |

Turning one off is never a crash: italic text renders upright, a script with no face
falls back to the sans, and with no sans at all frus asks the platform for its own.

Be careful with that last one on **Android**, though, where the platform's answer is
nothing: `fonts.xml` is not a font list an application can resolve. Dropping
`bundled-sans` there means you must supply a face yourself, or draw no text at all.

To ship your own face instead, register it before the application starts:

```rust
frus::fonts::add_font(include_bytes!("../fonts/Inter-Regular.ttf").to_vec());
frus::fonts::set_default_family("Inter");
```

## Testing

Because `update` is pure, the logic is testable **without a GPU or a window**:

```sh
cargo test
```

For rendering tests (snapshots/goldens), see
[`frus-test`](../crates/frus-test/src/lib.rs).
