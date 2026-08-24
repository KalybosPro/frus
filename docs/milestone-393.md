# Milestone 393 — Nobody hands the framework the screen

The question that started it: *does the reference make the developer give `Scaffold`,
`AppBar` and `Navigator` a width and a height?*

It does not. Checked in the source rather than remembered:

| widget | constructor | size parameters |
|---|---|---|
| `Scaffold` | `scaffold.dart:1688` | **none** — `appBar`, `body`, `drawer`, `bottomNavigationBar`, `backgroundColor`, `resizeToAvoidBottomInset`… |
| `Navigator` | `navigator.dart:1587` | **none** — `pages`, `initialRoute`, `onGenerateRoute`, `observers`… |
| `AppBar` | `app_bar.dart:195` | a **height only**, and it declares it rather than being told it |

`AppBar` implements `PreferredSizeWidget`, and `_PreferredAppBarSize` (`app_bar.dart:75`)
is built with `Size.fromHeight(...)` — that is `Size(double.infinity, height)`. An infinite
width means *fill what you are given*. The height is the bar's own, and the `Scaffold`
reads it.

The size travels down as **constraints**, never as arguments. A widget does not ask how
big the screen is in order to size itself; it is handed a box and fits in it.

Ours asked:

```rust
Scaffold::new(width, height)
AppBar::new(title).width(width)
Navigator::new(screen, width, height)
```

## What that costs

Milestone 392 is what it costs. A number that travels by hand gets arithmetic done on it,
and one of those subtractions is eventually wrong — three of them were, in our own demo,
each missing the same card margin. The failure is invisible until something reports it,
and then it is a whole screen laid out to the width of its widest line.

## The change

The framework already installed a description of the surface around every call to `view` —
size, pixel ratio, density, the intrusions the platform last reported — for
`MediaQuery::of()` to read. Three widgets now read it instead of being told:

- **`Scaffold::new()`** takes the size **and the three insets**. An application no longer
  measures the window, subtracts the notch and the bars, builds at the remainder and wraps
  the result in a padded background. It says `Scaffold::new()`.
- **`AppBar::new(title)`** folds its actions against the surface's width. Outside any
  description — a unit test building a bar on its own — there is no width to fold against
  and nothing folds, which is what it did before.
- **`Navigator::new(screen)`** slides across a window it does not have to be told the size
  of.

Each keeps an explicit override (`Scaffold::size`, `AppBar::width`, `Navigator::size`) for
the case that is genuinely not the whole screen, and for a test that would rather state a
size than install a description.

## `view` does not get the size either

```rust
fn view(&self, theme: &Theme) -> Box<dyn Widget<Msg>>;
```

Keeping `width` and `height` on the application's own entry point would have left the
habit in place with nothing to spend it on. A screen that really is the window still says
so — it reads `MediaQuery::of().size` — but it reads the description in force rather than
a number carried down from its caller's caller.

The demo's `view` went from twelve lines of arithmetic and a wrapper to one:

```rust
fn view(&self, theme: &Theme) -> Box<dyn Widget<Msg>> {
    Box::new(build_view(self, theme))
}
```

and `width`/`height` came off ten screen functions and off the router that fed them.

## The detail that a test found

A `Scaffold` with **no app bar** does not hold its body off the status bar. That looked
wrong until the reference said otherwise: `contentTop` (`scaffold.dart:1043`) is
`extendBodyBehindAppBar ? 0.0 : appBarHeight`, which is **zero** when there is no bar, and
`_BodyBuilder` (`scaffold.dart:960`) returns the body untouched in the ordinary case — the
body still sees the full `MediaQuery.padding` and is expected to say `SafeArea` itself.

So ours is right, and the demo's seven screens that have no `Scaffold` now say `SafeArea`,
which reads the same description. The background still runs under the bars; the content
does not.

`SafeArea` gained one line while proving it: `flex_grow: 1.0`. The reference's is a
`Padding` under the screen's own tight constraints — it *is* the box it was handed. A flex
node that grows nothing hugs its content instead, and a screen wrapped in one comes out the
width of its widest line, which is precisely the failure of milestone 392.

## What is checked

- **`a_screen_keeps_clear_of_the_bars_without_being_told`** — six routes, an 84 px status
  bar and a 45 px navigation bar, and nothing readable painted in either band. Text drawn
  *beside* the viewport is skipped: a `Navigator` paints the screen it is leaving at a
  negative x, and that one is off the glass. (The first run failed on exactly that, which
  is how the exclusion came to be written down rather than assumed.)
- **`the_shell_takes_its_size_and_its_intrusions_from_the_surface`** — the same shell,
  built twice with no number in either call, comes out at two sizes and clears the status
  bar on both.
- **`a_bar_folds_against_the_surface_it_was_built_for`** — three actions fit on a wide
  surface and fold into the overflow menu on a narrow one, with no caller saying how wide
  anything is.

## What is left

The reference's `Scaffold` has properties ours has not: `drawerScrimColor`,
`drawerEdgeDragWidth`, `drawerEnableOpenDragGesture`, `endDrawerEnableOpenDragGesture`,
`drawerBarrierDismissible`, `onDrawerChanged`, `onEndDrawerChanged`,
`persistentFooterDecoration`, `floatingActionButtonAnimator`, `primary`. Its `AppBar` has
`flexibleSpace`, `scrolledUnderElevation`, `shadowColor`, `surfaceTintColor`, `shape`,
`iconTheme`, `actionsIconTheme`, `titleTextStyle`, `toolbarTextStyle`,
`excludeHeaderSemantics`, `toolbarOpacity`, `bottomOpacity`, `forceMaterialTransparency`,
`actionsPadding`, `clipBehavior`, `systemOverlayStyle`. Its `Navigator` has `observers`,
`transitionDelegate`, `onGenerateInitialRoutes`, `restorationScopeId`. Those are the next
milestones in this series, one widget at a time.
