# Milestone 727 — A project's own devices

## Objective

The last open part of simulated devices (milestone 720): a developer who targets a phone frus
does not know describes it once, in the project, and runs on it like on a built-in one. Chosen
with the owner: in the application's `Cargo.toml`, with no file of its own.

```toml
[[package.metadata.frus.devices]]
id = "pixel-9"                  # what -d takes: lowercase, digits, -
name = "Pixel 9"                # optional: the id
platform = "android"            # ios or android
width = 412                     # logical, held upright
height = 924
ratio = 2.625
corner-radius = 40              # optional: 0
safe-area = [52, 0, 24, 0]      # top, right, bottom, left; optional: none
landscape-safe-area = [24, 0, 24, 52]  # optional: the upright one, turned
keyboard = 336                  # optional: two fifths of the height
landscape-keyboard = 260        # optional: three fifths of the width
cutout = "hole"                 # none, notch, island or hole; optional: none
cutout-size = [34, 34]          # width, height
cutout-top = 15
```

## What was done

- **`DeviceProfile::parse`** (frus-core) reads a device written as `key=value;…`, the same keys,
  lists as `a,b,c`. It fills the defaults and names what is wrong: a missing field, a word where
  a number goes, a list of the wrong length, an unknown platform or cutout. An **unknown key** is
  an error too, so a misspelt one cannot leave its default in place without a word. The outline
  is a plain body around the screen.
- **The command line** reads the tables from the project's `Cargo.toml` with its own reader
  (it still depends on nothing) and writes each one as such a description. `discover()` adds them
  for the project `frus` is run in, so `frus devices`, `frus run -d` and the editor's device list
  all see them. One with a built-in id replaces the built-in device. One missing a field is listed
  with what it lacks rather than left out.
- **The application receives the whole description** through the same channel as a built-in id:
  `FRUS_DEVICE` on a desktop, `window.__frusDevice` on a page (milestone 724). The shell tells
  them apart by the `=`. The page's script now escapes whatever the description holds instead of
  refusing anything that is not an id; the old literal also carried a raw line break, now `\n`.

## Checked for real

A Pixel 9 was added to `frus-examples/counter/Cargo.toml` for the check, then taken out again.
`frus devices` listed *Pixel 9 (simulated) • pixel-9 • Android, this project's: 412×924
@2.625x*. `frus run -d pixel-9 --machine` started the counter on it (the log says *simulating
Pixel 9 (Android, 412×924 at 2.625x)*). The screenshots showed its 40-point corners and its hole,
and on its side the status bar kept and the title clear of the hole.

On the counter, the status bar's clock is drawn in a serif face, unlike in the demo: the example
seems not to bundle the face the shell asks for. That is outside this milestone and is left noted.

## Tests

- `device::a_described_device_is_read`: every key; only the required ones, with the defaults; the
  safe area turned for landscape.
- `device::a_wrong_description_says_what_is_wrong`: eleven mistakes, each named.
- `simulated::a_whole_description_is_a_device_too`.
- `project::the_project_s_own_devices_are_read`: two tables, comments, lists; each one parses in
  frus-core; `;` and `=` cannot end a pair early.
- `devices::a_project_s_own_devices_join_the_list`: picked by name, replacing a built-in id,
  listed with what it lacks.
- `serve::a_page_is_told_its_simulated_device`: a description with a quote, `</script>` and a
  backslash stays inside its string.
