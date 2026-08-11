# Milestone 280 — An ambient surface, and widgets that withhold

## Two gaps that were really one

An application could not answer two ordinary questions about its own frame.

**Where does the screen actually end?** The shell knew — it computes the system
insets on every resize — but the only way that reached an application was
`Application::on_insets`, which hands them to the *app object*. Anything deeper had to
be given them by hand, screen by screen, widget by widget. Avoiding a notch cost a
field in the state, a parameter on every `fn …_screen(…)`, and a chance to forget.

**How do I make this part of the tree stop taking input?** There was no answer at all.
A form that must go inert while a save is in flight had to rebuild itself with every
`on_click` removed; a screen behind a busy indicator could not stop a stray tap
reaching the button underneath.

Both are the same shape of gap: information and policy that apply to a *region* had no
way to travel with that region.

## The ambient surface

`MediaQuery` describes the surface being drawn on — size, DPI scale, app density, the
permanently occupied edges (bars, notch) and the transient ones (the keyboard). The
framework installs it around every call to `view`, and any widget built during that
call reads it:

```rust
let mq = MediaQuery::of();
if mq.orientation() == Orientation::Landscape { … }
```

Nothing is threaded. Nothing is stored twice. The shell assembles it in one place from
values it already had.

**Why a scoped ambient rather than a build context.** Widgets here are ordinary Rust
values built eagerly by `view`; there is no context object to look an inherited value
up from. So the scope is *dynamic* — it covers whatever runs inside a closure:

```rust
MediaQuery::new(size).with_insets(insets).scope(|| app.view(&theme, w, h))
```

Two properties matter and are tested. It **nests**: an inner scope restores the outer
one, not the default. And it is **panic-safe**: the restore is a `Drop`, so one
exploding frame cannot leave a stale surface installed for every frame after it — the
kind of bug that would look like a rendering problem for hours.

Outside any scope, `of()` returns `MediaQuery::UNSET` rather than panicking. A unit
test that constructs a widget directly still builds; it simply has nothing to avoid.

### `SafeArea`

```rust
SafeArea::new(screen)                                     // every edge
SafeArea::new(list).edges(Edges::ALL.without_bottom())    // run under the gesture handle
SafeArea::new(form).avoid_keyboard()                      // and move up for the keyboard
```

`minimum` is a **floor, not an addition**: `minimum(20)` against a 28 px status bar
gives 28, not 48. That is what lets one widget express both "clear the notch" and
"give this screen its margin".

The keyboard is **not** avoided by default. A screen whose content scrolls wants the
keyboard handled by scrolling the focused field into view, not by squashing the whole
screen; `avoid_keyboard()` is for the short form that does not scroll.

**Nesting.** A `SafeArea` consumes the padding it applies. `SafeArea::build(|mq| …)`
builds its child inside a scope where the consumed edges are already zero, so a second
`SafeArea` further down adds nothing. `SafeArea::new(child)` cannot do that — its child
is already built by the time the widget exists — so it pads and says nothing about it.
That is the right constructor for one safe area at the root of a screen and the wrong
one when screens compose; the doc comment names the hazard rather than hiding it.

## Widgets that withhold

Five widgets, one mechanism.

| | laid out | painted | takes input | blocks what is behind |
|---|---|---|---|---|
| `IgnorePointer` | yes | yes | no | no — input falls through |
| `AbsorbPointer` | yes | yes | no | yes — input stops here |
| `Visibility` hidden, keeping its size | yes | no | no | no |
| `Visibility` hidden | no | no | no | no |
| `Offstage` | no | no | no | no |
| `ExcludeSemantics` | yes | yes | yes | n/a |

The mechanism is `Widget::barrier() -> Option<Barrier>`, four flags for what a subtree
may not contribute: input targets, the barrier's own absorbing target, primitives,
accessibility nodes. The walk visits the subtree **normally** and then drops what it
added to the selected registries.

**Dropping afterwards, rather than skipping beforehand, is the whole design.** A
widget deep inside registers its click target, focus stop, scrollable area, drag handle
or accessibility node without knowing that something above is holding the subtree out
of the frame. Truncating at the barrier catches every one of them — including the ones
added by widgets written after this code, which is the part a skip-list would get wrong
six months from now. It also leaves the walk's rect indexing untouched, which a skipped
subtree would break outright.

Two details fell out of it:

- **An absorbing barrier is a hit target with no message.** `Hit.msg` became
  `Option<Msg>`, and `AbsorbPointer` pushes `None` over its own box after clearing its
  subtree. Since the hit test takes the topmost match, that entry wins and yields
  nothing — which is exactly "swallowed".
- **The modal focus scope is an index into `focusables`.** A barrier that truncated
  past it would leave it dangling, and every later slice of the focus pool would come
  back empty — no focusable at all, instead of the ones the barrier meant to spare. It
  is clamped to the new length.

`IgnorePointer` keeps its subtree's **accessibility nodes**: a form that is momentarily
inert is still worth reading out. `ExcludeSemantics` is there for when it really should
not be reachable.

`Offstage` and a plainly hidden `Visibility` take the child out of the tree entirely,
so they hold no retained state — a caret, a scroll offset — and start fresh on return.
`Visibility::maintain_size` is the one to reach for when the box has to stay: it is what
stops a row twitching as a spinner comes and goes.

## One thing corrected on the way

`frus-core` already had an `Orientation` with `from_size` and the same
square-counts-as-portrait convention. The first draft of `MediaQuery` declared its own,
which compiled right up to the re-export and would have left the framework with two
identical enums the day one of them grew a variant. `MediaQuery::orientation` delegates
to the existing one.

Two stray French assertion messages in `clip.rs` are now English, in line with the rest
of the tree.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **709 tests, 0
  failures** (677 at milestone 279): 15 for the barriers, 8 for the ambient surface, 9
  for `SafeArea`.
- `cargo build --workspace --all-targets` — OK.
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK.

The barrier tests are about behaviour rather than shape: input falls *through* an
`IgnorePointer` to a layer underneath, and *stops at* an `AbsorbPointer`; a target three
levels down is caught; a `Scroll` inside a barrier stops scrolling; an ignored subtree
is still painted and still announced; a hidden-but-sized child is neither drawn nor
clickable while still occupying its box; and `maintain_interactivity` survives the
barrier.

## What's left

`MediaQuery` carries no **platform brightness** and no **system text scale**, because
the shell does not query either. Inventing values for them would be worse than their
absence; they belong with the platform work that can actually read them.

Neither `IgnorePointer` nor `Visibility` stops a subtree's **animations** from asking
for frames. Nothing is drawn, so nothing is visibly wrong, but a hidden spinner still
keeps the frame loop awake. That wants a `wants_animation` scope, not a fourth flag.

The wider gap this milestone starts closing is the catalogue itself. Still missing, in
rough order of how often they are reached for: pull-to-refresh and swipe-to-dismiss on
lists; a paged view; a shared-element transition between screens; the constraint boxes
(`ConstrainedBox`, `LimitedBox`, `OverflowBox`, intrinsic sizing, baseline alignment);
and a general drag-and-drop pair to sit under the reorder machinery that already exists.
