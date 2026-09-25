# Milestone 572 — The frus logo is every application's icon, and the developer's to change

Asked for directly: the logo in `crates/frus-shell/assets/logo.png` is to be each application's
default icon, which the developer can change. It was in the repository and used nowhere — a
window had the platform's blank icon, a browser tab the browser's, an Android launcher the
robot.

## What it does

- **`AppIcon`** — `Frus` (the default), `Png(bytes)`, `None` — is a public type of `frus-shell`,
  re-exported by `frus`. `Application::icon` returns the frus logo unless overridden;
  `FrusApp::icon(AppIcon)` is how an application overrides it: `AppIcon::from_png(include_bytes!(…))`
  for its own, `AppIcon::none()` for none of the framework's.
- **Desktop.** The window's icon (`with_window_icon`), and on Windows the **taskbar** one too
  (`with_taskbar_icon`, which winit sets separately: with only the first, `WM_GETICON` answered a
  handle for the small icon and zero for the big one, and the taskbar would have kept the
  default). Wayland and macOS take a window's icon from the bundle and ignore it. An icon that is
  not a readable PNG is a line in the log and a window with the platform's icon — never a panic.
- **Web.** The tab's icon: the PNG becomes a `Blob`, its object URL the `href` of the page's
  `<link rel="icon">` (made if there is none). **The default yields to a page that declares an
  icon** — an empty `data:,`, the way a page silences the browser's request for `/favicon.ico`,
  counts as none — but an icon the application names explicitly does not yield.
- **Android.** The launcher icon is the manifest's, fixed when the package is built, so it cannot
  be code. The generated project, the demo and `frus-hello` each carry the logo as
  `res/mipmap-{m,h,xh,xxh,xxxh}dpi/ic_launcher.png` (48 to 192 px) and name it in `Cargo.toml`
  (`icon = "@mipmap/ic_launcher"`); replacing the five files changes it.

## Decisions

**256 px, not the artwork.** `logo.png` is 1024 by 1024 and 806 kB. Embedded in every binary — and
in every wasm download — it would be most of a megabyte for pixels no window draws. The shell
embeds a 256-px cut (22 kB), and the artwork stays where it was. A test holds the embedded icon
under 64 kB, so a bigger one cannot arrive unnoticed.

**A page's own icon outranks the default, not the application's.** A web application is a page
first, and its author wrote a `<link rel="icon">` on purpose; a default that overwrote it would be
the framework getting in the way. The application's explicit call is more specific than the page,
so it wins.

**`None` is not "remove the page's".** It means the framework supplies nothing: a page keeps its own,
and a window keeps the platform's.

## Found by generating a project

Adding the five PNGs to the template **broke `cargo generate`** for every new project:
`Substitution skipped, found invalid syntax` in each `ic_launcher.png`. `cargo generate` reads every
file of a template as text with `{{ … }}` in it, and a PNG's bytes contained something that looked
like it. Nothing in the repository would have said so — the template is not built by a test — and the
first person to run `cargo generate` after a release would have met it. The template's
`cargo-generate.toml` now says `exclude = ["*.png"]`, which keeps the images out of the
substitution and copies them as they are: a project generated from it has the five icons
byte-identical to the template's, and `icon = "@mipmap/ic_launcher"` in its `Cargo.toml`.

## Verification

- Tests: the embedded icon decodes to 256 by 256 with a transparent background and a drawn mark;
  each choice shows what it should; a window icon is made for a PNG and only for a PNG (garbage
  bytes give none, and no panic); the default yields to a page's icon and to nothing else; a
  `FrusApp` has the logo until it names its own.
- **On Windows**, the demo built and launched: the window answers `WM_GETICON` with a handle for
  both the small and the big icon — after `with_taskbar_icon`; before it, zero for the big one.
- **In a browser** (headless Edge): the page's `<link rel="icon">` is one element pointing at a
  `blob:` PNG that loads at 256 by 256, replacing the page's empty `data:,`; the console still holds
  only its two info lines.
- **On Android**, `aapt dump badging` on the built APK names `res/mipmap-*-v4/ic_launcher.png` at
  all five densities as the application icon; installed on the Huawei STK-L21 (Android 10), the
  system's *App info* page for *frus demo* shows the frus logo — the orange and blue wing mark — as
  the application's icon, and the app starts as before. The launcher grid itself was not
  photographed.
- The logo's bytes are compiled into desktop and web builds only (`cfg(any(desktop, web, test))`):
  an Android build has no window or tab to show them in, so it does not carry the 22 kB, and
  Android's clippy, which lints the code nobody else does, found the constant unused there.
- fmt, clippy `-D warnings` (workspace, `frus-demo --features shots`, and `aarch64-linux-android`
  for the shell and widgets), rustdoc `-D warnings`, a `wasm32` check and `cargo test --workspace`
  (2,362 tests) pass, all with `--locked`.

## Left

- The adaptive-icon form (a foreground and a background, which Android 8 and later shape to the
  launcher's mask): the icons here are plain PNGs, which the launcher draws as they are.
- macOS and iOS bundle icons, and the Linux desktop entry, belong to packaging, which frus does not
  do for those targets yet.
