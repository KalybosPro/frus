# Milestone 365 — The selection controls, themed and overridable

Four widgets — `Switch`, `Checkbox`, `RadioGroup`, `Slider` — had **no way at all** to
change a colour. Not a builder, not a theme entry. Whatever the scheme's `primary` was,
that is what they were, and an application whose brand colour is not its accent could not
have a green checkbox.

That is a direct breach of this project's own standing rule: *themed defaults are fine,
hardcoded-only never*. The rest of the catalogue has followed it for a long time —
`WidgetThemes` carried eleven widgets before this — and these four had simply never been
brought in.

## The shape, unchanged

The pattern is the one `Chip` established and everything since has copied, so there is
nothing new to learn:

```
instance override  →  theme.widgets.<widget>  →  the scheme's role
```

Four new theme structs (`SwitchTheme`, `CheckboxTheme`, `RadioTheme`, `SliderTheme`) and
one builder per colour on each widget. The defaults are exactly what was painted before, so
no existing screen moves a pixel — the goldens are the proof of that.

## Three decisions the resolution forced

**A partial override must not tear the control in half.** A checkbox has a resting outline
and a brighter one under a pointer, and a caller who names *one* of them means the outline.
So the pointer state falls back to the resting override before it falls back to the scheme.
Without that, `Checkbox::border_color(green)` would give a green box that turned grey the
moment a finger came near it — the override working everywhere except where you were
looking. The same rule covers the radio's ring.

**A switch interpolates between two resolved ends, not two resolved colours.** The track
animates from off to on across `t`; each end is resolved separately and *then* mixed. Had
the override been applied after the mix, recolouring the track would have left the
animation running through the old colour and only landing on the new one.

**The slider's thumb ring follows the travelled track unless it is named.** They are the
same colour in the default scheme, and a caller who recolours the track and not the ring
means the accent — not a hairline left behind in the colour they just replaced.

**An off thumb follows the on thumb.** A switch is one thumb sliding, not two swapping
places, so `inactive_thumb_color` defaults to `thumb_color` rather than to white a second
time.

## Where the colours are resolved

At **paint** time, never at build time — which matters for `RadioGroup`, the only one of
the four that builds child widgets in advance. Its options carry the *unset* options down
and each resolves against the theme it is painted under, so a group inside a `Themed`
subtree takes that subtree's palette. Resolving in `rebuild()` would have baked in whichever
theme happened to be ambient when the builder ran, which is not a theme at all.

## Left

`overlayColor` — the tint of the ripple a selection control shows under a finger — is
`InkTheme`'s business rather than each widget's, and is already answered there.
`SliderThemeData`'s shape overrides (a custom thumb, a custom tick mark) are a different
kind of thing: painters rather than colours, and they want the reference's
`SliderComponentShape` before they want a builder.
