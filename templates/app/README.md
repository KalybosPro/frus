# {{project-name}}

A [frus](https://github.com/KalybosPro/frus) application. The same code runs on Windows,
macOS, Linux, Android and the web.

| platform | try it | ship it |
|---|---|---|
| Desktop | `cargo run` | `cargo build --release` |
| Android | `cargo apk run --lib` | `cargo apk build --lib --release` |
| Web | see [Web](#web) | see [Web](#web) |
| Every platform at once | | push a `v*` tag: see [Releases](#releases) |

`cargo test` runs the tests. They need no window and no GPU.

## Desktop

```sh
cargo run              # debug, with the log in the console
cargo build --release  # target/release/{{project-name}}(.exe)
```

On Linux the build needs the windowing headers:

```sh
sudo apt-get install libxkbcommon-dev libwayland-dev libxkbcommon-x11-dev \
    libx11-dev libxcursor-dev libxrandr-dev libxi-dev
```

A release build on Windows opens no console window; a debug build does, and logs to it.

## Android

Once:

```sh
cargo install cargo-apk
rustup target add aarch64-linux-android
```

and an Android SDK and NDK, with `ANDROID_HOME` and `ANDROID_NDK_ROOT` set.

```sh
cargo apk run --lib                  # build, install and start on the connected device
cargo apk build --lib --release      # target/release/apk/{{project-name}}.apk
```

`--lib` is required. A debug APK is about 300 MB because it carries every symbol; the release
one is about 5 MB. A release APK has to be signed with your key: the comment on signing in
`Cargo.toml` says how.

- **The icon** is the five `res/mipmap-*/ic_launcher.png` files.
- **The colour shown while the application opens** is `launch_background` in
  `res/values/styles.xml`. Make it your first frame's background colour.
- **The package name**, `com.example.{{crate_name}}`, is `package` in `Cargo.toml`. Change it
  before your first release: a store knows the application by it.
- **The version code** comes from `version` in `Cargo.toml` (each part at most 255).

## Web

Once:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
```

Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/{{crate_name}}.wasm
python3 -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers only offer on `https` pages and on `localhost`.
`web/` is the whole site: put it on any static host, and serve the `.wasm` compressed.

## Releases

`.github/workflows/release.yml` builds everything in release when you push a tag that matches
the version in `Cargo.toml`:

```sh
git tag v0.1.0
git push origin v0.1.0
```

and publishes a GitHub release with:

| file | what it is |
|---|---|
| `{{project-name}}-0.1.0-x86_64-pc-windows-msvc.zip` | the Windows program |
| `{{project-name}}-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | the Linux program (x64) |
| `{{project-name}}-0.1.0-aarch64-apple-darwin.tar.gz` | the macOS program, Apple silicon |
| `{{project-name}}-0.1.0-x86_64-apple-darwin.tar.gz` | the macOS program, Intel |
| `{{project-name}}-0.1.0.apk` | the Android application, signed with your key |
| `{{project-name}}-0.1.0-web.zip` | the web site |

For the APK, give the repository two secrets (Settings → Secrets and variables → Actions):
`ANDROID_KEYSTORE`, your keystore as base64 (`base64 -w0 release.keystore`), and
`ANDROID_KEYSTORE_PASSWORD`. Without them the release has no APK, and the run says so.

What the workflow does not do: sign the desktop programs (Windows SmartScreen and macOS
Gatekeeper warn about an unsigned program the first time it opens), make a macOS `.app` or an
installer, or build an Android App Bundle (`.aab`) for Google Play.

`.github/workflows/ci.yml` checks every push: formatting, clippy, the tests, and a build for
the desktops, the web and Android.

## The icon

The frus logo is the icon until you change it. On the desktop and the web:

```rust
// use frus::AppIcon;
frus::main!(FrusApp::stateful(Counter).icon(AppIcon::from_png(include_bytes!("../assets/icon.png"))));
```

On Android, the five `res/mipmap-*/ic_launcher.png` files.

## Building against a checkout

To try a change to frus itself, point every frus crate at your checkout. They are versioned
together:

```toml
[patch.crates-io]
frus         = { path = "../frus/crates/frus" }
frus-core    = { path = "../frus/crates/frus-core" }
frus-gpu     = { path = "../frus/crates/frus-gpu" }
frus-image   = { path = "../frus/crates/frus-image" }
frus-l10n    = { path = "../frus/crates/frus-l10n" }
frus-layout  = { path = "../frus/crates/frus-layout" }
frus-shell   = { path = "../frus/crates/frus-shell" }
frus-test    = { path = "../frus/crates/frus-test" }
frus-text    = { path = "../frus/crates/frus-text" }
frus-widgets = { path = "../frus/crates/frus-widgets" }
```
