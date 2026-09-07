# Milestone 440 — A switch that never answered

Hovering a switch did nothing. Focusing it did nothing. Holding it down did nothing. It had
no state layer at all — the only interactive widget left in the crate with none — so the
only feedback a pointer got from it was the toggle itself, after the fact.

It also had no thumb icon, which is the one thing Material 3 offers for the switch that
needs to be legible without colour.

## The state layer, and one deliberate difference

The reference paints the toggle's radial reaction over the track and under the thumb, in
`primary` when the switch is selected and `on_surface` when it is not
(`switch.dart:2264`). Here it is `Theme::state_layer` — one opaque lerp, in the space the
tokens were written in, because a translucent circle handed to the GPU blends in linear
light and paints at something other than the number it names. That is milestone 329's
finding and milestone 437's, and this is the third widget to take it.

**The ink is not the reference's, and the reason is geometry.** The reference's reaction
circle has radius 20 against a track 32 tall: it is *wider than its track*, and the part
that shows is over the page. `primary` over the page is visible at either end.

This one is bounded by the switch's own box, which is exactly the track. So its ground is
the track — and `primary` lerped over a `primary` track is that track again. The layer would
vanish precisely where a pointer is most likely to be. It takes the **track's content
colour** instead (`on_surface` off, `on_primary` on), which is what `Theme::state_layer` is
documented to want and what makes it visible at both ends. The role differs because the
ground differs; the rule does not.

Nothing lights on a disabled switch, in any of the three states — the same sentence
milestones 436 and 437 wrote about a destination.

## The thumb icon, and the size it forces

`thumbIcon` is null by default in the reference and unset here, because a switch is legible
without one. It is there for the setting that has to be readable in more than colour and
position: a tick inside the thumb says *on* a third way.

The part worth knowing is what it does to the geometry. An off thumb is a 16-pixel dot, and
the glyph is 16 pixels — it does not fit. The reference's answer is
`thumbRadiusWithIcon = 12` (`switch.dart:2369`): **a thumb carrying a glyph is the on-thumb's
size at both ends**. And the rule is about the switch, not about the end it is at — giving
only the on end an icon still grows the off thumb, because a switch that changed size when
flipped, for a reason that has nothing to do with being flipped, is two switches.

The glyph's off colour is `surfaceContainerHighest` (`:2349`) — the track's own colour — so
it reads as a hole punched through the thumb rather than a mark drawn on it.

Which glyph shows follows the switch's **state**, not its animation, as the reference
resolves `thumbIcon` from `WidgetState.selected`.

## The pressed radius

`pressedThumbRadius = 14` (`:2357`), past both ends of the travel: the squish a finger
expects back. Discrete here, because `Status` carries `hover_progress` and `focus_progress`
but no press *progress* — a press is a state in this framework, not a progression. That is
the one thing between this and the reference's animated version, and it is recorded.

## The tests

- `a_switch_answers_the_pointer` — two rectangles at rest, three when hovered; the layer
  opaque, equal to the theme's rule over the track, inside the box at either end, and as
  tall as the track.
- `the_layer_is_the_accent_at_one_end_and_the_ink_at_the_other` — including that it is
  *visible against* the track, which is the assertion the reference's own role would fail.
- `a_disabled_switch_does_not_light` — hover, focus and press.
- `a_thumb_that_carries_a_glyph_is_the_larger_one` — and that the on end's icon grows the
  off thumb.
- `the_glyph_is_drawn_at_the_end_that_has_one` — both colours, and nothing where no icon was
  named.
- `a_held_thumb_swells` — past both ends.

Four of the six fail with the three changes reverted; the other two exercise a builder that
did not exist.

## Still open

The press does not animate, for want of a press progression in `Status`. And `splashRadius`
— the reference's reaction is wider than its track, which needs a switch whose box is larger
than its track, i.e. the 48-pixel tap target this widget does not reserve.
