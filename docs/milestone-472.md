# Milestone 472 — Every colour has a name, and so does every icon

Two vocabularies an application expects to find already written: the material palette,
and the material icon set. Neither is hard. Both were missing, and the shape each one
takes in Rust is not the shape it takes in the reference, so both are worth writing down.

## `Colors` — 308 colours and 35 ramps, and no wrapper to unwrap

The reference's `Colors.red` is a *swatch object* that also happens to be a colour,
because its language lets a class extend `Color`. Rust does not, and the naive
translation makes every call site pay for it: `Text::new("…").color(Colors::RED)` stops
compiling the moment `Colors::RED` is a struct with a colour inside, and every widget
API in the framework would have to become generic over `Into<Color>` to buy the sugar
back.

So the palette is **plain `Color` constants**. `Colors::RED` is a `Color`. So is
`Colors::RED_300`, `Colors::BLUE_ACCENT_700`, `Colors::BLACK54`. Nothing to unwrap,
nothing to convert, and not one existing signature had to change.

### The ramp is a separate type, because it answers a different question

A single tone is what a caller picks. A **ramp** is what a theme derives from: give me
the whole red family and I will choose a step per role. That is a different question and
it gets a different type, `MaterialColor`, reached as `MaterialColor::RED` — with
`shade(300)`, `steps()`, and a `From` into its primary so a ramp can stand in for a
colour without a cast.

The steps are a `&'static [(u16, Color)]` rather than a fixed array on purpose. Grey has
twelve steps, not ten — it carries `350` and `850` — and the accents have four. A fixed
array would have forced grey's two extras out of the type or padded every other family
with holes. A slice lets a ramp **state which steps it has**, and lets `shade` answer
`None` for one it does not, which is the honest answer and not the nearest neighbour.

### The neutrals are alphas, and that is the whole point of naming them

`Colors::BLACK54` is black at 54% — an **alpha**, not a grey. The distinction is not
pedantry here: this framework has been bitten before by a translucent token modelled as
an sRGB blend, which paints at roughly a third of the opacity it names. Shipping the
palette's own eight black and nine white opacities as alphas is what stops the next
person from re-deriving one by hand and getting it wrong.

There is a test that asserts exactly this: the RGB of every neutral is pure black or
pure white, and only the alpha moves.

### What the tests actually check

Structural properties, not a transcription of the table:

- every ramp's primary **is one of its own steps** — the one error a palette can carry
  silently, and the one a spot-check of six values would miss;
- the steps ascend, and darken monotonically by relative luminance, which is the property
  a caller relies on when reaching for a step to get contrast;
- a handful of exact hex codes, sampled across the table, because the structural tests
  cannot see a whole family shifted by one step.

Writing that third test found a real error — in the test. The first draft paired
`YELLOW_ACCENT_700` with `#FFEA00`, which is the `400` step. The constant was right; the
expectation was written from memory. That is the argument for generating the constants
from the specification and hand-writing only the assertions.

## `Icons` — an enum with sixteen arms does not become an enum with 2 233

The old `Icons` was an enum, and each icon was an arm of a `match` returning a
hand-drawn `Path`. Sixteen of them, each one carefully placed on the 24 × 24 grid, and
every one of them good. The approach does not survive contact with a real icon set:
nobody is going to place 2 233 icons by hand, and an enum closed at the crate boundary
means an application whose mark is not in the set has no way in at all.

### An icon is a value, not a variant

`IconData` is now the type, and it holds one of two things: an index into the bundled
set, or **a function that draws a path**. That second case is the escape hatch — a
`const fn`, so a caller's icon is declared exactly where a bundled one would be:

```rust
fn lozenge() -> Path { Path::rect(Rect::new(4.0, 8.0, 16.0, 8.0)) }
const LOZENGE: IconData = IconData::custom(lozenge);
```

A function pointer rather than a `Path` because a `Path` is not a `const`, and because
an icon that is never painted should cost nothing.

`Icons` survives as the **name**: a unit struct carrying 2 233 associated constants,
`Icons::ADD`, `Icons::STAR`, `Icons::ARROW_BACK`. Rust's constant case is the only
deviation from the set's own spelling, plus the fifty-seven names that begin with a
digit and cannot be identifiers — `Icons::TEN_K`, `Icons::FOUR_K_PLUS` — which the set's
own bindings already spell out the same way.

`Icons::by_name("arrow_back")` is there for the case a name arrives at runtime, from a
configuration file or a design tool, and `Icons::all()` walks the set for a picker.

### Function pointers do not compare the way `derive` thinks they do

`IconData` has to be `Eq` and `Hash`: existing widget code compares `Option<IconData>`,
and a caller may key a cache on an icon. Deriving them draws a warning, and the warning
is right — the compiler may merge two identical functions to one address, or give one
function two addresses across codegen units.

So the comparison is written by hand, over `std::ptr::fn_addr_eq`. It can answer `true`
for two different functions that happened to be merged. It will never answer `false` for
one function compared with itself, and that is the direction that matters for a cache.

### The outlines are a blob, and the blob is checked by the compiler

2 233 icons as constant path expressions is roughly a megabyte of Rust that every build
would have to parse. They live instead in `assets/material-icons.bin`, 307 KiB — about
140 bytes an icon — walked into a `Path` on demand.

The format is a header, an offset table, and one byte-code stream per icon. Seven
opcodes: close, move, line and cubic, each of the last three in a **signed-byte delta**
form and an absolute `i16` form for the rare jump a byte cannot express. Coordinates are
the font's own integers, so the blob is exact — the generator checks the round trip
before it writes anything, and the decoder in `icons/mod.rs` is the twin of the one in
`scripts/gen_icons.py`.

Two things are asserted at **compile time**, in a `const` block: the blob's signature,
and that its offset table ends exactly where the file does. A build that picked up a
stale or foreign file would otherwise decode it into nonsense — silently, in every icon
on the screen. That is not a failure worth discovering in a screenshot.

The grid is read from the blob rather than written in the widget. Three widgets had been
carrying their own `const ICON_GRID: f32 = 24.0`; they now read the blob's, so the grid
the paths were generated on and the grid the widgets scale from cannot drift apart.

### The rename, and what it cost

Sixteen names changed, because the set's names are not the ones we had invented:

| was | is | why |
| --- | --- | --- |
| `Icons::Heart` | `Icons::FAVORITE` | the set's name |
| `Icons::Play` | `Icons::PLAY_ARROW` | " |
| `Icons::ArrowLeft` | `Icons::ARROW_BACK` | " |
| `Icons::ChevronDown` / `Up` | `Icons::EXPAND_MORE` / `LESS` | the set has no vertical chevron under that name |
| `Icons::Eye` / `EyeOff` | `Icons::VISIBILITY` / `_OFF` | " |
| the other ten | upper case | Rust's constant case |

Every one of the framework's own call sites moved. Thirty-five goldens changed, and were
looked at before being accepted: the artwork is the set's own, which is thinner and
better proportioned than what we drew — the back arrow most visibly — and nothing else
in any of the thirty-five frames moved.

One test broke for a reason worth recording. `transparent.rs` walks `src/` and reads
every entry to check that each wrapper states the hooks the macro leaves out. `icons.rs`
became `icons/`, and on Windows reading a directory is a `PermissionDenied`, not an
empty string. The walk now skips anything that is not a `.rs` file. A test that finds
its subjects rather than listing them is still the right test; it just has to know what
a subject is.

### Right-to-left, and the sixteen places that painted an icon

An arrow, an indent, a reply and a chevron all point somewhere, and where they point is
relative to the reading order: in a right-to-left order, *back* is to the right. A tick,
a star and a magnifying glass point nowhere and must not be touched.

Which is which is not a judgement a widget can make, and it is not a judgement worth
making twice. The set states it, and **76 of the 2 233** carry the flag; the generator
bakes it into the constant, so `Icons::ARROW_BACK` is built by a different constructor
from `Icons::CHECK` and the flag costs no lookup at run time. A caller's own icon says it
the same way: `IconData::custom(draw).mirrored()`, still a `const`.

Applying it was the interesting part. Sixteen places in the crate painted an icon, and
every one of them wrote the same three calls by hand:

```rust
name.path().scaled(size / ICON_GRID).translated(x, y)
```

Sixteen places to remember a rule in is sixteen places to forget it in — and three of
them were carrying their own `const ICON_GRID: f32 = 24.0` to do it with. So the whole
expression became one method on the icon:

```rust
name.placed(size, x, y, theme.direction)
```

which scales, positions, and reflects in the vertical centre line of the square it was
just placed in — so a mirrored icon occupies **exactly** the box an unmirrored one would,
and no layout that reserved a square for an icon has to know any of this happened.

The reflection is `Path::mirrored_x`, new in `frus-core`. It reverses each contour's
winding, and it reverses *every* contour, so a shape's holes stay holes: the non-zero
rule cares about the relative direction of the contours and not about their absolute one.

Two things say it works. A unit test weighs the reflection algebraically — every x
reflected about the axis, every y untouched, the bounding box unmoved, and mirroring
twice the identity. And a golden renders `arrow_back` alone in a frame **exactly its own
size**, so the layout's mirroring has nowhere to move it to and anything that changes is
the glyph: under right-to-left the arrow's ink leans the other way by the same amount,
and a tick renders byte-identical in either direction.

Not one of the thirty-five existing goldens moved when the sixteen call sites changed,
which is the evidence that `placed` is the expression they were each writing.

### The other three styles, and the names that nearly broke them

The set draws each icon four ways — filled, outlined, rounded, sharp — and the pairing is
what they are for: an outlined icon at rest and its filled twin when selected is how a
navigation bar says which destination you are on. All four are here, one blob each,
behind `icons-outlined`, `icons-rounded` and `icons-sharp`.

| style | icons | blob |
| --- | ---: | ---: |
| filled | 2 233 | 307 KiB |
| outlined | 2 193 | 344 KiB |
| rounded | 2 199 | 400 KiB |
| sharp | 2 200 | 272 KiB |

None of the three is on by default. 1.3 MB of artwork is not something to hand every
application, and a feature that is off costs nothing at all: `include_bytes!` never runs,
and that style's constants do not exist. Nor does its *name*: `Icons::by_name` answers
`None` for `"add_outlined"` without the feature rather than handing back the filled
drawing, because an application that asked for the outlined set and silently got a
different one would ship looking wrong, and a `None` it can report is worth more than a
picture nobody chose.

**Adding the styles found a bug in the filled set.** The names had been read out of the
reference's own bindings by stripping a `_outlined` / `_rounded` / `_sharp` suffix — and
two icons are *named* that way without being variants of anything:
`insert_chart_outlined` and `wifi_tethering_error_rounded` are filled icons in their own
right, whose own outlined variants are `insert_chart_outlined_outlined` and
`wifi_tethering_error_rounded_outlined`. Both had been silently dropped. A third,
`class`, had come through as `CLASS_`, carrying an escape that belongs to the reference's
language and not to the icon.

The fix was to stop inferring. Membership and codepoints now come from the **font's own
manifest**, which names each icon and style explicitly and cannot be misread; the
bindings are consulted only for spelling, and only for the fifty-seven names that begin
with a digit and therefore cannot be identifiers. Every match is verified rather than
assumed, and the two names that read like a suffix have a test of their own — a lookup
that stripped suffixes would get exactly that pair wrong, so the lookup does not strip
suffixes.

One icon, `face_unlock`, exists in the three variant styles and not in the filled one.
It is kept: `Icons::FACE_UNLOCK_OUTLINED` exists, `Icons::FACE_UNLOCK` does not, and a
hole nobody could see would have been worse than an asymmetry that is written down.

### What is not here

- **A path cache.** Decoding is a linear walk over a few hundred bytes, which is cheap
  and not free. The module says so, and says to hold the `Path` if you are painting the
  same icon thousands of times a frame.

## Regenerating

```bash
python scripts/gen_icons.py path/to/MaterialIcons-Regular.otf
```

Zero dependencies: the OpenType and CFF/Type 2 readers in that script are the minimum
needed to walk a glyph outline. It reads `scripts/material-icons.codepoints` — the
name → codepoint table, checked in — and rewrites both the blob and
`src/icons/names.rs`, which is why it writes both and the module checks that they agree.

The artwork is the material icon set, CC BY 4.0; see
`crates/frus-widgets/assets/README.md`. Only the geometry is redistributed, not a font.
