# Milestone 720 — Device profiles

## Objective

A frus developer should see an application as it will look on a phone without owning one, and
without the platform's own tools: an iOS simulator needs a Mac, an Android emulator a heavy
install. frus draws every pixel itself, so the same desktop binary can **behave** like a phone:
the screen's size and density, the areas its system keeps, its keyboard, its platform's
conventions, drawn inside the device's outline. On every development system: Windows, Linux
and macOS, and later the web.

This is the first of five steps:

1. **Device profiles** (this milestone): what a device is, as data.
2. The simulated screen in the shell: the application drawn inside the screen, at the device's
   density, with its insets and its platform; the outline, the status bar and the cutout painted
   around it; pointer input mapped into the screen.
3. Interactions: rotation, the soft keyboard, light and dark, text size.
4. The command line: the profiles in `frus devices`, `frus run -d iphone-15`, hot reload.
5. Later: the web, and an in-application preview.

Running the real iOS was set aside: the simulator only exists inside Apple's developer tools,
which only run on macOS, and the iOS SDK's licence keeps it to Apple hardware. A simulated device
is a **preview**. It shows the layout, the safe areas, the keyboard and the platform's behaviour.
It does not show the device's own fonts (text measures differently), its graphics driver, its
native services or its speed.

## Decisions

- **Debug only, for now**: the profiles feed `frus run`; no widget shows them inside an
  application yet.
- **A real outline**: a body with rounded corners, bezels and side buttons, drawn by frus. It
  is not a picture of any device.
- **A generic system**: one `DeviceProfile` type for every platform, so an Android phone, an
  iPad or a developer's own device is described the same way as an iPhone.

## What was done

`frus_core::DeviceProfile`, pure data with no dependency:

- `id` (`iphone-15`, what a command line types), `name`, `platform`;
- `screen` (logical, held upright), `pixel_ratio`, `screen_radius`;
- `cutout`: `None`, `Notch`, `Island` or `Hole`, centred on the top edge;
- `portrait` and `landscape`: `DeviceInsets { safe, keyboard }`, the system's areas and the
  keyboard's height measured from the bottom edge;
- `frame`: `DeviceFrame { bezel, radius, home_button, buttons }`, with `SideButton` on a
  `ButtonEdge`.

Queries the shell will need:

- `screen_size`, `physical_size`, `insets_in` for an `Orientation`;
- `window_insets(orientation, keyboard)`: the `WindowInsets` a real shell reports, built with
  `WindowInsets::from_baseline`. The keyboard covers the home indicator, so `padding.bottom`
  falls to zero while `view_padding` keeps it;
- `cutout_rect`, `outer_size`, `screen_rect`. Landscape is the device turned a quarter turn
  counter-clockwise: the cutout and the top bezel go to the left;
- `find(profiles, wanted)` by id or name, ignoring case.

`DeviceProfile::builtin()`: `iphone-se`, `iphone-15`, `iphone-15-pro-max`, `ipad`,
`android-phone`. The iPhone and iPad screens, densities, islands and safe areas are the published
figures. The keyboard heights are typical; real ones vary with the keyboard, the language and the
suggestions bar. **The Android phone's status bar, navigation bar and keyboard are estimates**
(48, 24, 300 upright). They are to be measured on the emulator before step 2 relies on them.

Re-exported from `frus-widgets` (the `core_exports` guard).

## Tests

`device.rs`:

- `every_builtin_profile_has_its_own_id`: ids unique and command-line shaped; found by id or by
  name in any case; an unknown one is not found.
- `every_builtin_profile_is_consistent`: whole physical pixels; safe areas and keyboard fit on
  the screen in both orientations; the keyboard covers the bottom safe area; the cutout is inside
  the area kept for it; the screen sits inside the body.
- `landscape_is_a_quarter_turn_counter_clockwise`: sizes, cutout and screen position on its
  side, on a phone with an island and on one with a home button.
- `the_keyboard_moves_the_view_insets_only`: the split of `WindowInsets` upright and on its
  side, keyboard down and up.

## Not done

- No profile is read from a file yet. Step 4 decides the format with the command line.
- The Android figures, as said above.
