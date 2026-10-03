# Milestone 616 — A scroll behaviour, in force for a subtree

## Objective

How a scrollable behaves — its physics, whether it draws a scrollbar, what it shows past an
edge, how it reads a fling — was decided in four places, each at compile time from the
build target: `ScrollPhysics::platform_default`, `Scrollbars::platform_default`,
`OverscrollIndicator::platform_default` and `VelocityTracker::platform_default`. An
application could pin three of them through three `Application` methods, for the whole
application at once and nowhere smaller. A theme set to iOS (milestones 614, 615) changed
the app bar and the back arrow and left every list scrolling the Android way.

The reference decides all four in one object, `ScrollBehavior`
(`scroll_configuration.dart:73`), answered from one platform (`getPlatform`), put in force
for a subtree by `ScrollConfiguration` (`:397`). Its base follows `defaultTargetPlatform`;
the application layer installs one whose `getPlatform` is the theme's (`app.dart:854`).
Each scrollable asks the behaviour where it stands (`scrollable.dart:619`) and lays its own
physics over the behaviour's (`:620`). This milestone ports that shape.

## What was done

- **`ScrollBehavior`** (`frus-widgets/src/scrollbehavior.rs`). `ScrollBehavior::new()` is the
  base: it follows `default_target_platform()` and glows past the edge on Android and
  Fuchsia. `ScrollBehavior::material()` is the application's: it follows `Theme::platform`
  and stretches on Android. The reference's `copyWith` is `with_scrollbars(bool)`,
  `with_overscroll(bool)`, `with_physics(ScrollPhysics)` and
  `with_platform(TargetPlatform)`. Its answers — `platform`, `scroll_physics`,
  `scrollbars`, `overscroll_indicator`, `velocity_tracker` — are the reference's tables:

  | | physics | scrollbars | overscroll | fling |
  |---|---|---|---|---|
  | Android | clamping | none | stretch (base: glow) | regression |
  | Fuchsia | clamping | none | glow | regression |
  | iOS | bouncing | none | none | recent average |
  | macOS | bouncing, fast | shown | none | recent average, desktop weights |
  | Linux, Windows | clamping | shown | none | regression |

- **`ScrollConfiguration::new(behavior, child)`** puts a behaviour in force for a subtree,
  and **`ScrollConfiguration::of()`** reads the one in force, the base where nothing
  installed one, as the reference's does. It is carried the way `ScaffoldInfo` is: a
  `Widget::scroll_behavior_override` hook, applied by the layout, paint and deferred-build
  walks and hashed into the relayout fingerprint, and forwarded by every transparent
  wrapper.
- **`Application::scroll_behavior()`**, the reference's `MaterialApp.scrollBehavior`,
  defaults to `ScrollBehavior::material()`. The shell puts it at the root of the tree
  each frame (`ScrollConfiguration::set_root`).
- **Each scroll area records what its behaviour settled on** when it is registered, under
  the theme it is laid out in: `Scrollable::physics` (its own if it asked, else the
  behaviour's), `Scrollable::overscroll` and `Scrollable::fling`. The scrollbar decision
  asks the behaviour in force. The shell's drag, fling, wheel and overscroll paths read
  the area's values; a drag on an area reads its fling with the area's tracker.

## What changes for an application

- **Breaking**: `Application::scroll_physics()`, `scrollbars()` and `overscroll_indicator()`
  are gone. One override replaces them:

  ```rust
  fn scroll_behavior(&self) -> ScrollBehavior {
      ScrollBehavior::material().with_physics(ScrollPhysics::BOUNCING)
  }
  ```

- **Breaking**: the `platform_default()` constructors and the `Default` impls of
  `ScrollPhysics`, `Scrollbars` and `OverscrollIndicator` are gone: their answer depends on
  a theme. `VelocityTracker::default()` is the base tracker (regression).
  `Scrollable::physics` is no longer optional, and `Runtime::advance_scroll`,
  `glow_pull` and `glow_absorb` take what the area settled on instead of an application
  default. `Runtime::scrollbars`, `Runtime::overscroll_indicator` and
  `Runtime::with_scrollbars` are gone.
- A `ListWheel` without physics of its own now takes its behaviour's.

Three divergences from the reference went with the compile-time defaults: Fuchsia had
scrollbars, the desktops glowed past an edge where the reference shows nothing, and a
theme's platform reached no scrollable at all.

## Alternatives weighed

- **A `for_platform` constructor on each type**, read from the theme by each consumer. It
  is four decisions in four places, which is what the reference avoids by giving them one
  owner, and it leaves no way to change them for one subtree.
- **The behaviour in the theme.** The reference keeps it out: it is how things move, not
  how they look, and a subtree changes it without restyling anything.
- **Resolving the behaviour in the shell, at gesture time.** The shell's gesture code runs
  after the walk and does not know which subtree an area was in; recording the answers at
  registration, as the area's own physics already were, is what makes a subtree's
  behaviour reach the drag.

## Tests

- `the_application_s_behaviour_follows_the_theme_s_platform`: the table, for every
  platform. `the_base_behaviour_follows_the_system`,
  `a_changed_behaviour_keeps_what_it_did_not_change`, `an_installed_behaviour_is_put_back`.
- `a_scroll_area_carries_its_physics_into_the_registry`: an untold area under an iOS theme
  bounces, shows nothing past the edge and reads flings the iOS way; under Android it
  clamps and stretches; under a `ScrollConfiguration` it takes that behaviour, which is put
  back after the subtree; and an area's own physics win.
- `the_scroll_behaviour_at_the_root_is_the_applications`: the shell puts the
  application's behaviour at the root.
- The scrollbar tests lay out under a behaviour that draws one, or one that does not.

Mutation testing: ten mutants, all killed — the application's behaviour following the
system instead of the theme, Android's stretch turned to a glow, Fuchsia given scrollbars,
`with_scrollbars(false)` ignored, a subtree's behaviour never put back, a
`ScrollConfiguration` that installs nothing, an area with no indicator glowing anyway (on
a pull and on a fling), a drag reading its fling the application's way instead of the
area's, and the shell not putting the application's behaviour at the root. The test that
forwards hooks through every transparent wrapper also caught `ScaffoldScope` dropping a
`ScrollConfiguration` under it.
