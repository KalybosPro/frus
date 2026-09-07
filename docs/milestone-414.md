# Milestone 414 — The last seven, and the note that was wrong about them

Milestone 413 mapped twelve widgets' private text constants onto the reference's type scale
and left seven behind, with this reason:

> They are a **different problem**: the reference has no counterpart for them, so their role
> has to be argued rather than read.

**That was wrong about five of the seven.** The reference names a time picker's dial, its
help line and its day period; it names a slider's *value indicator*; it names an input
decorator's label and helper line; it names an app bar's title and its toolbar text. Only
the breadcrumb, the board and the error summary have no counterpart.

The note was an estimate standing in for a look — which is exactly what the constants
themselves were.

## What the reference actually says

| widget | was | the reference | now |
| --- | --- | --- | --- |
| `TimePicker` cells | 15 | `dialTextStyle` → `bodyLarge` | 16 |
| `TimePicker` "Hour"/"Minute" | 13 | `helpTextStyle` → `labelMedium` | 12 medium |
| `TimePicker` preview | 28 | — (see below) | `headlineMedium`, 28 |
| `TimeRange` "Start"/"End" | 14 | — | `titleSmall`, 14 medium |
| `Slider` value bubble | 12 | `valueIndicatorTextStyle` → `labelLarge` | 14 medium |
| `AppBar` title | 22 **medium** | `titleTextStyle` → `titleLarge` | 22 **regular** |
| `AppBar` actions | 14 | a bar's actions are text buttons → `labelLarge` | 14 |
| `NavigationBar` title | 20 medium | the same `titleLarge` | 22 regular |
| `TextField` value | 16 | `labelStyle` → `bodyLarge` | 16 — already right |
| `TextField` helper | 12 | `helperStyle` → `bodySmall` | 12 — already right |
| `Breadcrumb` | 15 | — argued | `bodyMedium`, 14 |
| `Kanban` card | 15 | — argued | `bodyLarge`, 16 |
| `Kanban` column title | 16 | — argued | `titleMedium`, 16 medium |
| `ErrorSummary` bullet | 13 | — argued | `bodySmall`, 12 |
| `ErrorSummary` heading | 14 | — argued | `titleSmall`, 14 medium |

The three argued ones are argued **from the reference's own vocabulary**, not from taste: a
breadcrumb is a secondary line above the page's own content; a board's card is a small piece
of content read on its own, which is the step a list tile's title takes; a summary bullet
restates an error already shown under its field, and the reference sets that helper line in
`bodySmall`.

The time picker's **preview** is the fourth of that kind. The reference puts an editable pair
of fields at `displayMedium` — 45 px — where this widget shows one read-only line above two
grids. It takes the heading step that line actually is.

## The app bar's title had the weight wrong, and a test that could not see it

`const TITLE_SIZE: f32 = 22.0` with `.weight(FontWeight::Medium)`. The size was right and
the weight was not: `titleLarge` is regular.

There *was* a test. It read:

```rust
assert_eq!(inherited, TITLE_SIZE, "a title that chose nothing wears the bar's type");
```

That compares the title against the constant the title came from. It is a tautology, and it
would have passed at any value. It asserts against `theme.text.title_large.size` now, and
`NavigationBar` — whose title was 20 px medium, **both halves wrong** — gained the first test
it has ever had for its default.

## A boolean beside a value

`AppBar` carried `title_style: TextStyle` next to `title_style_default: bool`, because a
non-optional style cannot say "nobody set me". That is the shape milestone 402 removed
elsewhere — "a `Chosen` record of booleans beside a `Text`'s style" — and it survived here.
It is `Option<TextStyle>` now, and the flag is gone.

## One more survivor of milestone 412

`TextField::sub_block` computed `frus_text::line_height(FIELD_SUB_SIZE)` while `sub_style()`
sat two methods away. Written against a bare constant rather than a style — the formulation
that sweep could not find, and the second one this pair of milestones has turned up.

## What is deliberately not done

**`TextField`'s type does not read the theme**, and it is the one widget here that keeps a
constant on purpose.

Its sizes are already the reference's, and a caller can already say `.size(…)`. The missing
term is the theme's — and wiring it in would break something worse. `TextField::layout` is
what places the caret and answers a hit-test, and it is reached from `on_key` and from the
pointer path, where **there is no theme**. A field whose glyphs came from the theme and whose
caret came from a constant is the milestone-406 bug rebuilt by hand: two numbers for one
thing, and the reader who most needed it right is exactly the one who gets it wrong.

Making it right means giving the field a resolved style it can carry — through
`build_themed`, which does see the theme — and that is a design change, not a mapping. It is
recorded as its own step rather than half-wired here.

## Verification

Eleven goldens moved and every one was read. The app bar's title is visibly lighter, the
slider's bubble wider, a breadcrumb a shade smaller, a summary's heading finally reads as a
heading. No layout broke: the time picker's cells grew and its box grew with them, because
milestone 413 had already made that box ask its cells.
