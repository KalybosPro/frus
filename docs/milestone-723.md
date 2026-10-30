# Milestone 723 — `frus run -d iphone-15`

## Objective

Step 4 of simulated devices (milestone 720 has the plan): what milestones 721 and 722 built
behind `FRUS_DEVICE` becomes an ordinary device of the command line, listed by `frus devices`
and run by `frus run -d`, the way the desktop, a phone and a browser are.

## What was done

- **`frus devices` lists the simulated devices**, always and on every system, after the real
  ones:

  ```text
  iPhone 15 (simulated)         • iphone-15         • iOS, simulated: 393×852 @3x
  Android phone (simulated)     • android-phone     • Android, simulated: 411×914 @2.625x
  ```

- **`frus run -d iphone-15`** builds the desktop application and starts it with `FRUS_DEVICE`
  naming the profile, on both the restart path (`r`, `R`) and the hot-reload path (`--hot`,
  which hands the variable to the application dx starts). `-d` also takes the name
  (`-d "iPhone 15"`). `-d simulated` alone is too many and lists them.
- **`--release` is refused** with a simulated device: the shell simulates in a debug build
  only, and a release window would silently be the desktop's.
- `-d ios` still means a Mac's real iOS simulators. A simulated device has a platform of its
  own, `simulated`.
- **The editor** sees a simulated device as the desktop it really is (`platform` the host's,
  category `desktop`). An editor asks the application whether it supports a device's platform,
  and what runs is the desktop build.

The command-line tool depends on nothing, on purpose, so the list of simulated devices (id,
name, platform, screen, density) is written in `devices.rs`. A test, with frus-core as a
dev-dependency only (left out when the crate is published), keeps it the same as
`DeviceProfile::builtin()`.

## Checked for real

- `frus devices` on Windows lists the desktop, an Android emulator, Edge, the web server and
  the five simulated devices.
- `frus run -d iphone-15 --machine` in `frus-examples/counter`, driven through its JSON channel:
  the build ran, the application started (`app.start` names *iPhone 15*, the log says
  *simulating iPhone 15 (iOS, 393×852 at 3x)*), `l` turned it (*iPhone 15 Landscape*), and `s`
  wrote the screenshot: the counter on its side, its title clear of the island, the home
  indicator below.

The same run found a bug that is not this milestone's, and is on the plain desktop too
(`-d windows`): **on an idle application, `s` writes nothing until something else asks for a
frame.** Two presses wrote one file. The second press's frame carried both requests, and both
named `frus_01.png`, the first free name when each was sent. The demo always has a frame
coming, which is why the screenshots of milestones 721 and 722 never met it. Left for its own
fix.

## Tests

- `devices::the_simulated_devices_are_the_core_s`: the same ids, names, platforms, screens and
  densities as frus-core's, in order.
- `devices::simulated_devices_are_picked_like_the_others`: by id, by name, `simulated` too many,
  no `-d` still the desktop, the table's line.

## Not done

- Profiles of a developer's own, from a file in the project. The type takes them; the format
  and where the file lives are still to decide.
- The web (step 5).
