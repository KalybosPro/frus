# Milestone 469 — A field with no container, inside a container

`SearchBar` — the raised pill an application searches from (`search_anchor.dart:1404`).
It did not exist, and the way to fake it was a `TextField` with a stadium radius, which is
a form field wearing a shadow rather than a search bar.

The difference is not decoration. A form field is *sunk into* the page: it has a container
of its own, it floats a label into that container's border, it sits in a column of other
fields. A search bar goes *over* the content it filters: it is raised, it has no label, and
it holds a row — a menu button, the words, a clear cross — of which the words are only the
middle.

## The thing that was actually missing

```dart
decoration: InputDecoration(hintText: …).applyDefaults(
  InputDecorationThemeData(
    enabledBorder: InputBorder.none,
    border: InputBorder.none,
    focusedBorder: InputBorder.none,
    contentPadding: EdgeInsets.zero,
    isDense: true,
  ),
),
```

Five lines of the reference's search bar exist to say **this field has no container**. A
field that keeps its own box inside a pill is two containers deep, with the outer one's
corners cut by the inner one's — and there was no way to say it here. `TextFieldStyle` can
make a container transparent, but that is eight overrides written out at every call site to
express what the reference says in one word.

So `TextFieldVariant::None` is new, with `TextField::borderless()`. It lays out like
`Filled` — the label floats inside rather than notching a border, there being no border to
notch — and paints nothing behind the content. The whole change is one arm:

```rust
if self.is_borderless() {
    // **Nothing.** The container belongs to whatever this field was put inside.
} else if self.is_outlined() {
```

`the_field_inside_paints_no_container` counts the rectangles: a filled field draws some, a
borderless one draws none. It is the widget the rest of this milestone is built on, and it
is the widget an inline table editor and a chip's input field will want next.

## The hint's type has five rungs

Everything here resolves `caller ?? theme ?? framework`. The hint's type does not:

1. the caller's hint style
2. the theme's hint style
3. **the caller's *value* style**
4. **the theme's value style**
5. `bodyLarge` in `on_surface_variant`

The two in the middle are the reference's (`search_anchor.dart:1727`) and they are worth
copying. A bar told to say its value in one type would otherwise say its hint in another,
which nobody notices until the field is empty — and the field is empty exactly when the
hint is the only thing showing.

`the_hint_takes_the_values_type_before_its_own_default` holds all three cases; removing the
two middle rungs fails it.

## Eight, applied twice

`padding` defaults to eight either side, and the reference applies it **twice**: once
around the row and once around the field inside it (`search_anchor.dart:1783` and `:1790`).
So the gap from the bar's edge to a leading icon is eight and the gap from that icon to the
first letter is sixteen. One number, two places, and the asymmetry is the point — an icon
wants to sit near the edge and text does not want to sit near an icon.

## A disabled bar fades

This framework has a rule and this is the one place it gives way. A disabled control here
[flattens rather than fades](../crates/frus-widgets/src/disabled.rs): every variant
collapses to the same two colours, so *unavailable* reads as unavailable rather than as a
quieter version of whatever the control was.

A raised pill has nothing to flatten to. Its unavailability *is* the raising going away —
and the reference agrees, dimming the whole thing, shadow and icons included, at 38 %
(`search_anchor.dart:48`). The exception is written down where the constant is, because an
exception nobody wrote down is a bug the next person fixes.

The four other halves of the disabled contract are kept: the press goes nowhere, the tab
skips it, it is announced as unavailable, and **nothing lights on it** — a state layer is
the promise of an interaction and there is none.

## Read-only is not disabled

`read_only(true)` with `on_tap` is the bar that only *looks* like a field: pressing it opens
the thing that really searches. It still lights, still answers, still reads as available,
because it is — which is a different fact from `enabled(false)` and had to be a different
builder.

A press anywhere on the pill counts, not just on the field. The reference wraps the whole
thing in an ink well (`search_anchor.dart:1772`), so the eight pixels beside a leading icon
are not dead.

## `SearchBarTheme`

Twelve fields. Every one of them is a `WidgetStateProperty` in the reference, resolved
against the bar's state; here they are plain values. That is not a shortcut for eleven of
them — a fill or a shape or a padding that changes on hover is a thing the reference *can*
say and its own defaults never do. The twelfth is the highlight, and this framework already
answers it: `Theme::state_layer` is one rule from the ground toward the ink, resolved
opaquely, covering hover, focus and press at once.

## What the picture showed

Three bars: one carrying a query with a menu and a cross, one empty with a hint, one
disabled. The pill's corners are the pill's and there is no second box inside them, which
is the whole claim of the milestone and the one thing a count of rectangles cannot show.

## What this turned up

Two gaps, both on the roadmap:

- **A field's hint cannot have its own type step**, only its own colour. The placeholder is
  drawn at the value's size here, so `hint_style` can carry a colour but not a size and the
  five-rung resolution above is doing less than it says on one axis.
- **A field cannot ask for the keyboard when it appears.** The reference's `autofocus` has
  no equivalent, so `SearchBar` has no `auto_focus` builder rather than a builder that
  quietly does nothing.

And one widget: `SearchAnchor`, the view a search bar opens — the suggestions under it on a
desktop, the full screen it becomes on a phone. `SearchBar` is the anchor without the
anchor, which is exactly how the reference ships it too (they are separate widgets and a
bar is useful alone).

## Verification

`cargo fmt`, clippy across the workspace with all targets and all features: silent.
`RUSTDOCFLAGS='-D warnings' cargo doc`: silent. **1280 unit tests**, all green — nine of
them new, three checked by breaking what they guard and watching them fail. Goldens
**91 + 33 + 14**, one picture added and none moved.
