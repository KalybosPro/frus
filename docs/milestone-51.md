# Milestone 51 — System insets (safe area / SafeArea)

On mobile, the interface spilled **under the system bars** (the status bar at the
top, the navigation bar at the bottom). The framework now carries the platform's
**insets** up to the application, which keeps its content clear of those zones —
while the background still extends edge to edge.

## Where the insets come from (Android)

`android-activity` exposes `AndroidApp::content_rect()`: the content rectangle
**excluding** the system bars, in physical px. The shell keeps a handle on the
activity (`run_android`), and derives the insets from it each frame:

```
inset.top    = content.top
inset.left   = content.left
inset.right  = surface_width  − content.right
inset.bottom = surface_height − content.bottom
```

converted into **logical** px (÷ scale). A degenerate rectangle (before the first
layout) gives zero insets. On desktop platforms: always zero (no Android handle).

## Propagation to the application

A new `Application` trait entry point, on the model of `on_resize`:

```rust
fn on_insets(&mut self, _insets: Insets) {}   // default: no-op
```

The shell calls `on_insets` when the insets **change** (just after `on_resize`,
before building the view). The app stores them and uses them.

## How the demo applies them

The interface is built at the **inner** dimensions (window minus insets), then
wrapped in a full-window container with a `background` fill and
`padding_each(insets)`:

```rust
let w = width  - insets.left - insets.right;
let h = height - insets.top  - insets.bottom;
let nav = build_view(self, theme, w, h);
Container::new().width(width).height(height).color(theme.background)
    .padding_each(insets.top, insets.right, insets.bottom, insets.left)
    .child(nav)
```

The background covers the whole screen (including under the bars); the content
stays inside the safe area. Zero insets → no wrapping (desktop unchanged).

## Validation

- **On the device** (Huawei, Android 10): measured insets `top 84 px`,
  `bottom 45 px`, `left/right 0`; the content clears the status bar and the
  navigation bar, background edge to edge (confirmed by a screenshot).
- **Desktop**: `Insets::ZERO` everywhere, no regression (build + tests green,
  `on_insets_updates_safe_area` test).

## Limits (v1)

- **Overlays** (modal sheet, drawer, menus) are still positioned relative to the
  whole window, not to the safe area — a sheet can graze the navigation bar.
- No **soft keyboard** `viewInsets` yet (raising the content above the keyboard):
  that will be the IME milestone.
- The insets do not re-drive the responsive **tier** (computed on the full
  width); no effect in practice (side insets are zero in portrait).
