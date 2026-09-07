# Milestone 407 — The platform reports its user, and two bugs a compiler could not see

Milestone 403 made the framework scale text by the reader's font setting. Milestone 406
made every widget obey it. Both were recorded as *"no platform reports the setting yet"* —
inert on a device, correct only in tests.

This is that wire. It took two device runs to find that the milestone was two bugs deep.

## What the platform now answers

`Configuration.fontScale`, the night setting, `fontWeightAdjustment`,
`AccessibilityManager.isTouchExplorationEnabled`, the animator duration scale,
`DateFormat.is24HourFormat`. It needs **no dex and no bridge class**, unlike the IME: every
one is a public field or a public method, one JNI walk from the activity. On desktop winit
answers the one thing its window system knows — the light/dark theme.

The read is cached, not per-frame: it is a walk across a language boundary and the answer
changes about once a year. It is refreshed when the surface appears, which on Android is
also when it *re*appears — no `configChanges` is declared, so a font-scale or night change
recreates the activity, and a trip to the system settings and back is exactly that event.

## The design question, which had to come first

The application was until now the **only** source of these settings, through
`Application::accessibility()`. Turning the platform on would have made the two fight in
silence: either the app's answer overrules a real user preference, or the platform
overrules an app that deliberately set one.

The rule is the reference's, and the only honest one: **the platform answers, and the
application overrides what it chose to speak for.** The settings belong to the person using
the device.

That needs a type that can say *nothing*, per setting. A plain `Accessibility` cannot — a
`false` in it is indistinguishable from silence, so an application forcing *reduce motion*
would also be declaring that its user runs no screen reader. So
`AccessibilityOverrides`, every field an `Option`, `NONE` meaning *ask the user*. It is the
same shape and the same reason as `TextStyle` against `ResolvedTextStyle` (milestone 402):
one type asks, the other answers.

A side effect worth naming: `runtime.still` read `self.app.accessibility()`, so even once
the platform reported *reduce motion*, the framework's own animations would never have
heard it. It now reads the resolved answer.

## The first device run: a crash on launch

```
No pending exception expected: java.lang.NoSuchFieldError:
no "I" field "fontWeightAdjustment" in class "Landroid/content/res/Configuration;"
```

`fontWeightAdjustment` arrived in **API 31**; the test phone is API 29. The field does not
exist, the read threw, and `.ok()` discarded the Rust `Err` — **but not the Java
exception**, which stays pending on the thread. The next JNI call, any call, aborted the
whole runtime.

So: a crash on launch on every device below API 31, in code that compiled perfectly,
because JNI resolves names at runtime and a compiler has nothing to check.

Three changes, and the third is the one that matters:

1. `clear_pending` after every swallowed failure, with the reason in the module's own docs.
2. Each fallible read became a pair — an inner `Result`-returning function holding no borrow
   of the environment, an outer one that clears before returning `None`.
3. The version-gated field is **not asked for** below API 31 rather than asked and
   recovered. Recovering works; not throwing is better, because every throw is another
   chance to leave one behind.

`read()` clears on its own failure path too: a walk that gives up half-way leaves an
exception that would kill the *next* caller — the IME bridge, for instance.

## The second device run: the scaling had never worked at all

The app launched. The font scale was 1.30. The screen was **pixel-identical** to 1.0.

`MediaQuery::scope` wrapped `self.app.view(&theme)` — the *construction* of the widgets.
But a size becomes a number in three places:

| | when | scaled before? |
|---|---|---|
| building the widgets | inside `view` | yes |
| measuring and laying out | `build_ui` | **no** |
| painting | the render pass | **no** |

The two steps that decide how big text actually is ran at scale 1. The layout measured one
size and the renderer drew another — the precise failure `TextStyle::resolved()` was built
to make impossible.

**Milestone 403 was wrong from the day it landed**, and nothing in the suite could see it:
its tests wrap `build_ui_inspected` themselves, so the harness installed the very condition
the real code forgot. 91 goldens, clippy, strict rustdoc — all silent. Two identical
screenshots settled it in a second.

The fix separates two lifetimes that had no reason to be the same. The *description* is
only meaningful while widgets are built; the *reader's font size* must hold for the whole
frame. `install_text_scale` returns a guard instead of taking a closure, and the shell
holds it across build, layout and paint.

`with_text_scale`'s own documentation said "the framework installs this around `view`".
That was accurate, and it was the bug.

## Verified on the device

A Huawei STK-L21, Android 10 (API 29), at `font_scale` 1.0 and 1.30, screenshots read side
by side. At 1.30 the title grows, the tip paragraph rewraps from two lines to three, the
buttons and rows grow with their labels — which incidentally proves milestone 406's floors
on real hardware — and the page scrolls, as a page whose content grew should.

## Left

- **No test covers the shell's frame.** The scope bug lived in `app.rs`, which no test
  drives: the suite exercises `Application::view` and `build_ui`, never a frame. This was
  found by a screenshot and it would be found again the same way. A frame-level harness is
  the fix and it is not written.
- **A live change is picked up on resume, not instantly.** Android recreates the activity
  for the font scale and the night setting, so those are immediate in practice; TalkBack
  turning on mid-session is not seen until the app is resumed.
- **Desktop reports no text scaler.** winit exposes no equivalent of Windows'
  `UISettings.TextScaleFactor` or GNOME's `text-scaling-factor`, so it stays at 1 there.
- **The scale is still linear** where the reference's `TextScaler` is a function.
- **iOS and Web report nothing**, having no shell of their own for this yet.
