# Milestone 567 — Shipping only the glyphs you draw

## Objective

Issue #20: the bundled DejaVu and Naskh faces are 3.4 MB, about 40% of a minimal application,
and cover far more of Unicode than any one application draws. The issue asks for a build step
that subsets them, and asks that the interface be agreed *before* the tool is built: how an
application declares the ranges it needs, what happens when a glyph is missing, and that nothing
may panic. The maintainer chose to start with a **documented recipe** and no new tool.

## What it does

No code changes. The getting-started guide gets a section, *Shipping only the glyphs you draw*,
and the existing "ship your own face" example is corrected: it showed `add_font` with no place to
put it. `frus::main!` takes an **expression**, so a block runs first on every platform.

```rust
frus::main!({
    frus::fonts::add_font(include_bytes!("../fonts/DejaVuSans-Subset.ttf").to_vec());
    frus::fonts::set_default_family("DejaVu Sans");
    FrusApp::stateful(Counter).title("my-app")
});
```

## What it weighs

The template's counter, generated with `cargo generate` and built against the crates published
on crates.io (0.2.1), release, `aarch64-linux-android`, rustc 1.96.1, NDK 26.3. Each number is
the size of the file.

| fonts | APK | `libmy_app.so` |
|---|---|---|
| all four bundled groups (default) | 5,198,242 | 11,340,584 |
| `default-features = false, features = ["bundled-sans"]` | 4,166,050 | 9,356,896 |
| no bundled font; DejaVu Sans regular and bold cut to Latin (43 kB) | 3,453,346 | 7,933,248 |

The library shrinks by 3.4 MB from first to last row — exactly the fonts — and the APK by
1.74 MB, a third. The first row is the counter without the extra text line the other two carry;
that line is a few bytes. Not measured: the web build and the desktop binary.

## What a phone showed

The two builds carried a line of text, `Café € 日本 ب Ωμέγα`, and a Huawei STK-L21 (Android 10)
drew them.

- **`bundled-sans` only:** Latin and Greek rendered, the Arabic letter rendered (DejaVu covers
  it), the Japanese became boxes.
- **Latin cuts only:** Latin rendered; Greek, Japanese and Arabic each became boxes — **and the
  `−` on the template's Outlined button too.** The first list of code points did not include
  U+2212, the minus sign the template draws. Adding it brought the button back, and the size did
  not move. This is the recipe's sharpest edge and the guide says it in those words: the
  framework and the template draw characters the sources do not spell out.
- No panic, and nothing in the log, in either.

The boxes are the face's `.notdef` glyph, kept with `--notdef-outline`. The subset was
not built without that option, so what a missing character does then is **not known here**.

## Decisions

**A recipe, not a tool.** The issue's hard part is knowing which glyphs a build uses, and a
static reading of the sources finds only the literals. `pyftsubset --text-file` over `src/*.rs`
does that reading and costs one line, and `--unicodes` declares the ranges runtime text can fall
in; together they are the "declare the ranges" interface, made of a tool that already exists. A
Rust command reading a `frus-fonts.toml` would add a crate to publish and maintain for what this
does today; it is worth building the day the recipe proves tiresome, not before.

**Measured on Android only.** Nothing in the recipe is specific to it, but only Android was
measured and looked at on a device, and the guide says so.

## Left

- **The tool**, and a fallback that *names* the missing character (a log line, once per
  character) instead of drawing a box the developer has to interpret. Neither exists.
- **`.bold()` on a regular-only cut** was not tried. The guide says the measured build carried
  both weights and gives the renderer's exact-weight rule as the reason, not as a result.
- **Sizes on the web and the desktop.**
- **The Arabic and monospace faces**, which the recipe does not cut: DejaVu Sans Mono and
  Naskh follow the same commands with their own `--unicodes`, untried.
