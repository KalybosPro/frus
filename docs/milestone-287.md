# Milestone 287 — The app bar, reviewed against Flutter's

A review of `AppBar`, prompted by the observation that it needed one, and carried out
by putting it next to Flutter's — which is the reference the rest of this framework
follows.

## What was wrong

Four things, three of them invisible until a bar got cramped:

1. **The bar hugged its content.** It never occupied the width it was *told* about, so
   `background(color)` painted a stripe behind the text rather than across the bar, and
   there was no free space in which anything could be centred. This one had been true
   since the widget existed.
2. **A long title pushed the actions off the edge.** Nothing truncated, nothing
   ellipsised. On the demo's own phone screen the title was already jammed against the
   first action button.
3. **The leading slot was a constant.** 56 px, whatever widget was actually in it, so a
   wider one silently broke the folding budget.
4. **No centred title and no `bottom` slot** — the two things every real bar needs and
   the reference has.

## What the reference has, and what was taken

Flutter's `AppBar` carries about thirty parameters. Ported, with its names:

| taken | as |
|---|---|
| `centerTitle` | `center_title(bool)` |
| `bottom` | `bottom(widget)` |
| `leadingWidth` | `leading_width(f32)` |
| `titleSpacing` | `title_spacing(f32)` |
| `foregroundColor` | `foreground(Color)` |
| `elevation` | `elevation(f32)` |
| `toolbarHeight` | the existing `height` |
| `backgroundColor`, `titleTextStyle`, `leading`, `title`, `actions` | already there |

Deliberately not taken: `flexibleSpace`, `shape`, `surfaceTintColor`,
`scrolledUnderElevation`, `systemOverlayStyle`, `toolbarOpacity`/`bottomOpacity`. Each
either belongs to a Material-specific surface model frus does not have, or is a
scroll-coupled effect that wants the scroll position threaded into the bar — a design,
not a parameter.

And one thing frus has that the reference does not: **folding actions into an overflow
menu**. Flutter's actions simply overflow. That difference turned out to decide the
whole layout policy, below.

## Who yields to whom

A bar has three claimants on one row — the leading, the title, the actions — and the
interesting question is what happens when they do not all fit. Flutter's answer is that
the actions take their intrinsic width and the title, being `Expanded`, gets the
remainder and ellipsises.

Copying that exactly gave a worse bar here, and the device showed it plainly: with the
demo's five actions, three of them squeezed inline and the title was cut to `My Ta…`
even though the screen was 1080 px wide. The reason is that frus has a mechanism
Flutter does not — the overflow menu — and using the *title* to make room while an
emptier tool sits unused is the wrong order.

So the rule is:

> The title keeps its natural width, up to **half** the bar. The actions fold into the
> overflow to fit what is left. Truncating the title is the **last** resort — for when
> even one action and the `⋯` button will not fit beside it.

The half-bar cap is what stops a long title from starving the actions instead; below
it, the title is cut with an ellipsis and the actions are served first. On the device
the same screen now reads `☰  My Tasks  [Pause] [⋯]`, whole title and all.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **790 tests, 0
  failures**; 5 new for the bar: a title that fits is untouched, one that does not is
  cut rather than pushed off, a centred title sits between the two ends, a `bottom`
  slot lands under the toolbar and not beside it, and a declared `leading_width` is
  counted in the folding budget.
- `cargo build --workspace --all-targets` — OK, no new warning.
- **On a physical device** (Huawei, Android 10): before and after screenshots of the
  demo's own bar, the title going from jammed-against-the-actions to whole and spaced.

## Still to do, and next

- **`Scaffold` and `body` have not been reviewed yet.** They are the other half of the
  same question — Flutter's `Scaffold` carries `extendBody`,
  `extendBodyBehindAppBar`, `resizeToAvoidBottomInset`, `drawer` as well as `endDrawer`,
  `persistentFooterButtons`, `floatingActionButtonLocation` — and frus's has none of
  them. That is the next milestone; putting it in this one would have made a change too
  large to check on a device in one pass.
- **`AppBar::build()` is still a builder**, not a `Widget`. Every other widget in the
  framework *is* one; this one has to be finished with `.build()` because it needs the
  available width before it can decide anything. Worth revisiting, but it is an API
  change with call sites, not a fix.
- **No header semantics.** The bar's title should carry `Role::Heading`; it carries
  nothing.
- **The overflow glyph and the ellipsis are hardcoded** and not localisable.
