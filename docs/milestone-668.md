# Milestone 668 — The keys of `frus run`

## Objective

`frus run` had four keys: `r`, `R`, `q` and `h`. The reference's tool has about twenty, and
developers use them every day. Some look into the running application (the widget tree, the
focus, what a screen reader is told); some change how it shows (outlines around every widget,
another platform's behaviour, the other brightness, a performance panel); and some act on the run
itself (a screenshot, detaching, clearing the terminal). The editor offers the same keys as
commands, a save by hand reloads, and Ctrl+Shift+F5 restarts.

## What was done

- **The keys**: `w`, `f`, `S` print the widget, focus and semantics trees; `i` toggles the
  inspector (F12's), `p` debug paint (the outlines alone), `P` a performance panel; `o` switches
  the target platform, in the reference's order (Android, iOS, Windows, macOS, Linux, Fuchsia);
  `b` switches the brightness the theme is resolved with; `s` saves a screenshot as `frus_01.png`
  (02, 03…); `d` detaches, `c` clears.
  - `t`, `L`, `U`, `I`, `a`, `M`, `g` and `v` say why frus has nothing for them: one tree, no
    layer tree, no timeline, no shader compilation, no DevTools yet.
- **Telling the application**, on every platform, with a `do <command>` line that the next
  frame carries out (`frus-shell/src/dev_commands.rs`):
  - desktop: the socket it already reports on;
  - Android: a file in its own storage, written with `run-as` (a debug build), read four times a
    second;
  - web: `window.__frusCommand`, which the page's script calls with what `/__frus/commands`
    hands it.
  A key pressed while the application starts, or just after a reload, waits for it to connect
  instead of going to the stopped one.
- **In the application**: the dumps go through `log` (so they reach the terminal from every
  platform), and the toggles say what they did. The screenshot is the frame drawn again
  offscreen and written as a PNG with no library: an uncompressed one, with its CRC and Adler
  checksums.
- **For editors**: `{"method":"app.key","params":{"key":"p"}}` presses any key, and
  `app.detach` detaches. The VS Code extension has a command for each key, binds Ctrl+Shift+F5
  to a restart, and `frus.reloadOnSave` takes the reference's modes: `manual` (the default: a
  save by hand, Ctrl+S, not auto save), `manualIfDirty`, `all` and `never`.
- **A restart on Android without a change** no longer installs the package again: cargo-apk
  writes the same bytes for the same build, and an install took 10 s of the 15 s of a restart on
  the test phone.

## Alternatives weighed

- **The Android channel.** A broadcast receiver or a socket would need code in the manifest or
  the `INTERNET` permission; `run-as` needs neither, only a debug build, which is what `frus run`
  builds by default.
- **A screenshot from the window.** The surface's image cannot be read back; drawing the frame's
  scene again offscreen gives the same pixels, and the code already exists for the goldens.
  Android uses the device's own screenshot, which includes the system bars, as the reference
  does. The web has none yet: the page cannot hand its picture to the tool without more work.
- **`--fastdeploy`** for Android installs was measured: on the test phone it fell back to a
  full install and was slower (14–16 s against 10 s).

## Verified

- Unit tests: the key map and its help (every listed key does something, every key that does is
  listed); the `app.key` command; the screenshot names; the web server's command queue; the
  commands from the socket; the PNG (signature, header CRC, the pixels stored); the platform
  cycle; the focus and semantics dumps; a command sent before the application connects.
- Desktop, on the counter and authentication examples: every key, with its answer in the
  terminal; the screenshot shows the outlines, the dark theme and the performance panel; after
  `d` the application was still running.
- Web, in headless Edge: `p`, `b`, `P` and `f`, with the page's screenshot showing outlines, the
  dark theme and the panel.
- Android, on the test phone (Huawei STK-L21): `p`, `o`, `b`, `P`, `w` through `run-as`, each
  answered in the terminal; `s` saved the device's screen; `R` and `r` without a change said "The
  package has not changed" and took 5.2 s and 7.3 s, against about 15 s before.
- The phone showed what the desktop had hidden: the panel under the status bar (it is placed
  below the top inset now), and the application's text and icons over the panel and the
  inspector's card — a scene draws its shapes first and its text after, so both are painted over
  the application's frame taken as one layer.
- VS Code 1.142, isolated: **frus: Toggle Debug Paint** from the command palette, right after a
  reload, and the application's "debug paint on" in the Debug Console.

## Left

- A screenshot on the web.
- The performance panel is text, not the reference's graphs.
