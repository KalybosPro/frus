# Milestone 626 — A font family name finds what was loaded

## Objective

An application registered Inter's four static files, named its default family `"Inter"`,
turned the bundled faces off with `default-features = false` — and on Android drew **no text
at all**. Nothing crashed and nothing was logged. On the desktop the same build looked fine.

Three things lined up:

1. **The family inside the files is not "Inter".** Inter's static files call their family
   `"Inter 24pt"` (name ID 16). `set_default_family("Inter")` named a family no face carried.
2. **Android has nothing behind a family that resolves to nothing.** cosmic-text's fallback
   lists are empty there, and with the bundled DejaVu turned off there was no other face to
   reach. The desktop had system fonts to fall into, which is why it hid the mistake.
3. **`default-features = false` also dropped `images`.** The guide recommended exactly that line
   for shipping your own fonts, and with it every `Image::memory` and `asset!` painted nothing,
   again without a word.

Each is a mistake a developer makes once, finds on a device, and cannot see from the code.
The framework should catch it.

## What was done

### A family name is resolved against the faces loaded

`frus-text` keeps an inventory of every font database it builds (`Loaded`): the families, the
ones the application registered (`add_font` now reads each file's family names), which have
italic faces, and whether the platform's generic sans and monospace resolve. Every family
name, whether it comes from `set_default_family`, `set_monospace_family` or a `TextStyle`'s
`FontFamily::Named`, goes through `loaded_family`, which tries, strictest first:

| try | example |
|---|---|
| the name exactly | `"Inter 24pt"` → `"Inter 24pt"` |
| up to case, spaces, hyphens, underscores | `"open-sans"` → `"Open Sans"` |
| a family that extends it by a word | `"Inter"` → `"Inter 24pt"` |

Among several extensions the application's own come first, then the shortest (`"DejaVu"`
finds `"DejaVu Sans"`, not `"DejaVu Sans Mono"`). It guesses no further: `"Inter Tight"` does
not find `"Inter"`, and `"Int"` finds nothing. A wrong face drawn silently is the failure
this removes, so the matching must not introduce a new one.

When a name matches nothing, text is drawn in the application's first registered face, then
the platform's sans, then any face that draws text. A name that resolves to nothing is never
passed to the shaper. Monospace without a monospace face draws in the default family.

**The console says so, once per name:**

```text
WARN frus: no font family is called "Inter"; drawing "Inter 24pt", the name inside the font files. Write "Inter 24pt" to say so.
WARN frus: the default font family "Helvetica Neue" is not loaded; drawing "Inter 24pt" instead. The name must be the one inside the font file (add_font registered ["Inter 24pt"]).
WARN frus: add_font was given 1234 bytes that hold no font face; nothing was registered
```

### Android takes its own sans when nothing else can draw

When no bundled or registered face can draw default text, `new_font_system` loads the
platform's `Roboto-*.ttf` from `/system/fonts` (not its condensed cut). It is the last resort,
not the default: a build with `bundled-sans` or its own faces never touches it.

### Every registered weight is used

`available_weight` answered 400 or 700, whatever was loaded. That was right for the bundled
sans and wrong for an application that registers Medium and SemiBold: they were never drawn.
It now picks among the weights the default family covers, by the rule web type follows: exact;
for 400–500 the next heavier up to 500, then lighter, then heavier; below 400 lighter first;
above 500 heavier first. A variable face covers its whole `wght` axis. The bundled sans gives
the same answers as before.

### An image that cannot be decoded says so

`Image::memory`, `asset!` and `Image::network` now log a decoding failure once per image. The
missing-decoder message names the cause and the fix: *"this build dropped the `images`
feature, which `default-features = false` turns off; add `features = ["images"]` to the frus
dependency"*. The getting-started guide's line for shipping your own fonts now keeps `images`,
and says why.

## Limits

- A weight is chosen from the **default** family's faces. A run in another family (the
  Arabic face, a named family) asks for that weight too, and when that family lacks it
  cosmic-text falls through its last pass to a face that has the glyphs. Before this
  milestone the same happened with the bundled mono's missing bold.
- Matching reads family names only. It cannot tell that a face lacks a script's glyphs; that
  limit is unchanged and documented on `family_for_style`.

## Tests

- `a_family_name_finds_the_one_inside_the_file`: the three tries, the preference order, and
  the names that must not match.
- `a_weight_snaps_to_the_nearest_loaded_by_the_webs_rule`: the bundled answers unchanged,
  four static weights all used, each branch of the rule, a variable range.
- `the_inventory_reads_the_faces_loaded`: families read from a file, a non-font refused, the
  inventory of a database, and when Android should load its own sans.
- `a_family_found_by_its_plain_name_draws_real_glyphs`: in a database with no system font, as
  on Android, the family `match_family` found shapes real glyphs from that face.
- `the_platform_sans_is_robotos_files_and_nothing_else`.
