# Milestone 503 — The system bars, told what the screen under them wants

Answers [#46](https://github.com/KalybosPro/frus/issues/46).

An application could not set the colour of the Android status bar or navigation bar, nor
say whether their icons should be dark or light. Nothing in the shell touched them, so they
kept whatever the launch theme left — and the launch theme is the *platform's*, following
the phone's night setting rather than the application's theme. A light screen could end up
under light icons, the clock and the battery gone, in a way no golden can catch because the
bar is not ours to render.

## Who decides: the part of the screen underneath

Not the application, once, at start-up. A screen with a dark photograph behind the status
bar wants light icons, and the settings screen next to it wants dark ones. So, as in the
reference, a subtree says what it wants:

```rust
AnnotatedRegion::new(SystemUiOverlayStyle::for_background(photo), header)
```

and each frame the shell asks which regions lie **against each bar** — the first row of
content under the status bar, the last one above the navigation bar. The region drawn last
answers first, **field by field**, so an inner region that only states its icons still
takes its colour from the outer one. What no region states comes from the theme: the bars
in its background colour, with icons that can be read on it.

Two rules make that last half hold up:

- **A colour stated without icons gets icons read on that colour**, not the theme's. A
  region that paints the status bar black under a light theme would otherwise get dark
  icons on black — the bug again, one level down.
- **The contrast is the reference's estimate**, a relative luminance of about a third
  rather than a half, because it weighs contrast against white and against black the way
  the accessibility guidelines do.

The icons are named by **their own** brightness (`Brightness::Dark` is dark icons). The
platform's word for the same thing is a "light status bar", which is the opposite word;
the translation happens once, at the JNI boundary.

## How it is found each frame

The regions are a registry of the paint walk, like the semantics and the ink — each one
kept with its box. That is the reference's arrangement too (its annotated regions are
found by a hit test on the layer tree at the bar's position) and it means a region is
found wherever it ends up, including inside a subtree that was **replayed from the paint
cache** rather than walked. A registry left out of the cache's recording is the kind of
bug that answers on the first frame and falls silent on the second; a test builds the same
frame twice, checks the second came from the cache, and checks the region came back with
it.

The hook is forwarded by the transparent wrapper, `Box` and `Responsive`, so a region
behind a `Keyed` still answers.

## Getting it onto the device

The window and decor-view calls check the thread they are made on, and the native code
runs on its own. The shell already had a way across: the input bridge's dex, loaded by an
in-memory class loader, whose methods post to the Java UI thread. So the system bars are a
second class in the **same dex** — `FrusSystemBars.apply(activity, status, nav,
darkStatusIcons, darkNavIcons)` — loaded through the same loader, and on its own terms: a
failure to load it costs the bars, not the keyboard.

- **Colours:** `setStatusBarColor` / `setNavigationBarColor`, with the window told to draw
  the bars' backgrounds. That flag is set **at runtime, after the first frame**, and not in
  the launch theme: the demo's `styles.xml` records that setting it there brought back the
  white flash on opening.
- **Icons:** the `WindowInsetsController` appearance flags from API 30, the decor view's
  light-bar flags before it (the navigation bar's from API 26).
- **API 35 and later:** an application targeting them is drawn edge to edge and the colour
  setters do nothing. This one targets 34; the icon half is the part that survives.

The shell tells the platform **only when the answer changes**, and a request that did not
reach the Java side is asked again next frame rather than remembered as made. Desktop and
web have no such bars, and nothing is sent.

## A demo bug in the way

Photographing the light theme's bars was impossible at first: on a phone in night mode,
the demonstration's own **Light** switch did nothing. The demo pins its theme mode, and the
framework then asks `theme()` for the light theme — but since milestone 472 the demo's
`theme()` read the *platform's* brightness and returned the dark theme under a night-mode
phone. `theme()` is the light theme again, and the screenshot tool that called it directly
goes through the resolved theme instead. A test installs a dark platform and pins light;
with the old `theme()` put back, it fails (`Dark` against `Light`).

## In the demo

The **task screen** asks for the navigation bar in its bottom app bar's colour, so the bar
continues into the system's navigation bar instead of stopping at a line where the
platform's colour starts. It says nothing of the status bar, which stays the theme's.

## On the device

Huawei STK-L21, Android 10 (API 29), phone in night mode:

- **dark theme** — both bars the theme's dark background, light icons;
- **light theme, switched live** — both bars light, the clock, battery and signal **dark**
  and readable; before, on this phone, the bars stayed dark whatever the application was
  showing;
- **the task screen** — the navigation bar takes the bottom app bar's colour.

## Found on the way, and not fixed here

On the task screen, the `NavigationBar` at its head **draws under the status bar**. It is
older than this milestone: the scaffold tells its app-bar slot how much the status bar
takes, `AppBar` reads that and holds its toolbar clear, and `NavigationBar` never reads
it. It is its own fix, in the next milestone.

## Verification

- Seven unit tests on the regions: the one under the status bar answering for it and not
  for the navigation bar; the theme answering what no region states, on both themes; a
  colour stated alone getting icons read on it; nested regions field by field; a keyed
  region; a region replayed from the paint cache; the contrast rule at its ends.
- Two demo tests: the light switch under a dark platform, and the task screen's request.
- `frus-shell` checked for `aarch64-linux-android`; the dex rebuilt; the APK built,
  installed and looked at, three screens.
- Six mutations, each failing the test meant for it: the wrapper not forwarding the hook;
  the regions left out of the cache's recording; a stated colour taking the theme's icons;
  the region drawn first winning; the demo's `theme()` following the platform again; the
  task screen asking for nothing.
