# Milestone 446 — A snack bar has one behaviour where the reference has two

Every notification this framework drew was rounded, held its text in by 16, and kept
nothing clear of the page except whatever its host happened to impose. The reference has
**two** arrangements, and it chooses between them with one question.

## `SnackBarBehavior`

`snack_bar_theme.dart:27`. Two values, and the documentation is about **position**:

- **`fixed`** — fixed to the bottom of the scaffold. It shows above a bottom navigation
  bar, and it pushes the other non-fixed things in the scaffold up: a floating action
  button moves out of its way.
- **`floating`** — shown *above* the other widgets in the scaffold, the navigation bar and
  the button included. It moves nothing; it covers.

Everything else follows from that. A bar flush against three edges of the page and a bar
hovering over it are not the same object with a different colour:

| | `fixed` | `floating` | here, before |
|---|---|---|---|
| corner | **none** (`snack_bar.dart:798`) | 4 (`:983`) | 4, always |
| horizontal padding | 24 (`:687`) | 16 | 16, always |
| kept clear of the page | nothing | `insetPadding` = 15, 5, 15, 10 (`:989`) | nothing |
| its own width | forbidden (`:678`) | allowed (`:329`) | — |
| the default | **this one** (`:986`) | | — |

The corner is the one worth stating plainly: the reference passes a `shape` at all only
when the bar is floating, and a `Material` with no shape has square corners. A bar sitting
flush against three edges has nothing to round, and rounding it leaves four slivers of
page showing through the corners — which is exactly what this drew.

## Three resolutions, one question

```rust
pub enum SnackBarBehavior { Fixed, Floating }
```

`SnackBar::behavior(…)`, then `SnackBarTheme::behavior`, then `Fixed` — the chain this
framework uses everywhere. The padding, the corner, the margin and the width are **not**
separately settable per behaviour: they are what the behaviour means. A caller who wants
one of them differently still can, through the theme or the builder, but the default is
one decision rather than four.

`SnackBar::width` and `SnackBar::margin` are floating's alone, as in the reference
(`snack_bar.dart:678`, which asserts). A builder cannot assert usefully, so a fixed bar
**ignores** both: a width would contradict the behaviour rather than override it.

## The inset belongs to the bar

`ScaffoldMessenger` imposed 16 on everything it held. That is not the host's to decide —
the reference puts the margin inside the bar (`snack_bar.dart:823`), which is the only
place that knows whether there should be one. A floating bar was getting 16 **plus** its
own; a fixed bar was getting 16 where it should get none.

So the host's default padding is now **nothing**, and `ScaffoldMessenger::padding` is
still there for a caller who wants a layer of their own arrangement.

## The tests

- `a_bar_is_fixed_by_default_and_has_no_corners` — the default, the square, and the 24.
- `a_floating_bar_rounds_and_keeps_clear_of_the_page` — the 4, the 16, the `insetPadding`,
  and a caller's own margin over it.
- `a_width_is_a_floating_bar_s_alone` — both ways round.
- `a_theme_places_every_bar_at_once` — and one bar still answering for itself.

All four fail with the single behaviour restored, and so does
`a_bar_with_a_cross_makes_room_for_it`, which measures the room the cross gets from the
padding it now follows.

## Still open

**A snack bar still sizes itself to its text.** The reference never does: a fixed bar
spans the page, and a floating one spans the room it is given less its margin
(`snack_bar.dart:734`). Making it span means the bar has to stretch on its host's cross
axis, and the fade wrapper between them (`AnimatedOpacity` forwards its inner
`Container`'s style, not its child's) shrink-wraps in between. That is a question about
what a notification layer *is* here — the reference's messenger shows one bar across the
bottom of the scaffold, and this one is a corner that stacks several — so it is a
milestone of its own rather than half of this one.
