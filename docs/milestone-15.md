# Milestone 15 — Theme system

Centralises the *design tokens* (colours, radius, spacing) in a `Theme` injected
at render time, with light and dark presets.

## What ships

- **`Theme`** (frus-widgets): the tokens `background, surface, primary,
  on_primary, on_surface, muted, border, focus, selection, radius, spacing`;
  presets `Theme::dark()` / `Theme::light()` (default = dark).
- **Injection**: `build_ui(root, size, &runtime, &theme)` passes the theme on to
  `Widget::paint(bounds, status, &theme, scene)`.
- **Themed defaults**:
  - a `Text` with no explicit colour → `theme.on_surface`;
  - `TextInput`: background/border/focus/selection/text from the theme;
  - scrollbars → `theme.muted`.
- **Overriding**: an explicit colour (`Container::color`, `Text::color`) still
  wins.
- **Demo**: root background `theme.background`, buttons `theme.primary`, and a
  **"Light/dark theme"** button that flips the whole UI.

## Decisions

- The theme is **read at paint time** (the `paint` signature gains a `&Theme`)
  rather than resolved at construction: widgets adapt without the caller
  re-injecting the colours.
- The application background is a root `Container(theme.background)` (no coupling
  to the renderer or to its clear colour).

## Tests

- `Theme::dark` / `Theme::light`: distinct tokens.
- A `Text` with no colour paints with the theme's colour (through the themed
  paint).

## Scope (v1)

- `Container` keeps its **explicit** colours (deliberately coloured containers);
  the theme serves the **defaults** (text, field, bars) and the demo.
- No named component styles yet (Button/Checkbox…), and no animated transition
  between themes (the change is instant).
