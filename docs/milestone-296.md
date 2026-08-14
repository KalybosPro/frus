# Milestone 296 — The goldens cover the widgets, not just the tables

Two rendering defects in five milestones were found on a phone rather than by the
test suite: the notched bottom bar erasing a filled button (291), and text drawing
through every overlay (294, fixed in 295). Both times the note said the same thing —
"nothing in the suite draws this". This milestone stops saying it.

## What the suite actually covered

Measured rather than guessed: for each of the 86 widget modules, whether any golden
test names anything the module exports.

**58 of 86 had no pixel test at all.** The suite was 77 images deep on tables,
charts, forms and pickers — where the bugs had been — and empty everywhere else.
`Card`, `Checkbox`, `Switch`, `Icon`, `Divider`, `Stack`, `Grid`, `List`, `Drawer`,
`AppBar`, `Image`, `RichText` — none of them had ever had a picture taken.

That is the whole explanation for the two device findings. The suite could not have
caught either one, because neither drew a widget the suite knew about.

## 27 new goldens, and what they are grouped by

`crates/frus-test/tests/widgets.rs`. Widgets are grouped by what they are rather than
given one image each: a golden holding the three toggles in every state is a better
test than three holding one, and there is less to read when one of them moves. The
overlays get their own group, and each one draws text underneath it — the 295 defect,
in the suite this time.

**58 uncovered modules → 11.** The eleven left all need a state that a static render
cannot supply: a swipe in flight (`Dismissible`), a drag (`DragSource`), a route
transition (`Navigator`, `NavScaffold`, `Hero`, `Keyed`), a page offset (`PageView`),
a pull (`Refresh`), a glow at an edge (`OverscrollGlow`), and the two that read an
ambient size (`LayoutBuilder`, `Responsive`). Those want a harness that can drive the
runtime forward, which is its own piece of work.

## The harness was rendering a frame no shell ever produces

The very first golden of `Switch` came out with `Switch::new(true)` and
`Switch::new(false)` **drawn identically** — knob left, track grey, both of them.

The switch paints from `status.value`, its animated position, and the harness built
its `Runtime` and painted immediately. The shell does not: its frame settles the
implicit animations first, and `Runtime::advance_values` gives a widget seen for the
first time its target with no transition. Nothing had run, so every implicitly
animated widget was frozen at zero.

`render_widget` now does what the shell's first frame does, with `dt` of zero — the
point is the adoption on mount, not the passage of time:

```rust
runtime.advance_values(root, 0.0);
runtime.advance_colors(root, 0.0);
runtime.advance_sizes(root, 0.0);
runtime.advance_radii(root, 0.0);
runtime.advance_paddings(root, 0.0);
```

This is a harness bug, not a framework one — on a device a switch that is on is drawn
on. But it means **any golden of an implicitly animated widget would have pinned down
the wrong picture**, and it went unnoticed for the same reason as everything else
here: no such widget was in the suite. All 77 existing goldens are byte-identical
after the change, which is the proof of that.

## What reading the images turned up

Every one of the 27 was looked at before being accepted, in contact sheets. Four
things came back:

- **A `NavBar` with nothing to fill shrinks around its back button** and paints its
  centred title underneath it. Its `paint` centres the title in `bounds`, which only
  makes sense at full width, but its `style` is `Dimension::Auto`. A screen always
  gives it a width, so it has never shown; the test gives it one and says why.
- **A `RichText` that wraps needs a width to wrap inside.** In an `Auto` container it
  measures unbounded and runs off. The demo already works around this by computing
  its content width by hand. This is the `LayoutBuilder`-shaped hole the roadmap
  tracks, seen once more.
- **`Alert`'s info and success kinds are near enough the same green** to be hard to
  tell apart. Not a defect — the theme's primary *is* green — but worth seeing.
- A dead `batch_count` on `Painters`, left by milestone 294, is removed.

## Verification

- `cargo test -p frus-test --test widgets` — **27 passing**, and all 27 read.
- `cargo test -p frus-test --test goldens --test clip --test multiline_scroll --test
  transforms` — **84 passing, every image byte-identical**. The harness change moved
  nothing that was already blessed.
- The workspace suite is unchanged.
