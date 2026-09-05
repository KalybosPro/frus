# Milestone 428 — A switch is not a pill with a dot in it

Milestone 427 recorded the switch's off state as *one step or none*, because the reference
does not simply fill the off track with a different colour. It is a design of five parts
that only makes sense together, and this crate had a different one.

| | this crate, before | the reference |
|---|---|---|
| off track | `outline`, no edge | `surfaceContainerHighest` (`switch.dart:2246`) |
| the edge | — | `outline` at 2px, off only (`:2259`, `:2298`, `:2254`) |
| off thumb | whatever the on thumb was | `outline` (`:2212`) |
| on thumb | white | `on_primary` (`:2201`) |
| thumb size | one size | 8 off, **12** on (`:2354`, `:2317`) |
| track | 44 × 24 | 52 × 32 (`:2378`, `:2375`) |

Changing one of those alone makes things worse rather than better. A
`surfaceContainerHighest` fill with no edge is a pill you can barely see; an edge round a
thumb that already nearly fills the track is a ring with nothing to ring. **The thumb
growing is the part that carries the meaning**: off it is a dot inside an outlined track,
on it is a disc on a filled one, and that difference is most of what tells the two states
apart before you have read either colour.

## The rule belongs to one end

The reference resolves the track's edge as *selected → transparent, disabled → 12 %,
otherwise `outline`* (`switch.dart:2251`). Selected wins first, so a switch that is on has
no edge whether it is available or not.

Here, that either-or is written as the animation it already was: the ring is drawn at
`(1 - t)` of its alpha, where `t` is the travel the thumb is already interpolating along.
Off it is fully there, on it is gone, and halfway it is half there — which is what the
reference's own lerp between the two resolved ends produces, without a branch.

## The reversal

`inactive_thumb_color` used to default to whatever the **on** thumb was, with a reason
written beside it: *a switch is one thumb sliding, not two swapping places*.

The reasoning was right. The conclusion was not. The reference resolves both ends and
interpolates between them, so it is still one thumb — one that **changes colour as it
travels**, exactly as the track under it does. Saying the on colour no longer says the off
one, and `the_two_ends_of_the_thumb_are_two_colours` is the test that used to assert the
opposite.

## The disabled state, which already matched

The old test `a_disabled_switch_takes_both_halves_of_the_rule` was defending a real
argument: the switch is the one control that shows the 12 % container rule and the 38 %
content rule *at the same time*, which is what makes the split container-against-content
rather than one rule per widget.

That argument survives, but the container half moved from the fill to the edge. The
reference's disabled **off** track is `surfaceContainerHighest` at 12 % (`:2223`) — a wash
so close to the page that it is nearly nothing — while its ring is `onSurface` at 12 %,
which is `disabled_container` here. Filling the pill with `disabled_container` drew the
*ring's* colour across the whole shape, which is the opposite picture. The disabled **on**
track is `onSurface` at 12 % (`:2221`) and does stay `disabled_container`, with the opaque
`disabled_mark` thumb on it, both unchanged.

So the test now asserts on the ring, and adds that the wash inside it is quieter still.
Every other part of this crate's disabled model turned out to map onto the reference's
exactly, including the opaque thumb.

## The tests

- `the_defaults_are_the_reference_s` — the four colours and the ring, each against the line
  that names it. Both ends of the travel are the *arrival of a lerp*, so they land on the
  colour rather than matching it bit for bit; the module's own `lands_on` already existed
  for that, and the first draft of this test failed on the eighth decimal for want of it.
- `the_ring_belongs_to_the_off_end_alone` — alpha 1 off, 0 on, and **0.5 halfway**, so the
  ring cannot pass by snapping at one end.
- `the_thumb_grows_as_it_travels` — 16 across off, 24 on, and centred in the rounded cap at
  both ends so it never breaks the pill.
- `the_two_ends_of_the_thumb_are_two_colours` — above.

## Still open

The switch has no thumb **icon** (`switch.dart:2320`), which the reference offers for a
switch whose state needs saying in more than a position, and no pressed thumb radius
(`:2357`, 14 rather than 12) — this crate's `Status` carries hover and focus progress but
not a press progress the thumb could grow along.

`overlayColor` — the state layer round the thumb (`:2264`) — is likewise unpainted here.
