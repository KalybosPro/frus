# Milestone 354 — `Flexible` by name

The loose fit — *at most* your share of what is left, and less if that is all you want —
has existed since milestone 334 as `Expanded::new(child).loose()`. It is tested in both
directions, it is what `flexible()` in the DSL builds, and it behaves exactly as the
reference's does.

It simply is not called what the reference calls it. There, `Flexible` is the widget and
`Expanded` is its tight subclass:

```dart
class Expanded extends Flexible {
  const Expanded({super.key, super.flex, required super.child})
    : super(fit: FlexFit.tight);
}
```

so an application ported from it types `Flexible(fit: FlexFit.loose, flex: 2, child: …)`
and finds nothing. A name is not a small thing when the whole point of following a
reference is that someone arriving from it can type what they already know.

## What it is

`Flexible` and `FlexFit`, with the reference's defaults on both widgets:

| | takes its whole share | may take less |
|---|---|---|
| `Expanded` | `Expanded::new(child)` | `Expanded::new(child).loose()` |
| `Flexible` | `Flexible::new(child).tight()` | `Flexible::new(child)` |

`Flexible::fit(FlexFit::Tight)` takes the fit as a value, for code being ported from the
`fit:` argument, and says the same thing as `.tight()`.

## One box, not two

The interesting part is what was *not* done. `Flexible` could have been a second wrapper
with a second copy of the three properties that make a flex item — a basis, a grow
factor, and the lifted automatic minimum that makes `flex: 1` alone a no-op — and it
would have passed every test.

It would also have been the second place for the next fix to miss, and this particular
box has already cost three milestones once (333/334) precisely because the interaction
between those three properties is not obvious. So the three of them live in one
`flex_item(base, flex, fit)`, which both wrappers call and neither owns, and the tests
assert what that means: the same row, built both ways, lays out to the same pixel in both
fits.

`Flexible` is otherwise a transparent wrapper like `Expanded`, through the same macro —
which is what makes it forward the structural questions rather than whatever its author
remembered.

## Left

`shrink()` is still on `Flex` and `Container` alone while ten other widgets carry
`flex()`. Milestone 349 made that matter much less by turning the default around —
nothing is squeezed unless it says so, which is what those ten wanted `shrink(0.0)` for
in the first place — so what remains is the ability to *opt in*, and no screen has needed
it yet.
