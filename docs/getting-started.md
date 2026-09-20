# Starting a frus application

frus stays **cargo-native**: no proprietary tooling, no in-house CLI.
`cargo run` / `cargo test` / `cargo apk run` are all you need.

## The smallest application

A frus app is a root component: a widget that says what it builds, and keeps what it
has to remember. The canonical example lives in
[`crates/frus-hello`](../crates/frus-hello/src/lib.rs) (~60 lines, a counter) — copy it,
or generate a fresh project with the template below.

```rust
use frus::{button, column, text, BuildContext, FrusApp, Widget};

fn counter(cx: &BuildContext) -> Box<dyn Widget> {
    let count = cx.use_state(|| 0);          // kept between rebuilds
    let add = count.clone();
    Box::new(column![
        text(format!("{}", count.get())).size(48.0),
        button("+", move || add.update(|n| *n += 1)),
    ])
}

frus::main!(FrusApp::from_fn(counter));
```

Run it on the desktop:

```sh
cargo run -p frus-hello
```

## State, in three sizes

**A hook**, for a function component: `use_state`, `use_ref`, `use_memo`, `use_effect`. They are
matched to their values by the order they are called in, so call them unconditionally and in
the same order every time.

**A `StatefulWidget`**, when the state deserves a struct, or a lifecycle. The widget is its
configuration, made again on every rebuild; the `State` is what is kept.

```rust
struct Counter;
struct CounterState { count: i32 }

impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState { CounterState { count: 0 } }
}

impl State for CounterState {
    type Widget = Counter;
    // `init_state`, `did_update_widget` and `dispose` are optional
    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        Box::new(button("+", cx.callback(|state| state.count += 1)))
    }
}
```

`cx.callback(..)` is `set_state` wrapped up as a handler; `cx.handle()` gives a `StateHandle` to
call `set_state` from anywhere — a timer, a listener. Changing the state from inside `build`
itself is refused: a build that changes what it builds from never settles.

**A controller**, when something outside a widget needs to read or write what it holds.
`TextEditingController` is a text field's text: what is typed lands in it, what the program
writes appears in the field.

```rust
let email = cx.use_text_controller("");
TextField::new("").label("Email").controller(&email);
button("Send", move || send(&email.text()));
```

A state is found again by where its widget sits, or by its key (`keyed(id, widget)`), and is
disposed at the end of the frame that no longer builds it.

## Moving between screens

Routes are named by a pattern of path; a router holds them and the stack of pages.

```rust
let router = GoRouter::new(vec![
    GoRoute::new("/", |_, _| Home.into_widget()).routes(vec![
        GoRoute::new("users/:id", |_, state| {
            UserPage { id: state.param("id").unwrap_or_default().to_string() }.into_widget()
        })
        .name("user"),
    ]),
    GoRoute::new("/settings", |_, _| Settings.into_widget()),
]);

frus::main!(FrusApp::router(router));
```

From any handler, `cx.router()` gives a handle: `go("/users/42")` makes the stack the routes
that location is made of (so there is a page to go back to), `push("/settings")` adds a page on
top, `replace`, `pop`, `go_named("user", &[("id", "42")], &[])`. A page reads what the location
said from its `GoRouterState` — `param("id")`, `query("tab")`, `extra::<T>()`.

A **redirect** sends a navigation elsewhere — to a sign-in page while nobody is signed in:
`GoRouter::new(..).redirect(move |state| ...)`, and `refresh_listenable(&auth)` asks it again
whenever `auth` changes. The page transition and the back gesture are the router's; a page that
is covered keeps its state until it is popped.

**On the web the router's location is the address.** `https://host/app/#/users/42` opens the
application at that page, with home underneath; moving between pages changes the address and
adds an entry to the browser's history; the back and forward buttons move the router; and a
reload comes back to the same page. Nothing is written for it — a `FrusApp::router(..)` does it.
`.location_strategy(LocationStrategy::Path)` writes plain paths (`/app/users/42`) instead of the
hash, which needs a server that answers every address with the page. An application that is not
a router says where it is with `Application::location` and hears about the address with
`Application::open_location`.

## Messages, when you want them

An application can instead be written as a state struct, a **pure** `update` and a `view` — the
`Application` trait, with typed messages, effects (`Command`) and subscriptions. Everything
above is built on it: `FrusApp` is an `Application` whose message is a closure. Reach for the
typed model when an application's logic is worth testing as a pure function of its messages;
[`crates/frus-transforms`](../crates/frus-transforms/src/lib.rs) is written that way.

## An application of some size

[`crates/frus-demo`](../crates/frus-demo/src/lib.rs) is a dozen screens written with everything
above, and it shows where things are kept:

- **A screen's own state is its `State`.** The filter of the task list, the value of every
  control on the settings screen, the step of the sign-up wizard: each is a field of the
  screen's state struct, changed by a handler with `cx.callback(..)` or `cx.handler(..)`.
- **What more than one screen needs is one shared object** — the tasks, how the application is
  dressed, the notifications — handed to the screens that ask for it as part of their
  configuration, and changed through methods that ask for a rebuild.
- **Where the reader is belongs to the router.** Each screen is a route, a task's own screen is
  `/task/:id`, and `router.push(..)` is the whole of navigating.
- **What is not a widget goes through `host`.** `host::app().set_theme_mode(..)` dresses the
  window, `host::focus(key)` and `host::scroll_to(key, ..)` move the focus and a list,
  `host::spawn(work, then)` runs something on another thread and uses the answer, and
  `cx.block_back(open)` lets an open menu take the back gesture before the router does.

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

### The first pixel: your launch background

Android paints the window from the moment it opens it, which is well before your
first frame exists. Whatever it paints then is what somebody sees when they tap
your icon — and left to the platform's theme, on a device in light mode, that is
**white**. A dark application opens with a full-screen white flash.

The template ships the answer as `res/values/styles.xml`:

```xml
<resources>
    <color name="launch_background">#121418</color>

    <style name="LaunchTheme" parent="@android:style/Theme.DeviceDefault.NoActionBar">
        <item name="android:windowBackground">@color/launch_background</item>
    </style>
</resources>
```

with the manifest metadata pointing at it:

```toml
[package.metadata.android]
resources = "res"

[package.metadata.android.application]
theme = "@style/LaunchTheme"
```

**Change the colour** to the one your first frame starts with. `#121418` is the
default theme's background; an application that opens light wants `#F5F6F8`, and
one that follows the system wants a second copy of the file under
`res/values-night/`.

Keep the `NoActionBar` parent. The safe area is derived from the space the system
leaves the activity, and a theme that reserves an action bar makes that space 56dp
shorter than the window — which the app then pads away as if it were a system bar,
leaving a wide empty band above its own app bar.

## Shipping

`cargo run` and `cargo apk run` build in **debug**, and a debug build keeps every
symbol it ever generated. On Android that is a ~300 MB `.so`. It is not what you
install on anyone's phone:

```sh
cargo apk build --lib --release   # the APK to install or upload
cargo build --release             # the desktop binary
```

`--lib` is not decoration. A frus crate carries **both** a library (the `cdylib`
Android loads, holding your `android_main`) and a binary (the desktop entry point).
Asked for both, the packager panics with `Bin is not compatible with Cdylib` — *after*
it has already written and signed the APK, so the file is sitting there looking
finished while the command exits non-zero.

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

Or keep both out of the manifest: `cargo-apk` reads `CARGO_APK_RELEASE_KEYSTORE` and
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD` from the environment, and they take precedence over
the table above — which is what a CI job wants. Set both or neither: a release build given
a keystore without its password stops.

### More than one ABI

Every example here, and the template, builds for one ABI: `aarch64-linux-android`, which
Android calls `arm64-v8a`. An APK carries native code only for the ABIs it was built for, and
a device whose ABI is not in it cannot install it.

| Android ABI   | Rust target               | worth shipping?                                                   |
| ------------- | ------------------------- | ----------------------------------------------------------------- |
| `arm64-v8a`   | `aarch64-linux-android`   | Always: it is what nearly every phone and tablet in use runs.     |
| `armeabi-v7a` | `armv7-linux-androideabi` | For older and entry-level devices whose system is 32-bit. An addition, never a replacement: some recent devices run 64-bit code only. |
| `x86_64`      | `x86_64-linux-android`    | For the emulator on an Intel or AMD machine, and ChromeOS on those chips. |
| `x86`         | `i686-linux-android`      | No: old 32-bit emulator images.                                   |

List them in the manifest, and add each one's Rust target once:

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]
```

```sh
rustup target add armv7-linux-androideabi   # once, for each target you list
cargo apk build --lib --release
```

`cargo-apk` compiles the library once per target and packs every result into **one** APK,
with a `lib/<abi>/` directory each, at `target/release/apk/<name>.apk`. Every device that
installs it downloads every ABI in it:

| `build_targets`             | APK    | `.so`   | `.so` in the APK |
| --------------------------- | ------ | ------- | ---------------- |
| `arm64-v8a` only            | 4.8 MB | 10.5 MB | 4.8 MB           |
| `armeabi-v7a` only          | 4.6 MB | 8.4 MB  | 4.6 MB           |
| both                        | 9.4 MB | both    | both             |

That is the counter app in release, with every font bundled, measured later than the table
above, and the counter has grown a little since. A second ABI costs as much as
the first: the library is nearly the whole APK, and two architectures' machine code share
nothing.

Signing does not change: it is the APK that is signed, once, with the release key set up
above, whatever it holds.

To build one ABI without touching the manifest, name its target, which replaces the list:

```sh
cargo apk build --lib --release --target armv7-linux-androideabi
```

What `cargo-apk` does **not** do for you:

- **Split per ABI.** Building each target on its own gives one APK per ABI, but always at
  the same path — each build overwrites the last, so copy it away — and with the **same
  version code**: `cargo-apk` derives it from the crate's version and refuses one set in
  the manifest. A store that serves a different APK per ABI needs a different version code
  for each, so these APKs are for installing on devices you know, not for publishing side
  by side.
- **Build an app bundle.** Google Play has required an Android App Bundle (`.aab`) for new
  applications since August 2021, and makes the per-device APKs from it itself: there, the
  split is the store's job. `cargo-apk` writes APKs only. Producing a bundle for a frus
  application has not been done and checked yet, so this guide does not describe it.
- **Install the Rust targets.** `rustup target add` is yours to run; the NDK already has a
  compiler for each ABI.
- **Take a version past 255.** Each part of the crate's version becomes one byte of the
  version code, so `0.1.300` is refused.

One more thing to know before a store tells you: Google Play asks applications that target
Android 15 or later to support 16 KB memory pages, and a library built as above is aligned for
4 KB pages (`llvm-readelf -l` on the `.so` shows `0x1000` on every `LOAD` segment). The
template targets SDK 34, so it is not asked yet. How to build for 16 KB has not been checked
here, so it is not described.

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
