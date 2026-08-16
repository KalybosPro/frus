# Milestone 311 — A tab bar, not a row of buttons

`Tabs` was a `Flex::row` of `Button`s: the selected tab a filled primary button, the others
bordered secondary ones, six pixels apart, above a twelve-pixel gap and the panel.

Nothing about that is a tab bar. A tab bar is labels on the surface with a **sliding
indicator** under the selected one and a **hairline** separating the bar from what it
labels; the reference has had exactly that, in two variants, for as long as it has had tabs.
Buttons say "press me and something will happen elsewhere"; tabs say "this is the part of
this screen you are looking at". Using one for the other is not a styling difference.

It also had two builders — `new` and `tab` — so an application could not change a single
thing about it. That is the same finding as milestones 307 and 308, and the same fix.

## What the reference actually specifies

Read out of `tabs.dart` rather than remembered:

| | primary | secondary |
|---|---|---|
| indicator width | the **label** | the whole **tab** |
| indicator weight | 3.0 | 2.0 |
| indicator corners | rounded, radius = weight | square |
| selected label | `primary` | `on_surface` |
| unselected label | `on_surface_variant` | `on_surface_variant` |

with `_kTabHeight = 46.0` for a text-only tab, `kTabLabelPadding` 16 either side, and a
divider of `outline_variant` at `1.0` — the bar's height being the tabs' plus the
indicator's weight, since the indicator sits below them and not inside them.

All of it is here, all of it overridable: `variant`, `indicator_color`, `indicator_weight`,
`label_color`, `unselected_label_color`, `label_style`, `divider_color`, `divider_height`,
`label_padding`, `tab_height` — and `TabsTheme` carries the same ten, so an application can
say it once.

## The indicator slides

The selected index is handed to the runtime through `Widget::anim_target`, which means the
bar is painted a **fractional** index: 1.4 while the selection is on its way from the second
tab to the third. The indicator's centre *and its width* are interpolated between the two,
so moving from a short label to a long one grows it on the way. That is one line of state
and no controller, because the runtime already tweens per-widget values — the same mechanism
a `Switch` uses to slide its thumb.

The tabs share the bar's width in equal parts (a zero flex basis and equal growth), which is
what the reference does for a bar that is not scrollable. Sizing them to their labels
instead would move every tab whenever one was renamed.

## What that turned up

A widget whose appearance comes from `anim_target` was drawn **at zero** in any frame the
runtime had not advanced — an isolated `build_ui`, a test, a first frame. The runtime's own
rule is the opposite, and documented: *a widget seen for the first time adopts its target
with no transition*. The walk was not applying it, because `Status::value` read
`runtime.value(id)`, whose default is `0.0`.

So a `Switch` rendered on its own has always been drawn **off** however it was configured,
and the tab indicator would have sat under the first tab. The goldens hid it: their harness
settles the runtime before rendering, which is right for a golden and wrong as the only way
to get a correct frame. `Status::value` now falls back to the widget's own target.

## Verification

1029 tests (10 new), clippy silent, rustdoc clean, and **one golden re-blessed** —
`navigation_pickers`, which is the point of the milestone: three labels with the selected
one in the accent, a rounded indicator under it, and a hairline across the bar. The picture
was read before it was accepted.

The ten tests pin what the widget claims: the indicator is under the selected tab; a primary
indicator is as wide as its label and a secondary one as wide as its tab; the hairline runs
the whole bar and the indicator sits on it; the bar is the tabs plus the indicator;
`caller ?? theme ?? framework` on the one measurement that reaches layout; a tab announces
itself as a tab; and — the trap in a widget that assembles its children as it goes — a
builder called *after* `.tab(…)` still reaches the bar.

## Left

- **No scrollable bar.** The reference offers `isScrollable`, where tabs take their natural
  width and the bar scrolls, with a 52 px start offset. Everything here assumes tabs that
  share the width.
- **No icons in a tab**, so no `_kTextAndIconTabHeight` (72) either.
- **No bar without a panel.** `AppBar.bottom: TabBar` is a common shape and this cannot
  express it: `Tabs` insists on content for each tab. A first attempt at a `bar()` that
  threw the panels away was written and then deleted — it made the caller build content in
  order to discard it, which is not an API.
- **The panel does not slide.** Selecting a tab swaps the panel outright, where the
  reference's `TabBarView` moves it sideways.
