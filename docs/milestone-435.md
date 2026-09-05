# Milestone 435 — Three bands, two answers

`NavScaffold` exists to do one thing: read the size class and place the navigation where
that class calls for. There are three size classes. It had two answers.

```rust
compact: class == SizeClass::Compact,
```

That is the whole of what it kept. `Medium` and `Expanded` were the same case, so a
1400-pixel desktop window got the same 80-pixel glyph-only rail as a portrait tablet — the
narrower of the two presentations, on the wider of the two windows, for no reason except
that a boolean cannot count to three.

Nothing in the shell was wrong until milestone 433. There was only one rail to give, so
giving it to both bands was the only thing to do. The moment the rail learned its extended
form, the missing third answer became a missing feature.

## What the third band gets

The reference has no adaptive shell in the framework, but it ships an application that is
one, and it makes the choice explicitly:

```dart
// reply/adaptive_nav.dart:95
if (isDesktop) {
  return _DesktopNav(extended: !isTablet, …);
} else {
  return _MobileNav(…);
}
```

with `isDesktop = windowType >= medium` and `isTablet = windowType == medium`
(`layout/adaptive.dart:18`, `:23`). Three bands, three presentations:

| class | navigation |
|---|---|
| `Compact` | a bottom bar |
| `Medium` | a rail, glyphs alone |
| `Expanded` | an extended rail, labels beside the glyphs |

**This changes what an existing expanded window looks like.** A `NavScaffold` at
`SizeClass::Expanded` now hands 256 pixels to the rail where it handed 80, and the body is
176 narrower. That is deliberate, and it is the same reasoning as milestone 432's: the
default is the reference's, and a shell whose bands do not mean what the guidance says they
mean is a shell whose behaviour has to be memorised rather than known.

## And the caller has the last word

The same door milestone 434 opened on `Scaffold`:

```rust
NavScaffold::new(class, selected, Msg::Go)
    .destination("★", "Home")
    .rail(|rail| rail.extended(false))   // a wide window that wants its 176 pixels back
    .body(content)
```

plus `nav_labels`, which reaches whichever of the two widgets the class chose. A wide window
is a reason to offer the room, not a reason to insist on it.

## A silent mistake made loud

`NavScaffold::body` is the builder that **finalises** the shell: it takes `on_select`,
drains the destinations and assembles the children. Everything else *describes* the
navigation, and anything said after `body` was silently dropped — `destination` included,
long before this milestone.

That is the worst kind of bug, because it looks like a property that does not work. The four
describing builders now assert, and name themselves:

```
destination() describes the navigation and has to come before body(), which builds it
```

## The tests

- `each_of_the_three_classes_gets_its_own_presentation` — a column at compact, an 80-wide
  rail at medium, a 256-wide one at expanded.
- `the_extended_rail_can_be_declined` — the door, through `.rail(…)`.
- `the_label_mode_reaches_the_widget_the_class_chose` — through the rail and through the bar,
  and saying nothing leaves each on its own default.
- `describing_the_navigation_after_the_body_is_refused` — a `should_panic` on the message
  above.

Run first against the code without the change: the presentation and label tests fail with
the third band forced back to the plain rail.

**And the golden moved on purpose.** `the_nav_scaffold_both_ways` was two panes because
there were two presentations; it is `the_nav_scaffold_three_ways` now, and the picture shows
the compact bar with labels under its glyphs, the medium rail with glyphs alone, and the
expanded rail with the labels beside them — the glyph in the same column in both rails, and
the indicator still a pill around the glyph rather than a bar across the row, which is
milestone 433's rule made visible.

## Still open

The rail switches presentation without animating between them, at both thresholds: the
reference drives its extended state with a controller and fades the labels in over the first
quarter (`navigation_rail.dart:605`, `:781`). A shell that crosses a breakpoint mid-drag
therefore jumps.
