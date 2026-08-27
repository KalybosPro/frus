# Milestone 430 — The role was written for this widget and this widget never used it

`ColorScheme` has carried `inverse_surface` and `on_inverse_surface` for a long time, with
this beside them:

> An inverted surface (toasts and snackbars that stand out from the background).

The snack bar was a **card on `surface`**, with a border round it and a coloured stripe down
its left edge. It has never touched the pair named after it.

## What the reference's is

| | this crate, before | the reference |
|---|---|---|
| surface | `surface`, with a 1px border | `inverseSurface`, no border (`snack_bar.dart:949`) |
| message | `on_surface` | `onInverseSurface` |
| action | `primary` | `inversePrimary` (`:965`) |
| corner | the theme's, 10 | 4 (`:983`) |
| elevation | a hand-picked blur of 8 | 6 (`:980`) |

The reason `inverse_primary` exists at all is this widget. `primary` on an inverted surface
is the one pairing the scheme guarantees nothing about — every other `X`/`on_X` pair is
built to contrast, and *accent on inverted background* is not one of them. The test says so
directly: it asserts the action's colour **is not** `theme.primary`, because a version that
reached for the page's accent would have looked plausible and been unreadable on the seed
where the two collide.

The border goes with the surface. An inverted bar is already distinct from the page, and a
rule round it would be edging a thing that is separate already.

## The three kinds, and the one colour that cannot be a role

`SnackBarKind` — info, success, error — is this crate's own; the reference's bar has one
look. Its three colours were **literals**: `theme.primary`, `Color::rgb8(70, 190, 120)` and
`Color::rgb8(210, 96, 96)`, where the scheme's own documentation says

> Widgets reference roles, never literal colors.

Two of the three had a role waiting. Info is `inverse_primary` — the accent as it reads on
the surface it is actually drawn on. Error is `scheme.error`.

**Success has none.** Material 3 carries `error` and nothing that means *it worked*. A
framework that shipped no colour would be shipping no success variant, so one stays — named
`SUCCESS_ACCENT`, documented as this crate's own rather than the reference's, and
`SnackBarTheme::success_color` is where an application replaces it. Saying that out loud is
the point: an unexplained literal reads like an oversight, and this one is a decision.

## The state layer under the action

The action's hover tint was mixed from `theme.surface` toward `theme.primary` — the page's
ground and the page's accent, on a button that stands on neither. A state layer mixed from
the wrong ground is not a subtle error: it is a patch of an unrelated colour appearing under
the pointer. It is mixed from the bar's own surface toward the action's own colour now, and
both follow whatever the caller or the theme said, so an application that recolours the bar
gets a hover that still belongs to it.

## Overridable, as the rest of the crate is

`SnackBarTheme` gained `background_color`, `text_color`, `action_text_color`,
`accent_color`, `success_color`, `radius` and `elevation`; `SnackBar` gained
`background(…)`, `text_color(…)`, `action_text_color(…)` and `accent(…)`. Before this the
bar's surface, its text colour and its three accents were all unreachable — not themed
defaults that could be overridden, simply hard-coded.

## The tests

- `a_notification_stands_out_rather_than_sitting_on_the_page` — the bar's surface is
  `inverse_surface`, the message is `on_inverse_surface`, and the two surfaces are
  **tellable apart**, without which the first assertion could pass on a scheme where they
  coincided.
- `the_action_takes_the_inverted_accent` — `inverse_primary`, and *not* `theme.primary`.
- `the_kinds_name_a_role_wherever_one_exists` — the three defaults, then the theme
  replacing the one without a role, then the caller outranking all three.

Reverting the surface to `theme.surface` fails the first two, which is the shape of the
whole change.

## Still open

The reference's snack bar also has a close icon (`snack_bar.dart:992`, `:995`), a
`SnackBarBehavior.floating` that insets the bar from the window's edges (`:989`), and an
`actionOverflowThreshold` (`:998`) that drops a long action onto its own line. And the
nav rail draws its badge with a literal of its own — `Color::rgb(0.90, 0.24, 0.24)` in
`navrail.rs` — where the `Badge` widget beside it already takes the scheme's `error`
through its theme. That is the same rule broken in the same way, one file over.
