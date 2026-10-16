# Milestone 662 — Hot restart in `frus run`

## Objective

Milestone 661 made `frus run` build an application, start it on a device and show its log. A
change to the code still meant ending the run and starting another, and the application came
back on its first page: the developer had to find their way back to the screen they were
working on, and on a desktop the window jumped back to where the system puts new windows.

The reference's tool takes keys while it is attached: `R` restarts the application with the new
code, `q` quits. This milestone does the same, and the new build **opens where the old one
was**.

## What was done

- **Keys** while `frus run` is attached: `R` restarts, `r` asks for a hot reload and restarts for
  now (hot reload is milestone 663), `q` quits and stops the application, `h` lists the keys.
  - A terminal hands a program whole lines by default, so for the length of the run it is
    switched to single keys without echo.
    - On Windows this goes through the console's mode, from kernel32, which every program links
      already, so the tool still has no dependencies.
    - Elsewhere it goes through `stty` on `/dev/tty`.
  - The mode is put back when the run ends, and on Windows also on Ctrl+C, from a console
    control handler.
  - When the input is not a terminal (an editor driving the run), keys are read as lines.
- **A restart builds first.** The running application stays up while the new build compiles; a
  build that fails is reported and leaves it as it was.
- **The application reports where it is** (`frus-shell/src/dev.rs`). Only when `frus run`
  started it, it writes a line each time its location (`Application::location`) or, on a
  desktop, its window's place and size changes. Nothing is asked of it and nothing is waited for.
  - **Desktop**: over a local socket whose address is in `FRUS_DEV`. A restarted application is
    given `FRUS_RESTORE_LOCATION` (opened like a link on the command line) and
    `FRUS_RESTORE_WINDOW` (its outer position and inner size, physical pixels).
  - **Android**: through its log, as `frus-dev location /search`. `frus run` already reads that
    log, and it hides these lines. A socket would need the application to declare the `INTERNET`
    permission, and being restarted must not ask for one; the first version used one, through
    `adb reverse`, and on a phone the application simply never reported. The application knows
    it is being watched from the launch intent's `frus_dev` extra
    (`android_link::launch_extra`). It is restarted with its location as the intent's data, a
    link it reads like any other.
  - **Web**: nothing to report. The served page asks `/__frus/build` once a second which build
    it is, and reloads itself when a restart has made a new one; its address, which is the
    application's location, survives the reload.
- **The desktop application runs from a copy** of the program (`target/frus-run/`). Windows does
  not let a running program's file be written, so without the copy the new build could not
  replace it.

## Verified

On Windows, with `frus-examples/navigation`:

- **Desktop**: `frus run -- /products/3`. The window was moved and resized to 700×560 at
  (210, 140) through the Windows API, then `R`. "Restarting at /products/3", restarted in 6.1 s,
  and the new window came back at (210, 140), 700×560.
- **Desktop, the location alone**: the application started with no argument and
  `FRUS_RESTORE_LOCATION=/search?q=shoes&sort=price` opened on Search with "shoes" in the field
  and the Price sort selected.
- **Web**: `frus run -d web-server`, a headless Edge at `#/products/3`, then `R`. The page
  reloaded by itself (a marker set in its JavaScript was gone afterwards), still at
  `#/products/3`, with the application's console in the terminal again.
- **Android** (Huawei STK-L21, Android 10): `frus run -d android`, the Search tab tapped with
  `adb shell input tap`, then `R`. "Restarting at /search", the APK rebuilt and reinstalled,
  restarted in 34.4 s, and the application opened on Search again. No `frus-dev` line reached
  the terminal.

## Found on the way

- A Rust string continuation (`"\` then a new line) eats the next line's leading whitespace,
  so a key list written that way lost its first line's indentation.
- `android_link`'s new string read borrowed a temporary for too long. It only compiles for
  Android, so a desktop build never saw it; `cargo clippy --target aarch64-linux-android` does.

## Left

- **Hot reload** (`r`): milestone 663.
- **A restart on Android reinstalls the APK**, about 30 s for a debug build, most of it the 270
  MB of debug symbols crossing the USB cable.
- **State beyond the location and the window** (a form half filled, a scroll position) is not
  carried across a restart; that is what hot reload is for.
