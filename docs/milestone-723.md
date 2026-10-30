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

### A lost first command

The first runs wrote one screenshot for two presses of `s`, on `-d windows` too. That looked
like a shell that ignored a request on an idle window. It was not: sent to the application
directly, one request on an idle window was written in 0.7 s. The timestamped output of
`frus run` showed the real cause. Command 1 got **no answer at all**, while commands 2 and 3
did.

The test script was Windows PowerShell, which writes a UTF-8 byte-order mark before the first
line it sends a program. `frus run --machine` read `\u{feff}{"id":1,…}`, could not parse it,
and dropped it without a word. A blank first line made both screenshots appear.

So `frus run --machine` now reads past a byte-order mark, and answers a line that is not a
command (`{"id":null,"error":"not a JSON command"}`) instead of ignoring it. Run again with no
workaround, `s`, `l`, `s` gave three answers and two screenshots: portrait, its status bar in
dark ink over the light counter, then landscape.

## Tests

- `devices::the_simulated_devices_are_the_core_s`: the same ids, names, platforms, screens and
  densities as frus-core's, in order.
- `devices::simulated_devices_are_picked_like_the_others`: by id, by name, `simulated` too many,
  no `-d` still the desktop, the table's line.

## Not done

- Profiles of a developer's own, from a file in the project. The type takes them; the format
  and where the file lives are still to decide.
- The web (step 5).
