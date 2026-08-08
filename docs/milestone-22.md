# Milestone 22 — Navigation bar (`NavBar`) + animated titles

A persistent navigation bar widget, which **slides and fades with its screen**
during `Navigator` transitions.

## `NavBar`

```rust
NavBar::new("My tasks")                     // root bar (title only)
NavBar::new("Settings").on_back(Msg::Pop)   // with a back button
```

- Fixed height (56 px), **centred title** painted within the bounds (independent
  of the button), thin bottom **divider**.
- Optional back button on the left (an internal `Button`) emitting the supplied
  message. Its left margin (`PAD_LEFT = 28`) puts it **beyond the back gesture's
  zone** (`BACK_EDGE = 24`), so it stays clickable without triggering the swipe.
- The title fades through `Status::opacity` (the mount fade).

## Animated titles — "for free"

The `NavBar` is **inside each screen's tree**. So it inherits the `Navigator`'s
transition (the slide + parallax + darkening from J19): during a push, pop or
gesture, the outgoing screen's title slides away and the incoming one arrives —
synchronised with the content, **with no dedicated animation engine**. That is
the benefit of keeping the bar per-screen rather than having one global bar.

## Demo

The **Settings** screen (pushed) now starts with a `NavBar` with a back button,
replacing the old improvised `screen_header`. **Home** keeps its rich header
(title + theme toggle + "Settings →"): a root bar with actions on the right is
beyond `NavBar`'s v1.

## Tests

- `root_bar_has_no_back_button`: the root bar has no button.
- `back_button_emits_message`: clicking back returns the message.
- `bar_paints_title_and_divider`: the title is painted.
- Total: **33 frus-widgets tests** + the frus-demo tests + the doctest.

## Limits (v1)

- Single-line title, no iOS-style *large title* and no subtitle.
- No actions on the right in `NavBar` (Home keeps a custom header).
- A "per-screen" crossfade, not a global bar morphing a single title.
