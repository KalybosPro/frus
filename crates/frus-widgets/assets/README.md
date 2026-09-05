# Bundled icons

`material-icons.bin` holds the outlines of the 2 233 **filled** icons reachable as
`frus_widgets::Icons` constants — `Icons::ADD`, `Icons::STAR`, `Icons::ARROW_BACK` —
decoded to a `Path` on demand by `src/icons/mod.rs`.

The set draws each icon in three more styles, and those sit beside it, one blob apiece,
behind a cargo feature apiece:

| file | style | icons | size | feature |
| --- | --- | ---: | ---: | --- |
| `material-icons.bin` | filled | 2 233 | 307 KiB | always |
| `material-icons-outlined.bin` | outlined | 2 193 | 344 KiB | `icons-outlined` |
| `material-icons-rounded.bin` | rounded | 2 199 | 400 KiB | `icons-rounded` |
| `material-icons-sharp.bin` | sharp | 2 200 | 272 KiB | `icons-sharp` |

None of the three is on by default: 1.3 MB of artwork is not something to hand every
application, and most want one of them at most. A blob that is not compiled in costs
nothing at all — `include_bytes!` never runs, and that style's constants do not exist.

## Regenerating

All four are **generated**, not written by hand:

```bash
python scripts/gen_icons.py path/to/MaterialIcons-Regular.otf
```

That reads `scripts/material-icons.codepoints` — one row per icon, giving its name, its
codepoint in each style, and whether it turns round in a right-to-left reading order —
and rewrites every blob and `src/icons/names.rs` together. They are only ever right
together, which is why the generator writes them in one go and the module checks that
they agree at compile time.

## Why a blob and not Rust

2 233 constant path expressions is about a megabyte of source, and every build would pay
for it. A blob is ~140 bytes an icon and carries the coordinates as the integers they
already are in the font, so nothing is rounded away. See the module documentation in
`src/icons/mod.rs` for the format.

## Licence

The artwork is the **Material icon set** by Google, redistributed here as outline data.
It is published under the Creative Commons Attribution 4.0 International licence:

> Material icons — Copyright © Google, licensed under CC BY 4.0
> <https://creativecommons.org/licenses/by/4.0/>

Only the geometry is used; no font file is redistributed.
