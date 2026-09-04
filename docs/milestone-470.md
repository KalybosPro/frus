# Milestone 470 — The other half of the search bar

Milestone 469 built `SearchBar` and stopped, which is where the reference also stops if you
only reach for `SearchBar` — a bar alone is a useful thing. `SearchAnchor` is the other
half: press the bar and a surface grows out of it holding the same query, a rule, and
whatever the application thinks you meant.

## It is open because the application says so

The reference pushes a `_SearchViewRoute` — a `PopupRoute` — and keeps `isOpen` inside a
`SearchController`, because a Flutter widget owns its state. Nothing here owns state.

So `SearchAnchor::new(open, query)` takes both from the model, `on_open` is what the bar
emits when pressed and `on_close` is what the back arrow and the scrim emit. That is the
shape `Drawer` has had since milestone 46 and `ExpansionTile` since 463, and the payoff is
the same one every time: **a test that wants the view open just says so**. There is no
route to push, no controller to fake, no frame to pump.

It also removes the reference's `SearchController` entirely. The application already has a
`String` in its model; a second object holding the same string is a second object to keep in
step with it.

## Two views, one widget

On a phone the view **is** the screen; on a desktop it hangs under the bar. The reference
decides by platform (`search_anchor.dart:554`); this decides the same way, at compile time,
with `full_screen(bool)` to overrule it — a tablet in landscape is a phone that should be
answering *no*.

The difference is four things and no more:

| | full-screen | floating |
|---|---|---|
| size | the window | at least 360 × 240 |
| corners | square | 28 |
| header | 72 tall | 56, a bar's own |
| around it | nothing | the page, and a scrim |

**A full-screen view has no corners to round.** It is the screen, and a rounded screen is a
rounded rectangle with the wallpaper showing through its corners. The reference says the
same in one ternary (`search_anchor.dart:1947`) and it is the kind of line that gets
"simplified" away by someone who has only seen the floating form.

The taller header is the other one worth keeping: seventy-two is fifty-six plus sixteen, and
the sixteen is a thumb. A header that is the top of a screen is reached past; one hanging
under a bar on a desktop is not.

## The header is the bar again, flat and transparent

The view's header is a `SearchBar` with three things turned off — its background, its
elevation, and its highlight (`search_anchor.dart:1163`). The view is already a raised
surface; a raised pill inside it would be a second one, with its own shadow falling on the
suggestions.

Two of those three were builders 469 already had. The third was not, and the fix is a rule
rather than a property:

> **A bar with no surface has no state layer.**

A state layer is a lerp *from the ground* toward the ink. Lerping from a transparent ground
toward `on_surface` does not highlight the bar — it puts a grey wash over whatever is
behind it. So the state layer is skipped when the resolved background has no alpha, which
is what the reference's `overlayColor: transparent` is for, expressed once instead of at
every call site. `a_bar_with_no_surface_does_not_light` holds both halves.

## Two flex factors for one idea

The first picture of the view was a header, a rule, and a blank.

The suggestions live in a `SingleChildScrollView`, and a scroll view with nothing to grow
into collapses to nothing and clips everything in it — which is a trap this project has
already been caught by once and written down. The view has a floor of 240 that its content
does not reach, so:

- the **column** fills the view (`flex(1.0)`), and
- the **list** fills the column (`flex(1.0)`).

Both are needed, and the reference has both: `Flexible(fit: tight)` around the list
(`search_anchor.dart:1189`) inside a `Column` inside a box with the view's constraints. One
factor without the other leaves the list exactly where it started.

Nothing in the tree was wrong. Every widget was present, every style resolved, and the
golden was a blank rectangle — which is the third milestone running where the picture found
what the tree could not.

## The cross that lies

The clear cross exists when there is something to clear (`search_anchor.dart:1048`) **and**
when there is somewhere for it to say so. Two conditions, because a cross that emits nothing
is worse than no cross: it is a control that looks live and is not.

`clear_button_label` joins the localizations table, and deliberately not
`close_button_label`: the same glyph means two different things depending on what it sits
in, and *Close* on a control that empties a search box sends a reader looking for the thing
it closed.

## `SearchViewTheme`

Twelve fields, apart from `SearchBarTheme` because the reference keeps them apart and
because the two disagree where it matters — the view's shape depends on whether it is the
screen, its header is taller than a bar when it is, and it has a rule under that header that
a bar has nothing to do with.

## Small things that were in the way

`OverlayPortal` gained `new_boxed` and `overlay_boxed`. A widget that *assembles* its anchor
has a `Box<dyn Widget>`, not an `impl Widget`, and there was no door for it — every existing
caller types its anchor out. The two named constructors are two lines and the old ones
delegate.

## Verification

`cargo fmt`, clippy across the workspace with all targets and all features: silent.
`RUSTDOCFLAGS='-D warnings' cargo doc`: silent. **1289 unit tests**, all green — nine of
them new, three checked by breaking what they guard and watching them fail. Goldens
**91 + 34 + 14**, one picture added and none moved.
