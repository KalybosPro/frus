# Milestone 615 — Widgets follow the theme's platform

## Objective

Milestone 614 gave the theme a `platform`. Four widgets still decided by the build
target, at compile time, where the reference reads `Theme.of(context).platform`:

| Widget | Reference | frus until now |
|---|---|---|
| App bar title centring | `app_bar.dart:805`: iOS, macOS centre with fewer than two actions | `cfg(ios, macos)` |
| Back button glyph | `action_buttons.dart:132`: `arrow_back` on the web; `arrow_back_ios_new_rounded` on iOS, macOS; `arrow_back` elsewhere | `cfg(ios, macos)`: **`chevron_left`** |
| Search view full screen | `search_anchor.dart:554`: iOS, Android, **Fuchsia** | `cfg(ios, android)` |
| Reorderable list | `reorderable_list.dart:383`: drag handles on Linux, Windows, macOS; a hold on Android, **Fuchsia**, iOS | `cfg(android, ios)` |

Reading the reference's source for each one, rather than the frus comments that claimed to
follow it, found three divergences: the back glyph was a chevron where the reference draws
the iOS arrow, and Fuchsia was a desktop in two of the four.

## What was done

Each of the four reads `theme.platform`, with the reference's table:

- `platform_centers_title(platform, actions)` takes the platform, and the bar passes the
  theme's.
- The back glyph is chosen from the theme's platform. On the web it is always the arrow, as
  the reference's is (`kIsWeb`). The reference's iOS arrow is the rounded one, whose glyphs
  come with the `icons-rounded` feature; without it the filled iOS arrow stands in.
- `SearchAnchor` decides full screen from the theme it is built under; `full_screen`
  overrules it, as `isFullScreen` does.
- `ReorderableList` keeps `grab` unset unless told, and resolves it from the theme's
  platform. The list records the platform where it is laid out (`style_themed`), because
  the gesture hooks run outside layout; the grip resolves from the theme it is built
  under.

**Breaking**: `platform_centers_title` takes a `TargetPlatform`, and `ReorderGrab` no longer
implements `Default` (the default depends on a theme).

## Which platform, where

The reference does not read the theme everywhere. Questions about the real system read
`defaultTargetPlatform`: whether a route's name is announced (`app_bar.dart:1072`), what a
drawer is called to a screen reader (`drawer.dart:253`), whether the system has a back
button (`drawer.dart:684`). A theme set to iOS on an Android phone changes how the
application looks and moves, not what TalkBack hears. frus has none of these
platform-dependent announcements yet; when it does, they read `default_target_platform()`,
not the theme.

The scroll behaviour — physics, scrollbars, overscroll indicator, fling velocity — is not
in this milestone. The reference decides those in a `ScrollBehavior`, installed by a
`ScrollConfiguration`, whose `getPlatform` the application layer points at the theme
(`app.dart:854`). frus has no such object yet; its port is the next milestone.

## Tests

- `the_title_follows_the_platform_until_it_is_told_otherwise`: the reference's table, for
  every platform, on any build.
- `the_title_follows_the_theme_s_platform`: an iOS theme centres the title wherever the test
  runs, an Android one does not, and two actions put it flush.
- `the_back_arrow_follows_the_theme_s_platform`, `the_view_takes_the_screen_where_the_theme_s_platform_does`,
  `an_untold_list_follows_the_theme_s_platform`: each table, for every platform, and an
  explicit setting overruling it.

Mutation testing: eight mutants. Seven killed by the tests — each table's row changed
(Apple's action count, Fuchsia's full screen, Fuchsia's hold) and each widget handed the
build's platform instead of the theme's — and one by the compiler: an Apple platform
dropped from the back glyph's match leaves it non-exhaustive. A ninth first survived: the
list and its grips both recorded the theme's platform, so removing either changed nothing.
The grip is now the one place it is recorded, and removing it is caught.
