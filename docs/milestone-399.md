# Milestone 399 — The settings a screen reports about its user

Milestone 391 audited `MediaQueryData` against ours and left a list. This is the
accessibility half of it, and one of them the framework had to **obey** rather than merely
report.

## `disable_animations`, and what it actually asks for

A user who has asked for reduced motion has not asked for a broken interface. So:

> The implicit animations — the ones a widget starts by itself when a value it was given
> changes — **complete at once instead of over time**. The change still happens; it stops
> moving.

Skipping the change instead would leave the interface *wrong* rather than *still*, which is
the failure this setting is most often given. The test checks both halves: nothing is left
animating, **and** the value the widget asked for has actually arrived.

It does **not** touch scrolling. A fling is physics answering a finger, not a decoration,
and a list that jumped to a stop would be harder to use rather than calmer. The reference
draws the same line: its `disableAnimations` is read by the widgets it makes sense for, not
clamped globally.

`Runtime::still` is where it lands, set from the application every frame — so a settings
screen with a *reduce motion* switch of its own changes it while running, and does not have
to restart to be believed.

## The rest, reported

`MediaQuery` gains `text_scaler`, `platform_brightness`, `system_gesture_insets`, and an
`Accessibility` struct — `bold_text`, `high_contrast`, `disable_animations`,
`invert_colors`, `accessible_navigation`, `always_use_24_hour_format`.

They are one struct rather than six fields because they arrive from one platform query and
are read together, and because a widget that honours one usually honours its neighbours.

**Which have a consumer, plainly:** `disable_animations` is honoured by the framework. The
rest are reported for the application to act on. Saying so is worth more than a field that
looks obeyed and is not — which is the failure milestone 397 found behind a green tick.

## `text_scaler` carries the number and does not yet spend it

This is the largest accessibility gap the framework has, and it is deliberately **not**
half-done here.

`MediaQuery::scaled(size)` applies the user's font-size setting, and it is a *function*
rather than a multiplication for the reason the reference made `TextScaler` one: a platform
may scale non-linearly, large sizes growing less than small ones so a heading does not run
off the screen when body text is made readable. Ours is linear; the shape of the call is the
one that can stop being linear without every caller changing.

Making the **framework** honour it needs two things to agree, and the design is worked out
even though the work is not:

1. **Measurement.** 69 call sites across ~25 files reach `frus_text::measure*` directly. They
   cannot each be asked to remember; the scale has to be applied inside `frus_text`, at the
   top of `measure_wrapped`, `baseline` and `line_height`.
2. **Paint.** The renderer shapes at `Primitive::Text { size }` and nothing else
   (`frus-gpu/src/text.rs:205`), so that field has to carry the already-scaled size.

If those two ever disagree, the result is text measured at one size and drawn at another —
a layout that is wrong everywhere, all at once. That is why it is its own milestone rather
than a paragraph in this one.

## Not yet reported by any platform

Nothing fills these in yet: `Application::accessibility` defaults to
`Accessibility::NONE`, and an application that knows better answers it. Android reports the
lot through `Configuration` and `AccessibilityManager`, and wiring that up is the next step
on this thread.
