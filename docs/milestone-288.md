# Milestone 288 — The scaffold and the body, and who owns the bottom edge

The other half of milestone 287. The app bar was reviewed against the reference; the
shell it sits in was not, and that is where the interesting questions live — not "what
slots exist" (frus already had most of them) but **what the body is given, and what it
is given *under*.**

## First, a defect the review did not find — the device did

While reading the bar next to the shell, the phone showed a wide empty band above it.
It was not the bar, and it was not the scaffold. It was the **safe area**.

The shell derives the safe area from the space Android leaves the activity. That space
excludes every decoration the *theme* asks for — and the default theme reserves an
action bar, 56dp of it, which a frus app never draws because it draws its own. The
shell read those 56dp as a system inset and padded them away, on top of the status bar:

| | measured on the device |
|---|---|
| status bar, actually | 72 physical px |
| content rect top, reported | **215** physical px |
| inset handed to the app | 84.3 logical px, i.e. 215 |
| after the fix | 72 physical / 28.2 logical |

143 px of nothing. The fix is one line of manifest metadata —
`theme = "@android:style/Theme.DeviceDefault.NoActionBar"` — applied to every crate
here **and to the project template**, so it is not something each new app has to
rediscover. Committed separately (`cb37d4d`), because it is a platform bug and not a
widget decision.

Worth stating plainly: nothing in the test suite could have caught this. The safe area
comes from the platform, and the platform was answering a question about a widget that
does not exist.

## What the reference has, and what was taken

| taken | as |
|---|---|
| `extendBody` | `extend_body(bool)` |
| `extendBodyBehindAppBar` | `extend_body_behind_app_bar(bool)` |
| `resizeToAvoidBottomInset` | `resize_to_avoid_bottom_inset(bool)` |
| `drawer` (leading) | `drawer(panel, open, toggle)` |
| `persistentFooterButtons` | `persistent_footer(widget)` |
| `persistentFooterAlignment` | `persistent_footer_alignment(Justify)` |
| — | `window_insets(WindowInsets)`, the input the three above need |

Not taken: `floatingActionButtonLocation` and `floatingActionButtonAnimator` (the FAB
has one corner here; a *location* is a small design, not a parameter),
`drawerEdgeDragWidth` / `drawerEnableOpenDragGesture` (frus's drawers are opened by a
message, not by an edge drag — that is a gesture milestone), `primary`,
`restorationId`, and the `on…Changed` callbacks, which in an Elm loop are the message
the application already sends itself.

## The one that was not a parameter: `window_insets`

`resize_to_avoid_bottom_inset` cannot be honoured by a scaffold that is handed a single
`Insets`. The keyboard and the navigation bar arrive as one number, and only one of
them may be declined — no setting should let content sit under the navigation bar.
`frus-core` already split the two (`WindowInsets { padding, view_insets }`); the
scaffold simply had no way to be told. Now it does, and `insets(…)` keeps its old
meaning as the padding half.

The clearance is then `padding.bottom.max(view_insets.bottom)`, never a sum: the
keyboard covers the navigation bar rather than stacking on top of it.

## Who yields to whom, again

The bar had one contested row; the scaffold has one contested **column**, and the same
kind of question. Three answers, and each is a decision rather than a mechanism:

**The body's bottom clearance falls to whoever is last.** With a bottom bar or a
persistent footer under it, they hold the edge off and the body needs nothing. With
neither, it is on the body. That last case was quietly wrong before this milestone: a
scaffold with a body and no bottom bar ran its content under the system navigation bar.

**It is the viewport that shrinks, not the content that gets padded.** The body is
inside a `Scroll`; padding *inside* it would leave the last field of a form under the
keyboard with empty space behind it, unreachable by scrolling. The clearance is a
sibling of the scroll, so the viewport itself is shorter. This is the whole difference
between "resize to avoid" and "add some space".

**Extending is a move, not a flag.** A slot the body is told to run under does not stay
in the body's column with the body drawn over it — it *leaves* the column for an
overlay layer drawn on top. So no height has to be measured anywhere: the body fills
what the column gives it, and a spring in the overlay puts the bar back at the bottom.
The same widget, in one place or the other.

Consequently, with neither flag set the assembled tree is exactly what it was before —
the overlay layer is not created at all. A screen that did not ask for any of this
cannot have been changed by it.

One restriction, deliberate: the **bottom** slots only move on a compact layout, where
the bottom bar actually is one. Wide, the navigation is a rail *beside* the body, and
a full-width overlay would cross it — a body sliding under a side rail is nobody's
design. `extend_body_behind_app_bar` has no such restriction; the bar spans the
content either way.

## The same bug as the bar's, found the same way

The first build put the wizard's *Next* at the **left** edge, though the footer's
default alignment is `End`. Exactly the defect milestone 287 fixed in the app bar: the
row hugged its content, so the alignment had nothing to distribute. A row can only
place things within a width it has been *given*.

Two occurrences in two milestones is a pattern worth naming: **an alignment is a claim
on free space, so whoever aligns must first be told how much there is.** The footer's
row is now given `width − insets − rail − padding`, and a test walks the three
alignments and checks the mark lands where each says.

## An old warning, retired

`Scaffold::fab` has carried this since milestone 52b:

> ⚠ **Experimental**: the FAB is overlaid through a full-screen `Stack` layer, and such
> a top layer **intercepts the clicks** of the bottom half of the screen.

It does not, and it is worth being precise about why: only a widget that *asks* for
clicks is entered in the hit registry, so the transparent remainder of an overlay layer
is not a target at all. There is now a test that clicks the body through two overlay
layers (the FAB's and an extended body's chrome) and gets the body's message. The
warning is replaced by the explanation.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **798 tests, 0
  failures**; 7 new for the scaffold: the keyboard shortens the body and does not when
  the screen declines, a body alone still clears the navigation bar, an extended body
  gains the bar's height, an extended body starts above the app bar, the footer sits
  between the body and the bar, the footer is aligned where it was asked to be, and an
  overlay does not swallow the body's clicks.
- `cargo build --workspace --all-targets` — OK, no new warning.
- **On a physical device** (Huawei, Android 10): the band above the app bar, before and
  after; and the demo's sign-up wizard, now a `Scaffold` whose Back / Next buttons are
  a persistent footer. *Next* sits at the trailing edge (it sat at the leading one on
  the first build — see above), stays put while the steps scroll, and with the keyboard
  up — a 447 px bottom inset — the form is shortened and the footer rides just above it
  rather than being covered.

  One caveat, stated because it is easy to read the screenshot as more than it shows:
  the demo applies the safe area in its own `view`, so what the device confirms is the
  **footer's** behaviour under a keyboard. That the *scaffold* is the one shortening
  the body — `window_insets` plus `resize_to_avoid_bottom_inset` — is covered by the
  tests, not by that screenshot.

## Still to do

- **The FAB has a corner, not a location.** No docked or notched variants either. That
  is a design (where can it sit, and what does the bar do about it), not a parameter.
- **An extended body is not told what it is running under.** The reference passes the
  bar's height down as ambient padding so a list can pad itself; frus has the ambient
  description (`MediaQuery`) but does not yet write into it from the scaffold. Until
  then, a screen that sets `extend_body` pads its own content.
- **Drawers open by message only.** No edge-drag to open, so `drawerEdgeDragWidth` and
  friends have nothing to configure yet.
- **`AppBar` is still a builder**, not a `Widget` — unchanged from milestone 287, and
  still an API change with call sites rather than a fix.
