# Milestone 322 — One disabled state, and a guard that reads the source

Milestone 320 ended by saying `enabled` was still a flag per widget rather than the
shared state every control could hang off. That undersold it. The real count:

- **five** widgets had the flag, each with `on_surface.fade(0.12)` and
  `on_surface.fade(0.38)` written out by hand;
- **ten** had no way to be disabled at all — including `Checkbox`, `Switch`, `RadioGroup`,
  `Slider` and `Dropdown`, which is most of what a form is made of.

A framework where a form cannot grey out a checkbox has a gap in the framework.

## The two colours, and why they are two

`crate::disabled` holds the rule once: [`disabled_container`] at 12 %,
[`disabled_content`] at 38 %. A disabled control **flattens** rather than fading, because
fading a selected control's accent gives a pale accent, which reads as *quietly selected*.

The split is **container against content**, not one rule per widget, and the switch is
what proves it — the reference disables its *track* at 12 % and its *thumb* at 38 %, one
control taking both halves. That settles the cases that look ambiguous: a checkbox's box
and a radio's dot take the **content** opacity, because they are the mark itself with
nothing behind them, not containers that happen to be square.

A third colour turned out to be necessary. A mark drawn *on* a disabled fill — a ticked
checkbox's tick, an on switch's thumb — cannot be another translucent `on_surface`, or
38 % on 38 % leaves the two within a few percent and the tick vanishes into its own box.
It punches through opaquely, in `surface`, which is what the reference does too.

## Greying out is the easy half, again

Milestone 320 wrote that line and then proved it by getting it wrong. The whole contract
is now stated in one place — no message, no ink, out of the tab order, announced as
disabled — and, more to the point, **checked**:

`every_control_with_an_enabled_flag_honours_all_four` reads the crate's own sources,
finds every widget carrying `enabled: bool`, and insists that each relevant hook it
implements consults that flag. It is the same instrument as the transparent-wrapper
guard, which has now caught three different omissions in twelve milestones.

**It failed on its first run**, on milestone 320's own work: `Chip`'s delete cross had its
tap gated and nothing else. Tab still landed on it and a reader was still told it could be
pressed — the same control reported three different ways, two of them wrong, and the two
that were wrong are the two nobody sees in a screenshot. That is precisely the failure 320
described in prose while shipping it in code.

Two things about the guard had to be sharpened before it was worth trusting:

- **every occurrence, not the first.** A module often holds more than one widget — a chip
  and its cross, a group and its options — and checking only the first would have cleared
  the exact parent/child pairing that makes a live control on an inert thing possible.
- **not every control is operated by a tap.** A slider is dragged and a field is typed
  into, so `draggable`, `on_drag`, `on_drag_delta`, `on_key` and `on_edit` are in the list.
  A disabled control that was inert only to the gesture nobody was using on it is not
  disabled.

A hook that is *unconditionally* inert counts as gated — a field answers taps through
`positional_click` and returns a bare `None` from `on_click`, which is already the
disabled answer for every state there is.

## Three controls that could not be disabled

`Checkbox`, `Switch` and `RadioGroup` take `enabled(false)` now, and all three keep their
answer: ticked, on, chosen. Read-only is not invisible, and a reader who cannot change the
setting is still owed what it is. `RadioOption` gained semantics it never had — announcing
that an option is unavailable without ever announcing the option would have been announcing
an absence.

`RadioGroup` needed its options **derived** rather than frozen. They were built as each
label was added, so `.enabled(false)` at the end of a chain would have reached none of
them: a group that looked disabled at the call site and answered every tap. There is a
test that builds the same group in both orders and demands the same result, because the
failure depends on nothing but the order somebody typed two lines in.

## Verification

1091 tests (16 new), clippy silent, and a new golden. `disabled_selection_controls` puts
each of the three beside its live twin, selected, because the claim that a 38 % tick on a
38 % box would disappear is easy to assert and hard to believe without looking. It does not
disappear — the picture is what says so.

## Left

- **Seven controls still cannot be disabled**: `Slider`, `Dropdown`, `Rating`, `Stepper`,
  `Menu`, `Tabs`, `Pagination`. The slider is the interesting one, because it is dragged
  and keyed rather than tapped — the guard is already waiting for it.
- **The unselected disabled switch track** is flattened from `on_surface`, where the
  reference bases it on `surface_container_highest`. Both land at 12 % and both read as a
  flat grey, so it is a tone rather than a rule, but it is a deviation.
- **No single disabled option**, in a radio group or a segmented control: it is the whole
  thing or none of it, as it was in 320.
- **`enabled` is still a flag on each widget.** It is now *one rule* rather than one rule
  per widget, which was the whole of this milestone, but the reference hangs the state on
  a resolver every control shares — `WidgetStateProperty`, which would also let a theme say
  *this colour, except when pressed*. That is the next shape, not this one.
- **`Checkbox`, `Switch` and `RadioGroup` have no theme entry at all**, so their colours
  are not overridable the way a chip's are. Pre-existing, and now more visible for having
  three more colours in them.
