# Milestone 426 — Two names for five rungs

The scheme carried two container roles, `surface_container` and `surface_container_high`.
The reference carries five, and names them together in a single sentence
(`color_scheme.dart:1264`): `surfaceContainerLowest`, `surfaceContainerLow`,
`surfaceContainer`, `surfaceContainerHigh`, `surfaceContainerHighest`.

Seven call sites here reach for a container role, and two names were serving all of
them.

| widget | the reference's role | what it took |
|---|---|---|
| elevated `Card` (`card.dart:313`) | `surfaceContainerLow` | `surface_container` |
| filled `Card` (`:348`) | `surfaceContainerHighest` | `surface_container_high` |
| elevated `Button` (`elevated_button.dart:534`) | `surfaceContainerLow` | `surface_container` |
| `Menu` (`popup_menu.dart:1858`) | `surfaceContainer` | `surface_container_high` |
| filled `TextField` (`input_decorator.dart:5968`) | `surfaceContainerHighest` | `surface_container_high` |
| `MaterialBanner` (`banner.dart:510`) | `surfaceContainerLow` | `surface_container` |
| `AlertDialog` (`dialog.dart:1979`) | `surfaceContainerHigh` | `surface_container_high` ✓ |

Only the dialog was on the rung the reference names. Two of the compromises were written
down in the code — `Card`'s and `MaterialBanner`'s, in the same words — and that repetition
is what said the missing tone belonged in the scheme rather than in each widget.

## Why the ladder cannot simply take the spec's tones

The scheme has one recorded departure from Material 3: `surface` sits apart from
`background`, at tones 100/98 in light and 12/6 in dark, because a card here lays a surface
over the background and the 2023 spec conflates the two.

That departure rules out the spec's absolute tones. In a dark scheme the spec's three lower
rungs are tones 4, 10 and 12 — **all at or below this scheme's surface of 12**. A container
role means *more emphasis than the surface*; painted at those tones a menu would come out
darker than the page it floats over, which is the opposite of what the role promises.

So the ladder is anchored on this scheme's own surface. The two rungs that already existed
keep their exact values — nothing that was right moves — and the other three are placed at
**the reference's own tonal steps**, measured from them:

| | lowest | low | container | high | highest |
|---|---|---|---|---|---|
| reference, light | 100 | 96 | 94 | 92 | 90 |
| steps | | −4 | −2 | −2 | −2 |
| **frus, light** | **100** | **98** | 96 | 94 | **92** |
| reference, dark | 4 | 10 | 12 | 17 | 22 |
| steps | | +6 | +2 | +5 | +5 |
| **frus, dark** | **9** | **15** | 17 | 22 | **27** |

Every rung therefore stands off this surface by what it stands off the reference's. Two
details fall out of that and are worth naming rather than hiding:

- In light the top rung lands on tone 100, which is where this scheme's `surface` already
  is. `surface_container_lowest` and `surface` are the same white. That is the departure
  showing through, and it is also true of the reference itself, whose `surfaceContainerLowest`
  is tone 100 too.
- In dark, `surface_container_lowest` (9) is **darker** than the surface (12). That is not a
  mistake: "lowest" means the least emphasis, not the least light, and the reference's own
  lowest rung sits below its surface in dark for the same reason.

The hand-written schemes were given the same tones, read off the gradient the two existing
rungs already describe, so the ladder is one family of colours and not two.

## The test

`the_container_ladder_climbs_in_one_direction` walks the five rungs of four schemes — both
hand-written ones and a seeded pair — and asserts each is at least a tone of emphasis above
the one below it, in whichever direction that scheme's brightness sends them. A rung out of
order, or two rungs landing on the same tone, is two widgets that cannot be told apart while
their roles say they should be. Sixteen steps checked.

The direction is derived from the scheme rather than written down per scheme, so a scheme
generated from any seed is held to the same rule.

## Still open

The surface family also has `surfaceDim` and `surfaceBright` — "always the darkest" and
"always the lightest" — which the reference lists beside the containers
(`color_scheme.dart:104`). No widget here asks for either yet, and a role nothing paints
with is a value nothing can check, so they are recorded rather than added.

Widgets still filled from the flat `surface` where the reference names a container role:
`Drawer` and the bottom sheet (`surfaceContainerLow`), the bottom app bar and the navigation
bar (`surfaceContainer`), the autocomplete and dropdown panels (`surfaceContainer`).

And the scheme is still short of whole role families the reference carries: the tertiary
five, `errorContainer` / `onErrorContainer`, `inversePrimary`, `surfaceTint`, and the
`*Fixed` set.
