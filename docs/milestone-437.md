# Milestone 437 — Three mistakes in one line

A navigation destination lit up under the pointer like this:

```rust
} else if status.hover_progress > 0.0 {
    let a = 0.12 * status.hover_progress * o;
    scene.draw_rect(pill, theme.muted.fade(a), pill_h * 0.5, 0.0, Color::TRANSPARENT);
}
```

Every part of that is wrong, and each part is wrong in a different way.

## The wrong kind of colour

`theme.muted.fade(0.12)` is a **translucent** fill handed to the GPU. The render target is
`Rgba8UnormSrgb` and the blend happens in linear light, where 12 % does not paint like 12 %
— it paints like about a third. This is the framework's recurring bug, pinned by
`frus-test/tests/blending.rs` and resolved for the disabled tokens back in milestone 329:
**a state layer is resolved here, opaquely, or it is not the colour anyone chose.**

`Theme::state_layer(base, ink, status)` is exactly that rule — `base.lerp(ink, overlay)`,
one lerp, in the space the tokens were written in — and the rest of the framework already
asks it. This widget did not.

## The wrong role

`muted` is not a state-layer role; it is a text colour. The reference's ink for *this*
widget is `primary` (`navigation_rail.dart:946`), which is unusual enough to be worth
copying rather than guessing — a destination's hover is the rail's accent, not a grey wash.

## The wrong number

12 % is the **splash**'s opacity in the reference (`:943`). The hover's is smaller. Since the
framework keeps one state rule in the theme rather than a table of numbers per widget, the
fix is not to hard-code the reference's 4 % here but to ask the rule, which answers 8 % for
hover — and, being the rule, answers focus and press too.

## And a fourth thing: it was an `else`

The layer lived in the `else` of `if self.selected`, so **the selected destination never
responded at all** — the one a pointer is most likely to be over. The reference has no such
split: the ink well's overlay paints over the indicator, because the indicator is what the
destination is standing on when it is selected.

That is the shape the fix takes. A destination now works out the **ground** it stands on —
the indicator when it has one, otherwise the surface the rail or the bar painted under it —
and the state layer is a lerp from that ground:

```rust
let base = if self.selected { indicator } else { ground };
let fill = theme.state_layer(base, theme.scheme.primary, &status);
if self.selected || fill != base { … }
```

At rest and unselected, `fill == base == ground`, and nothing is drawn — which is what it
has always done, and what one of the older tests asserts.

## The ground has to be the one that was actually painted

A state layer starts from the colour underneath it, so a destination has to know which
colour that is. The two navigation widgets stand on different rungs (milestone 427: a rail
on `surface`, a bar on `surface_container`) and either can be told a third colour by the
caller or by the theme. So `NavItem` carries the caller's `background`, and resolves the
default the same way the widget above it does.

Which turned up a small latent bug: `NavigationRail::background` and `BottomBar::background`
were the two builders that did **not** invalidate the destinations, because until now no
destination read them.

## What a destination that cannot be reached does

Nothing, in any of the three states. A state layer is the promise of an interaction and
there is none here — the same reasoning milestone 436 applied to the hover, now applying to
focus and press as well because they arrived together.

## The tests

- `a_destination_s_state_layer_is_opaque_and_starts_from_the_ground` — alpha 1, equal to the
  theme's rule over the rail's own rung, and nothing at rest.
- `the_layer_starts_from_the_ground_the_widget_painted` — a rail and a bar differ, and a
  rail told a colour uses it.
- `a_selected_destination_lights_under_the_pointer_too` — the indicator at rest, the
  indicator plus the layer under the pointer.
- `focus_and_press_light_it_as_well` — both, where the old line read `hover_progress` alone.
- `a_disabled_destination_lights_in_no_state_at_all` — all three.

Four of the five fail against the old line; the fifth (`disabled`) passes, since the old
code already guarded its one state.

## Still open

The bar's own hover ink. The reference lets a `NavigationDestination` carry an
`overlayColor` as a `WidgetStateProperty` (`navigation_bar.dart:232`), a per-state colour
this framework has no equivalent of — its state rule is one lerp, not a function from state
to colour. That is a design question, not a missing field.
