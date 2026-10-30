# Milestone 721 — The simulated screen

## Objective

Step 2 of simulated devices (milestone 720 has the plan): a desktop window that **stands in
for a device**. The application is laid out for the device's screen and drawn inside the
device's outline, on Windows, Linux and macOS alike.

## What was done

`FRUS_DEVICE=<id or name>` in a **debug** build (`frus run -d` will set it in step 4) makes the
shell simulate that built-in `DeviceProfile`. A release build ignores it, with a warning. An
unknown name lists the known ones, and the window opens as itself.

`frus-shell/src/simulated.rs`:

- **The window** opens the device's shape, its margin included, and shrinks to fit 90% of the
  monitor.
- **Placement**: the device as large as the window allows, never larger than itself, centred.
  A window resized smaller zooms it down; nothing is cropped.
- **What the application is told**: the device's logical screen size, its `WindowInsets` (the
  profile's safe areas and keyboard), its platform (through the debug platform override, so
  every widget that follows `theme.platform` follows the device), and its pixel ratio in the
  `MediaQuery`, with a density of 1.
- **The scale**: `total_scale()` becomes the window's DPI × the zoom. The application's frame
  is drawn at that scale, moved to the screen's corner and clipped to the screen's rounded
  corners.
- **Pointer, touch and dropped files** are mapped from the window into the screen
  (`App::to_logical`), so the screen's corner is the application's origin at any zoom.
- **Painted by the shell, around and over the application**: the backdrop, the body and its
  shadow, the side buttons, the glass, the cutout (island, notch or hole), a status bar (time,
  signal, battery) whose ink follows what the application asks of the status bar
  (`system_ui_style`, as on Android), the home indicator or the home button. On its side, an
  iPhone has no status bar.
- The window's **shared title bar** is off while simulating: a device has none.
- **The test harness is never simulated**: only `App::new`, the real window's constructor,
  reads the variable.

## Checked in the real application

The demo was run as `iphone-15`, `iphone-se`, `ipad` and `android-phone`, and a screenshot was
taken through `frus run`'s channel (`do screenshot`). It shows the iOS or Android conventions,
the safe areas kept, the island, the hole, the home button, and the iPad zoomed to fit with the
demo's wide layout.

The first run showed **square black corners** past the body's: the glass under the application
was drawn square while the application was clipped round. It is now as round as the screen, and
a test asserts it; with the old line back, that test fails.

The pointer mapping was not exercised by hand in this milestone. Only the tests cover it.

## Tests

`simulated.rs`: a device named by id or name, and an unknown one listing the known ones; the
application told the device's size and insets, keyboard up and on its side; the device fitting
the window at its own size and zoomed down; the pointer mapped into the screen at both zooms;
the application's frame in the screen, clipped round, on round glass, with the island and the
status bar's ink over it; no status bar for an iPhone on its side.

## Not done (next steps)

- Rotation, the soft keyboard, brightness and text size from the keyboard: step 3.
- `frus devices` and `frus run -d`: step 4.
- The Android profile's figures are still the estimates of milestone 720.
