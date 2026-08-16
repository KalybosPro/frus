# Milestone 309 — The middle term

Three milestones in a row ended on the same sentence.

> *306:* "the theme carries no per-widget defaults … the same gap `splashFactory` ran
> into."
> *307:* "an application can override one divider, but cannot say *every divider in this
> app is 1 px* once."
> *308:* "an application can set one card's elevation, not every card's."

Each time the honest thing was to write it down and move on, because it is a design and
not a patch. Three times is enough. This milestone is the design.

## What was missing

The reference resolves nearly every property through a chain of three:

```dart
centerTitle ?? appBarTheme.centerTitle ?? platformCenter()
width       ?? drawerTheme.width       ?? _kWidth
elevation   ?? cardTheme.elevation     ?? defaults.elevation
```

Here the first and third terms existed and the middle one did not. An application that
wanted flat cards throughout had two options: write `.elevation(0.0)` at every call
site, or wrap `Card` in its own type and use that everywhere. Both are the framework
failing to do its job.

`Theme` now carries `widgets: WidgetThemes` — `CardTheme`, `DividerTheme`,
`DrawerTheme`, `InkTheme`, one `Option` per builder the widget already has:

```rust
let mut theme = Theme::dark();
theme.widgets.card.elevation = Some(0.0);   // a flat application
theme.widgets.divider.height = Some(1.0);   // hairlines, flush
theme.widgets.ink.color = Some(accent.fade(0.2));
```

Every field is an `Option` and `None` means *the framework's own default*, so a theme
that sets nothing behaves exactly as if there were none. That is not a nicety: it is
what keeps a widget's built-in default reachable instead of shadowed by a theme that
happened to be constructed.

## The part that was not obvious

**A theme has to reach layout, not only paint.**

`Widget::paint` takes a `&Theme`. `Widget::style` does not, and cannot be given one
without touching all ninety-odd implementations. The tempting answer was to theme only
the paint-time properties — colours, elevation, thickness — and call it done.

That answer is wrong for the setting people actually want. A divider's **height** is a
layout property. So is a card's **margin**, and a drawer's **width**. A theme that
stopped at `paint` could recolour a separator but not make one thin, which is the one
thing an application asks a divider theme for.

The way through is a second method with a default:

```rust
fn style_themed(&self, _theme: &Theme) -> Style {
    self.style()
}
```

Ninety widgets inherit that default and change nothing. Four override it. The layout
walk calls `style_themed`, and the only cost is one more argument threaded through
`effective_style`, `build_layout`, `natural_size`, and the layout cache.

**Transparent wrappers must forward it.** `Keyed`, `Responsive`, `DataTable` and the
implicit-animation wrappers all forward `style`; each now forwards `style_themed` too.
This is the trap this repository already has a name for — a wrapper that forwards *most*
of the structure — and it would have shown up as "the theme works, except inside a
`Keyed`", which is the kind of bug that takes an afternoon.

## The cache

The relayout cache keys each layout root on a fingerprint of its subtree's **effective**
styles. Since `effective_style` now consults the theme, the fingerprint follows: swap the
theme, the hash changes, the cache misses, the geometry is recomputed. Swap it back and
the old entry is still valid.

That is correct by construction rather than by care, which is the good kind — but it is
also the failure that would have been invisible, because it only appears on a theme
*change* and every screenshot is taken under one theme. There is a test for exactly it:
the same `Runtime`, and therefore the same cache, built twice under two themes.

## Verification

Seven tests, each pinned to one link of the chain:

- the theme reaches **layout** — a themed divider's box is shorter, and the line inside
  it is not;
- **caller > theme > framework**, one assertion per link, on a margin so that both
  halves of the resolution run;
- a theme can change **what a widget is**, not only a number: `card.variant` makes an
  untold `Card::new()` flat, shadow and all;
- the **ink** colour reaches a widget that computes its own — a `Button` derives its
  splash from its own `on` colour, which is the right default and the wrong thing to
  insist on;
- a **theme change invalidates the layout cache**;
- the framework's defaults are still reachable through an empty theme;
- and an empty theme equals no theme.

1010 workspace tests, `clippy` silent on every target, `rustdoc` clean under
`--all-features`, and **not one golden moved** — which is the result to want here: the
defaults are unchanged, so a frame built under an empty theme must be the same frame it
was yesterday. A golden that had shifted would have meant the chain leaked a value
nobody asked for.

Three of the seven failed the first time, all for the same reason: they built a `Divider` and
a `Card` as the **root** widget, where an `Auto` width resolves to nothing and a margin
has nothing to be a margin against. A widget tested outside a parent is not being tested
in the situation it ships in. They build inside a sized `Flex` now.

## Left

- **The rest of the widgets.** Four are wired. `Button`, `Chip`, `Tabs`, `TextInput`,
  `AppBar` and the others still hold their own literals — and `AppBar::center_title`,
  from milestone 306, resolves `caller ?? platform` with the theme's term still missing
  from the middle.
- **Themes do not compose.** The reference has `Theme` widgets that override an ambient
  theme for a subtree; here there is one theme for the frame. A dark card on a light
  page is not expressible.
- **Nothing validates a theme.** Setting `divider.thickness` larger than
  `divider.height` gives a line clamped to its box, silently. The reference does not
  check either, so this is parity rather than a defect — but it is worth knowing.
