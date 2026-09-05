# Milestone 418 — The bottom slot was padded from outside, so its surface stopped short

Milestone 417 split one switch into two at the top of the screen: the shell says what there
is to consume, the app bar consumes it. The bottom of the screen still worked the old way —
the `Scaffold` wrapped its navigation slot in a padding `Container` and handed the bar a box
that already ended above the gesture bar.

That is not where the reference puts the line, and the difference is one you can see.

## The colour is outside the safe area, in both bars

```dart
// navigation_bar.dart:285
Material(
  color: …,
  child: SafeArea(
    child: SizedBox(height: effectiveHeight, child: Row(…)),
  ),
)
```

```dart
// bottom_app_bar.dart:227
final material = Material(type: MaterialType.transparency, child: SafeArea(child: child));
return PhysicalShape(color: effectiveColor, child: material);
```

The surface is **outside**, the safe area **inside**. So a bar's background runs behind the
gesture bar and only its destinations are held clear of it. Padded from outside, the
background stops short of the screen's edge and a strip of the scaffold shows through
underneath — which is what frus drew.

And the shell's half of it, `scaffold.dart:3163`:

```dart
_addIfNonNull(
  children, widget.bottomNavigationBar, _ScaffoldSlot.bottomNavigationBar,
  removeLeftPadding: false,
  removeTopPadding: true,
  removeRightPadding: false,
  removeBottomPadding: false,
  maintainBottomViewPadding: !_resizeToAvoidBottomInset,
);
```

A **description** with the top intrusion removed and the bottom one left in. `bottom_clear`
is already the number that last argument arrives at — the scaffold worked it out a hundred
lines earlier, for the keyboard.

## What changed

- `Scaffold`'s `nav_pad` is a `MediaScope::tweak` rather than an `inset_pad`: top zero, the
  sides and the bottom as the shell resolved them.
- `BottomBar` grows by the bottom intrusion and pads its destinations by it, so the rule
  along its top edge and the surface behind it run the full width.
- `BottomAppBar` does the same, and its `paint` fills `bounds` — which now includes the
  intrusion, exactly as `PhysicalShape` outside a `SafeArea` does.

## The bug the change uncovered

```rust
bar.notched_at(fab_centre_x - insets.left, fab_size / 2.0)
```

The notch is cut in the **bar's own** coordinates, and the bar used to start at
`insets.left` because the padding put it there. It starts at zero now, so the subtraction
has to go. With no side intrusion the two readings agree and the mistake hides; a landscape
cutout is where it shows, and that is what the new test describes — cut at the old place,
the notch's nodes land a whole cutout away from the button they were cut for.

## What moved, and what did not

**No golden moved** (91 + 13 + 27), for the same reason as milestone 417: the golden scenes
describe no intrusions, so a bar consumes zero and the change is inert where there is no
gesture bar.

Three new tests, and each fails on the old arrangement:

- a bar's surface reaches the window's edge while its content stops above the gesture bar;
- a notch stays under its button beside a cutout;
- a `BottomBar` told about an intrusion grows by it, pads its destinations by it, and never
  consumes a top it was told nothing about.

One unrelated thing fixed in passing: `NavigationRail`'s doc line was the last French one
left in the crates.

## Still open

The other slots are still insetted from outside, and each is a step of its own:

- **the rail**, which the reference safe-areas on its leading edge, its top and its bottom
  (`navigation_rail.dart:553`) — and whose width the persistent footer's row arithmetic
  reads, so the two move together;
- **the footer**, which the reference already builds the right way round (decoration
  outside, safe area inside) but is still handed a padding by the shell;
- **the body**, which the reference does not inset at all: it hands it a description with
  the top removed when there is an app bar and the bottom removed when there is a bar below
  (`scaffold.dart:3029`), and a body that wants the notch avoided says so itself.
