# Milestone 422 — The bar could not see the screen it was standing on

`automaticallyImplyLeading` has been on the roadmap since the `AppBar` was written, and it
was never a matter of adding a flag. The flag is one line; what was missing is the thing the
flag reads.

```dart
// app_bar.dart:1010
if (leading == null && widget.automaticallyImplyLeading) {
  if (hasDrawer) {
    leading = DrawerButton(…);
  } else if (parentRoute?.impliesAppBarDismissal ?? false) {
    leading = useCloseButton ? const CloseButton() : const BackButton();
  }
}
```

`hasDrawer` comes from the **context**: `Scaffold.of(context)`. A bar built by the
application, handed to a shell it has never met, asks the tree what shell it is in and gets
an answer.

frus had no such answer. An `AppBar` is `build()`-ed before the `Scaffold` ever sees it.

## The third inherited thing

The theme has been inherited since milestone 309, the surface since 417. This is the same
mechanism a third time, and the third time is where it stops being a special case:

| | hook | ambient | scope widget |
|---|---|---|---|
| the theme | `theme_override` | the walk's `&Theme` | `Themed` |
| the surface | `media_override` | `MediaQuery::of()` | `MediaScope` |
| **the shell** | **`scaffold_override`** | **`ScaffoldInfo::of()`** | **`ScaffoldScope`** |

The same four walks make the swap — `build_layout`, `build_deferred`, the relayout
fingerprint and the paint walk — and the fingerprint hashes it, for the reason the two
before it document beside themselves: a bar that grows a button is a different row, and two
shells that differ in what they hold must not share one entry in the layout cache.

The repository's own guard did the rest: `every_wrapper_states_the_hooks_the_macro_leaves_out`
failed until all five transparent wrappers said what they do with the new hook.

## Carrying a message through an ambient

This is the part the first two did not have to solve. A `MediaQuery` is plain data; a shell
knows a **message**, and the message's type is the application's.

```rust
drawer: Option<Rc<dyn Any>>,
```

Held as `Any`, handed back only to a caller that names the type it went in with:

```rust
pub fn drawer_toggle<Msg: Clone + 'static>(&self) -> Option<Msg> {
    self.drawer.as_ref()?.downcast_ref::<Msg>().cloned()
}
```

A bar whose `Msg` is not the shell's asks and is told nothing — which is the honest answer,
because it could not have sent that message anyway.

The fingerprint hashes **whether** there is a drawer, never which message opens it. Two
shells with different messages and the same drawers lay out identically, and hashing the
message would mean hashing a pointer that moves every frame.

## What a bar now implies

- **A leading**, from a drawer: `automatically_imply_leading`, `true` by default as in the
  reference.
- **An action**, from an end drawer: `automatically_imply_actions`, likewise
  (`app_bar.dart:1113`).

Both fill an **empty** slot and never add beside what the caller put there — the reference's
test is the same, and an overflow toggle counts as something the bar has already put at its
trailing end.

## What has no counterpart, and why

The reference's second branch is a **back button** for a route that can be popped. frus's
`Navigator` is *controlled*: the application holds the route stack and rebuilds the screens,
so the framework has no depth to imply anything from. A screen that can be left says so with
`AppBar::leading`, and that is recorded rather than faked.

## The tests

Counted rather than searched for: a closed drawer has a dismiss target of its own carrying
the same message, so what the test asserts is **one more** click that opens the drawer — and
that one is the button. Breaking `ScaffoldScope`'s hook so the scope stops reaching the
subtree fails it, which is the mechanism being load-bearing rather than the flag.

No golden moved (91 + 13 + 27) — a golden scene has no drawer.

## Still open on the `AppBar`

`clipBehavior`, `scrolledUnderElevation` with `notificationPredicate` (which needs scroll
notifications reaching the bar), and `systemOverlayStyle` (a message to the platform, not to
the tree).
