# Changelog

All notable changes to frus are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) — with the usual 0.x caveat that
any release may break.

> frus is **pre-alpha** and **not on crates.io**. Releases are tagged source releases:
> depend on them by `path` or by git revision. For the reasoning behind any individual
> decision, the milestone notes in [`docs/milestone-*.md`](docs/) remain the authoritative
> record — one per step, 323 so far, each documenting the objective, the alternatives
> weighed, and the decision.

## [Unreleased]

### Added

- **`enabled` on `Slider`, `RangeSlider` and `Dropdown`** (J323). The other two of the five
  form controls milestone 322 named. A disabled slider takes no drag on its track, none on
  either thumb and no arrow key on a focused one — which is the case 322's guard widened
  itself for, a slider being neither tapped nor typed into. It is also the clearest picture
  of the container/content split: the track still to travel is a container at 12 %, the part
  travelled and the thumb are content at 38 %. A disabled `Dropdown` is **never open**,
  whatever `options(open, …)` was told, because a floating menu over a header that answers
  nothing traps a press and returns no message; its rows gained the semantics they never
  had.

- **One disabled state, shared** (J322). The new `frus_widgets::disabled` module holds the
  rule once — `disabled_container` at 12 % under `disabled_content` at 38 % — where five
  widgets had been writing both colours out by hand. The split is **container against
  content**, which the switch proves by taking both halves at once: its track flattens at
  12 %, its thumb at 38 %. A third colour, `disabled_mark`, is for a mark drawn *on* a
  disabled fill (a ticked checkbox's tick, an on switch's thumb) — 38 % on 38 % is not
  visible, so it punches through opaquely, as the reference's does.

- **`enabled` on `Checkbox`, `Switch` and `RadioGroup`** (J322). Three of the five controls
  a form is mostly made of had no way to be disabled at all. All three keep their answer —
  ticked, on, chosen — because read-only is not invisible. `RadioGroup` disables the whole
  group and derives its options rather than freezing them, so `.enabled(false)` at the end
  of a builder chain still reaches every one; `RadioOption` also gained the semantics it
  never had.

### Fixed

- **A disabled chip's delete cross was still in the tab order** (J322), and still announced
  as clickable. Milestone 320 gated its tap and stopped there — the same control reported
  three ways, two of them wrong. Found by the new guard on its first run:
  `every_control_with_an_enabled_flag_honours_all_four` reads the crate's own sources and
  insists every widget carrying `enabled` consults it in each hook it implements, drag and
  key hooks included, since a slider is dragged rather than tapped.

- **`enabled` on `Chip` and `SegmentedControl`** (J320). The gap milestones 312, 313 and
  314 each recorded. It turns off more than a colour: the press goes nowhere, no ink, out
  of the tab order, announced as disabled rather than falling silent, and still saying
  which segment is chosen — a disabled control is read-only, not invisible. A chip's
  **delete cross goes dead with it**, which would otherwise have been the one live control
  on an inert thing. Disabled **flattens** to `on_surface` at 12 % under a label at 38 %,
  as `Button` already did, rather than fading the accent: a pale accent reads as *quietly
  selected* where a grey one reads as *unavailable*.

- **`ThemeBuilder`** (J319). A widget that builds its subtree **from the ambient theme**,
  through a new `Widget::build_themed` hook called on the way down by the layout pass.
  It closes the hole milestone 318 ran into: `caller ?? theme ?? framework` reached
  painted properties and layout, but nothing decided while a composition was being
  *assembled* — `AppBar::build()` never saw a theme, and `center_title` picks which
  children exist rather than how they look. Unlike `LayoutBuilder` it keeps **retained
  state**: a theme is the same frame to frame where a box is not, so the subtree keeps
  its positional identity and an application bar inside one keeps its overflow menu open.

- **`AppBarTheme`** (J319). The first consumer. `center_title` resolves
  `caller ?? theme ?? platform` — the platform last, because where a title sits is a
  system convention before it is a design one — and the theme also carries the title's
  type, the background, the foreground, the elevation and the height.

### Changed

- **`Scaffold` no longer scrolls the body** (J321). **Breaking.** Every body was wrapped
  in a `Scroll`, so every screen scrolled whether or not it had anything to scroll, with
  no way to say otherwise. Scrolling comes from a widget the screen chooses — the
  reference's own body documentation says *consider using a `ListView`*, and consider is
  the whole point. That wrapper is what gave the home page a scrollable to light an
  end-of-list glow on in the first place; milestone 316 stopped it acting, this removes
  it. A body is now placed loose and top-aligned in the room the bars leave it: **a body
  that wants all of that room says `.flex(1.0)`**, and a body that may overflow goes
  inside a `Scroll` or a `List`. The keyboard, the safe area, `extend_body` and
  `extend_body_behind_app_bar` are unchanged — the bottom clearance is simply a sibling of
  the body now rather than padding inside a viewport.

- **The application bar is a fixed height** (J318). `AppBar` sized itself to whatever was
  in it, so a bar with two actions was taller than one without: every screen a slightly
  different shape, and the page moving the moment an action appeared. It is the
  reference's `toolbarHeight` — **64** — with `AppBar::height` still overriding. The title
  is `title_large` (22, was 20) and the actions are `label_large` (14, was 16), the
  reference's app-bar actions being text buttons.

- **`TextInput` follows the reference** (J317). The widget an application cannot avoid had
  nine hardcoded numbers and no way to change any of them. The measurements are read out of
  the reference now: content padding `(12, 8, 12, 8)` filled and `(12, 20, 12, 12)`
  outlined — the asymmetry being the room an outlined field's label takes **on** its top
  border — `body_large` for the value, `body_small` for the helper, 24 px icons, a 4 px
  radius, and a floating label that **scales** by 0.75 instead of taking a second fixed
  size, so a field given larger text keeps the relationship. The reference's two fields are
  both here at last: `filled()` is a tinted container with its top corners rounded and one
  line under it, not the outlined box wearing a tint. Fourteen settings per call or through
  the new `TextInputTheme`.

### Added

- **`dense` on `TextInput`** (J317). A field is 56 px tall, which is right for a form and
  wrong inside a table row. The reference's `isDense` gives the padding back —
  `(12, 4, 12, 4)` filled, `(12, 16, 12, 8)` outlined — without touching the shape, the
  label or the border. The editable table and the demo's grid use it.

- **`enabled` on `TextInput`** (J317). The gap three milestones in a row recorded as
  missing. A disabled field keeps 38% of its colours and still shows its value — the
  reference dims rather than hides — while leaving the tab order, refusing a caret,
  ignoring a key from a stale focus, showing no focus ring, and telling a screen reader.

### Fixed

- **One gesture, one axis, one area** (J316). Three things reported from a phone, all the
  same question. A finger scroll applied **both** deltas to the area under it every frame,
  so a page scrolling down drifted sideways the whole way — no finger travels in a straight
  line. A scrollable elsewhere has **one** axis and a gesture arena decides between them, so
  the axis is claimed once at the threshold and held to the end of the drag, the release
  included. The area is claimed there too: a strip that only slides sideways no longer
  swallows a drag straight down the page behind it, since the press now walks the whole
  stack of scrollables under the finger rather than taking the topmost. And an area whose
  content **fits** takes no drag at all — the rule the reference states outright, *only if
  there is content outside the viewport to reveal* — so the home page stopped lighting an
  end-of-content glow at edges it does not have, and a dead scrollable stopped swallowing
  presses meant for what is behind it. Pull-to-refresh and already-displaced content are
  the two exceptions, as they are there.

- **Scrolled multi-line text was painted over its own label** (J317). The content was
  clipped to the box rather than to the box *inside its padding*, so a scrolled field's
  text rode up onto the top border — which is where an outlined field's floating label
  sits. Wrong since the clip was written; six pixels of padding hid it and the reference's
  twenty did not.

- **The demo's drawer clears the status bar** (J316). Its content is in a `SafeArea`: a
  drawer is drawn edge to edge, and the "frus" title sat under the notch on the device.

### Added

- **`IconButton`** (J315). The widget milestones 313 and 314 kept working around: seven call
  sites had a button holding **one glyph** in a box sized for a word. It is the reference's
  — 40 × 40, circular, no fill, the glyph in `on_surface_variant` — with four variants
  (standard, filled, tonal, outlined), a `selected` state, and seven settings per call or
  through the new `IconButtonTheme`. It takes an icon from the bundled set **or a glyph**,
  since the set has thirteen shapes and applications draw more. `label` is what a screen
  reader announces, an icon having no text of its own. The date picker's month arrows, the
  navigation bar's back arrow, the stepper's plus and minus and the demo's delete crosses use
  it now.

- **One control, not three buttons touching** (J314). `SegmentedControl` built itself out of
  `Button`s — a filled one for the chosen segment, outlined ones beside it, two pixels apart
  — and had a single setting. It is one control now: **one** outline around the group with
  the reference's stadium ends, a hairline at each division, and the chosen segment filled
  with `secondary_container` and carrying a checkmark. Every segment takes the width of the
  widest, so renaming one does not move the divisions between the others, and the
  checkmark's room is reserved in all of them so the control does not change width as the
  selection moves. Eleven settings per call and through the new `SegmentedTheme`.
  **Breaking:** the colours, the shape, the height and the gap all change.

- **Five buttons, one of which has a shadow** (J313). `Button` had three variants and drew
  a shadow under **every** enabled one, which is the reference's *elevated* button drawn
  five times over. It has the reference's five now — `Filled` (the default), `Tonal`,
  `Elevated`, `Outlined`, `Text` — plus `Danger`, filled in the error role, which is the
  one variant that is frus's own. With them come its measurements: 64 × 40 minimum, a
  **stadium** shape whose radius follows the height, a `label_large` label, 24 px of room
  either side (12 for a text button), and elevation on the elevated variant alone.
  **Breaking:** `Variant::Primary` and `Variant::Secondary` are now `Variant::Filled` and
  `Variant::Outlined` — the old names described a surface rather than naming a button —
  and every button changes shape, size and shadow. Ten settings per call and through the
  new `ButtonTheme`.

  The framework's own one-glyph buttons (the date picker's arrows, the back arrow, the
  stepper's plus and minus) now ask for a 40 px circle, since `Button` is not an icon
  button and this framework has not got one yet.

- **The chip was a pill with nothing to say** (J312). `Chip` had two builders and painted a
  stadium filled with `muted` at 20 %: it could not be pressed, selected, given an icon or
  changed in any way. It is the reference's chip now — a 32 px rounded rectangle with an
  8 px radius, a `label_large` label, an `outline_variant` outline over no fill, filling
  with `secondary_container` and showing a checkmark once selected. The reference's four
  chip classes differ by **affordance** rather than shape, so they are builders here:
  `selected`, `leading`, `on_press`, `on_remove`, `show_checkmark`. Thirteen more settings
  per call and through the new `ChipTheme`. A pressable chip takes ink and focus, an inert
  one takes neither, and a selectable one announces whether it is on. **Breaking:** the
  shape, the colours and the size all change, and a chip that used to be a grey pill is now
  an outline.

  Fixed along the way: the delete cross was painted in a transparent colour and only
  appeared under the pointer — invisible at rest, and invisible full stop on a touch screen.

- **A tab bar, not a row of buttons** (J311). `Tabs` painted its header as `Button`s — the
  selected one filled, the others outlined — and had two builders, so nothing about it could
  be changed. It is a tab bar now: labels on the surface, a **sliding** indicator under the
  selected one (the runtime tweens the index, so the indicator's centre and width travel
  together), and a hairline dividing the bar from the panel. Both of the reference's
  variants are here — `TabsVariant::Primary`, whose indicator is as wide as the label and
  rounded, and `Secondary`, whose indicator spans the tab — with `variant`,
  `indicator_color`, `indicator_weight`, `label_color`, `unselected_label_color`,
  `label_style`, `divider_color`, `divider_height`, `label_padding` and `tab_height`
  settable per call and through the new `TabsTheme`. A tab now announces itself as a tab and
  takes ink. **Breaking:** the bar looks entirely different, and the twelve-pixel gap under
  it is gone — the hairline is what separates the bar from its panel.

  Fixed along the way: a widget animated through `Widget::anim_target` was painted **at
  zero** in any frame the runtime had not advanced, against the runtime's own documented
  rule that a widget seen for the first time adopts its target. A `Switch` rendered on its
  own came out off however it was set.

- **A theme for one subtree** (J310). New `Themed` widget: `Themed::new(theme, child)`
  replaces the theme wholesale for a subtree, `Themed::tweak(|t| …, child)` changes part
  of the one it inherits. Nesting composes, outer first. It reaches **layout** as well as
  paint — the swap happens in the layout walk and in the relayout cache's fingerprint of
  that walk, so the two cannot disagree — and a deferred overlay (dialog, drawer, tooltip)
  now **carries the theme it was declared under** rather than the root's. The layout
  direction comes from the ambient theme too, which reaches everything the walk decides,
  though not the flow of the rows around it (see the milestone note).

  Transparent-wrapper forwarding became a macro rather than a second hand-written copy;
  a test comparing it against the `Widget` trait immediately found `reorder_axis` and
  `reorder_draggable` missing from `Keyed`, where they had always been: a keyed board card
  dragged along the wrong axis, and a drop-only slot could be lifted. **Fixed.**

- **Per-widget theme defaults** (J309). `Theme` carries `widgets: WidgetThemes` —
  `CardTheme`, `DividerTheme`, `DrawerTheme`, `InkTheme` — so an application can say
  *every card in this app is flat* once instead of at every call site. Resolution is the
  reference's chain, `caller ?? theme ?? framework`; every field is an `Option` and a
  theme that sets nothing behaves exactly as none. The theme reaches **layout** as well
  as paint, through a new `Widget::style_themed` that defaults to `style` — a theme that
  stopped at paint could recolour a divider but not make one thin. The relayout cache
  fingerprints the themed style, so swapping themes recomputes geometry instead of
  serving the old one.

- **The card is three cards** (J308). `Card` had two fields — a padding and its child —
  and wrote everything else into `paint` as a literal, drawing a shadow **and** an
  outline, which is none of the reference's three. New `CardVariant::{Elevated, Filled,
  Outlined}`, with `Card::filled()` and `Card::outlined()`; the outline now belongs to
  exactly one of them. New `Card::{elevation, color, radius, margin}`, `CARD_MARGIN` and
  `CARD_ELEVATION`. Elevation is a **height**, not a blur radius: the blur and the drop
  are both derived from it, so the number means the same thing between widgets.
  **Breaking:** an elevated card has no hairline, sits on `surface_container`, and
  carries the reference's 4 px margin (`margin(0.0)` to refuse it).

- **A `Divider` that can be told anything** (J307). It was a unit struct with no fields
  and no builders, drawing its line by filling its whole one-pixel box. It now carries
  the reference's two separate measurements — `height`, the room the separator takes in
  the layout, and `thickness`, the line drawn inside that room, defaulting to 16 and 1 —
  plus `indent` and `end_indent`, which inset the **line** and not the box, and `color`.
  New `DIVIDER_SPACE` and `DIVIDER_THICKNESS`. **Breaking, and visibly:** a default
  divider takes 16 px of layout instead of 1. The old flush hairline is
  `Divider::new().height(1.0)`.
- **`Drawer::width`** (J307): the panel's width is a default now, not a constant nobody
  could change.

- **The ink ripple** (J306). A tap on a material surface now leaves a circle of ink that
  grows from under the finger, drifts towards the middle of the surface and fades —
  there was none at all before. The motion is the reference's, transcribed: fade-in
  75 ms, a radius that swells over a whole second while the finger is down and finishes
  in 225 ms once it lifts, a 375 ms fade-out that does nothing for its first 225, a
  starting radius of 30 % of the target and a final one 5 px past it, half the box's
  diagonal for the target, and `ease` on both the radius and the drift. New `InkWell`
  (a transparent box that splashes and clicks), `InkStyle`, `Widget::ink`, and
  `Runtime::ink`. `Button` takes ink; the colour and the rounding are the caller's, with
  the theme as the default. Ripples are painted over a surface's own paint and under its
  children, wired into the repaint-boundary cache so a splash inside a cached subtree
  animates instead of freezing.

### Fixed

- **A drawer is the reference's width, and an RTL test that was inverted** (J307).
  `DRAWER_WIDTH` was 280 where the reference is 304. Correcting it broke
  `rtl_flips_the_drawer_side`, which turned out to have been asserting the **opposite**
  of what an end drawer does — LTR on the left — and passing because a 280 px panel
  anchored to the right edge of a 200 px window starts at a negative `x`, leaving the
  strip its two pixel probes found on the *left*. The test now uses a window wider than
  a panel and measures where the drawer's colour actually is. Known and not fixed: a
  drawer wider than its window overflows instead of shrinking, where the reference
  enforces the width against the parent's constraints.
- **The divider is drawn in the theme's discreet outline** (J307), `outline_variant`
  rather than the full-strength `outline`, as the reference does — a separator as strong
  as a control's border competes with it.
- **An application bar's title follows the platform** (J306). `AppBar` started its title
  flush after the leading on every platform unless asked to centre it, and its own
  documentation argued that the choice was not one a bar could make. The reference makes
  it: `centerTitle ?? theme ?? platformCenter()`, where the platform centres on Apple's
  systems and only while there are fewer than two actions. `AppBar::center_title` now
  overrides a default that follows the target, resolved at compile time like
  `ScrollPhysics::platform_default`. **Breaking** on iOS/macOS builds, where a bar with
  at most one action now centres its title unless told otherwise. New
  `platform_centers_title`.

- **A `Scaffold` no longer moves the navigation when the window changes size** (J305).
  Reported from a device: turning the phone to landscape moved the navigation from the
  bottom of the screen to a rail on the left edge. `Scaffold::build` measured its own
  width, swapped a `BottomBar` for a `NavRail` past the `Compact` threshold, and took no
  parameter — so an application could neither ask for it nor decline it. The reference
  does not do this: its screen shell has one navigation slot, at the bottom, with no
  breakpoint anywhere in it, and a rail is a separate widget placed by whoever wants
  one. **Breaking:** the navigation is now a bottom bar at every width. New
  `NavPlacement::{Bottom, Rail}` and `Scaffold::nav_placement` pin a rail instead —
  fixed, at any width. There is deliberately no `Adaptive` variant: navigation that
  follows the size class is `NavScaffold`, a separate shell named for what it does and
  taking the `SizeClass` as an argument, so choosing it is a decision written down.
- **The overscroll glow is light, not a dent** (J301, J302). Reported from a device:
  *"I scrolled vertically, top to bottom. But it happens as if I pulled the sides too."*
  The glow was a **flat** fill — `Primitive::Path` carried a colour and nothing else, so
  a path could not fade — and a hard curved boundary drawn across the full width reads
  as the page being bent, because that is what a curved edge across a page normally
  means. Paths take a gradient now, aimed at points in the path's own space rather than
  at a bounding box, since the glow's ellipse is mostly off screen. J302 checked it on
  the phone and found the same defect one layer down: a **straight** fade reaches zero
  on a *line*, so the arc's boundary, which rises towards each flank, still cut the fade
  short and left an edge at each end. The gradient is **radial** there — distance
  measured in radii, so the far end of the fade is the ellipse itself — and it is
  resolved in the fragment shader rather than per vertex, a radial fade not being affine
  and an ellipse tessellating into triangles whose every corner sits on the boundary.
  New API: `Scene::fill_path_gradient`, `Scene::fill_path_radial`, `PathGradient`.
  `CustomPaint`, the charts and `ClipPath` can all fade now.
- **The renderer draws in the order the scene asked for** (J294). It drew one pass per
  kind of primitive — rect, image, path, text — so every path covered every rectangle in
  the frame, wherever the two sat in the scene. Found on a device in J291 (a filled button
  on a notched bar), and it applied to `CustomPaint`, the charts, `ClipPath` and the
  overscroll glow just as much. Primitives are now given a **level** from what they
  cover, and a level costs one draw call per kind it holds: a twelve-row list is 3 draw
  calls, where ordering primitive-against-primitive would have cost 25. Text was left out
  of the plan here and folded in by J295, below. `frus_gpu::draw_calls(scene)` reports the
  cost.
- **Text stops drawing through every overlay** (J295). J294 left text in a pass of its own
  above the frame — a `Primitive::Text` records where the text *starts*, not the box it
  fills — and wrote that down as a rule: covering text needs a layer. No widget in frus
  uses `scene.layer`, so in practice every menu, dropdown, dialog and sheet had the labels
  beneath it reading straight through. `Primitive::Text` and `Primitive::RichText` carry a
  `bounds` now, set by the widget walk from the box it is about to hand the widget, so the
  planner orders text like anything else. It cost nothing: that same twelve-row list is
  still 3 draw calls. Two things fell out of it — a rectangle's footprint no longer grows
  by its `blur` (the shader softens the edge *inside* the quad, so it was double-counted),
  and `TextInput` no longer paints its floating label before its own box, which only ever
  worked because text was painted last.
- **47 stale goldens, and the process that hid them** (J294). The golden suite had been
  red since J289 — a deliberate change to text measurement that moved glyphs by a pixel —
  and nobody saw it: the routine test command excludes `frus-test`, and the CI job that
  does run the goldens was wholly advisory. The goldens are re-blessed, and the GPU job is
  split so that everything asserting on numbers rather than pixels is required.
- **A golden of an implicitly animated widget pinned down the wrong picture** (J296).
  `render_widget` built its `Runtime` and painted at once, where the shell settles the
  implicit animations first — so `Switch::new(true)` was drawn exactly like
  `Switch::new(false)`. The harness now does what the shell's first frame does. No
  existing golden moved, because no such widget was in the suite; that was the actual
  problem, and it is what J296 is about.

### Changed

- **Asynchronous effects are actually asynchronous** (J303). `Command::perform_async`
  existed, and natively it was `std::thread::spawn` plus `pollster::block_on` — a
  thread per effect, and **no reactor**, so a future that waited on a timer or a socket
  parked its thread and nothing ever woke it. Only futures that were already ready or
  that blocked internally worked; asynchrony was in the type system and absent
  underneath it. There is now one executor on four worker threads, each running it
  inside `async_io::block_on`, which is the line that installs the reactor. Every
  asynchronous effect and every subscription is a task on it instead of a thread.
  Deliberately not tokio — `async-executor` + `async-io` is a scheduler and a reactor,
  small, pure Rust, and they run on Android; a future needing *tokio's* reactor still
  wants the application's own runtime, and letting frus be handed one is the next step.
  One consequence worth knowing: a **blocking** call inside `perform_async` now starves
  a small pool rather than wasting a thread of its own, so the `net` client moved to
  `blocking::unblock`. The rule is in `Command`'s own docs.
- **Text is measured once, not once a frame** (J300). J299's baseline said three
  quarters of the cost of building a frame was `frus_text::measure`, re-shaping through
  cosmic-text on every call for strings that had not changed. There is a cache now,
  keyed on `(text, size, weight, italic, max_width)` with the weight and style
  **resolved** — a Medium asked for on a family that only ships Regular hits the
  Regular's entry, which is the same shaping either way. Eviction is two generations
  and a promotion on hit: a string still on screen never falls out, a string that has
  gone leaves with its generation, and there are no timestamps. Registering a font
  empties the cache outright, an answer from before that being wrong rather than stale.
  `measure/line` 16.3 µs → **79 ns**. Measured against the same tree with every string
  replaced by a box of the size it would have taken — the only control that survives a
  machine having a slower day — building a twelve-row frame went from **4.1×** that
  tree to **1.20×**: text is a sixth of a frame now, where it was three quarters.
- **A performance harness, and what it found** (J299). `crates/frus-bench` measures
  `build_ui`, text measurement and wrapping, and batch planning, under the release
  profile. The baseline says something the roadmap had wrong: **three quarters of the
  cost of building a frame is measuring text**, which is re-shaped through cosmic-text
  on every call with no cache — a twelve-row screen spends ~290 µs of its 382 µs
  re-answering questions it answered 16 ms ago. Rebuilding the tree, the bottleneck the
  roadmap named, is the small half. The batch planner also turns out to be O(n²).
- **rustfmt, clippy and rustdoc are blocking checks** (J298). All three were advisory,
  each with a `TODO` naming the backlog to clear first: 40 unformatted files, 71 clippy
  warnings, twelve broken intra-doc links. Cleared, and `continue-on-error` dropped —
  J294 having shown what an advisory check that goes red is worth. New public API from
  the cleanup: `CellFn<Msg>`, the closure a table or board cell is built from, and
  `ValueAnim`, which was already the type of a public field.
- **A test harness that can run the clock** (J297). `frus_test::Stage` holds the
  retained state and steps the frame loop the way the shell does — every animation
  family, in the shell's order, with gestures going in through the shell's own entry
  points (`refresh_pull`, `dismiss_drag`, `glow_pull`). It is what a widget whose
  picture is a *gesture in flight* needs before it can be photographed at all, and with
  it **every widget module that draws now has a pixel test**: 86 of 86. `render_widget`
  is three lines on top of it.
- **The goldens cover the widgets, not just the tables** (J296). 58 of the 86 widget
  modules had no pixel test at all — `Card`, `Checkbox`, `Switch`, `Icon` and
  `Divider` among them — which is why two rendering defects in five milestones had to
  be found on a phone. 27 new goldens in `crates/frus-test/tests/widgets.rs` take that
  to 11, and the eleven left all need a state a static render cannot supply: a swipe
  in flight, a route transition, a pull, a glow.
- **The demo is 22 files, not one** (J293). It was 4,360 lines in a single `lib.rs`, and
  since it is the only large frus application anyone can read, it taught that an
  application has to be written that way. It does not: `model.rs` (the state and the
  questions worth asking it), `message.rs` (`Msg` alone), `update.rs` (`reduce`, the one
  place state changes), one file per screen under `screens/`, and a `prelude.rs` so a
  screen does not name thirty widgets before drawing one. No behaviour changed — same
  widgets, same 37 tests, same scene. New guide: `docs/app-structure.md`.

### Added

- **`Command::after(delay, message)`** (J303). A message on a real one-shot timer — a
  task on the reactor natively, a `setTimeout` on the Web. A hundred pending timers are
  a hundred queue entries rather than a hundred threads. It could not have existed
  before J303, because before J303 nothing in the framework could wait.
- **Pictures in the README, and a tool that makes them** (J304). A GIF of the demo
  moving between four screens, stills of the charts, the board, the data table and the
  light theme, and one real photograph of a phone. Everything but the phone is
  *rendered*, through the same pipeline a window uses:
  `cargo run -p frus-demo --features shots --bin shots -- docs/media`. Regenerating a
  picture after a change is now a command rather than an afternoon, which is the only
  way a README's screenshots stay true. New: `Snapshot::write_png`.
- **Fourteen issues, written to be startable** (J304), now [open on the tracker](https://github.com/KalybosPro/frus/issues). `.github/issues/` and
  `scripts/seed-issues.sh` — each one says why it matters, where in the code to look,
  how to know it is done, and where the obvious wrong turn is. They live in the
  repository so they can be reviewed like anything else, and so a fork does not start
  with an empty tracker.
- **An application weighs 4.9 MB, not 286** (J292). The demo installed at 286 MB because
  `cargo apk run` builds in **debug** and nothing stripped the `.so` on the way in. There
  is now a `[profile.release]` — fat LTO, one codegen unit, `panic = "abort"`, `strip` — in
  the workspace and in the project template, and a *Shipping* section in the guide that
  says to use it, with the signing step a release APK needs. Same code, 59× smaller.
- **The bundled fonts are a choice** (J292). 3.4 MB of faces, ~40% of a minimal app, split
  into four features (`bundled-sans`, `-italic`, `-mono`, `-arabic`), all on by default and
  forwarded to the facade so an application can drop what it does not draw — a megabyte off
  a counter. Dropping one is never a crash: `available_style` asks the database what it can
  serve, so italic without an oblique face comes out upright rather than panicking on
  Android, and a family nobody loaded resolves to the generic one. `frus::fonts::add_font`
  and `set_default_family` let an application ship its own faces instead.

- **A bottom app bar, cut with a notch** (J291). `BottomAppBar` carries the actions of the
  screen you are on, as against the navigation bar's choice of screen, and a docked FAB
  sits in a notch cut into its top edge. The notch is the **scaffold's** to cut, not the
  bar's — it is the party that knows where both are — which is why `bottom_app_bar` takes
  the bar by its own type rather than as an opaque widget. The curve is the reference's:
  two quadratics onto the button's circle and an arc between them, so the bar meets it
  tangentially. `Path::arc_to` came with it.
- **The FAB has a location, not a corner** (J290). `FabLocation` places it at either end
  or centred, **floating** clear of the bottom bar or **docked** astride its top edge —
  the placement a notched bar is cut for. `EndFloat` is what the scaffold already did, so
  nothing moves unless it is asked to. The reference's `mini` twins are not variants here:
  a mini FAB is a smaller button, and `fab_size(40.0)` docks it correctly. Docking needs a
  height the scaffold cannot measure, so it is declared — 56 px by default.
- **`Scaffold` and `body`, reviewed against the reference** (J288). New:
  `window_insets` (the system bars and the keyboard, kept apart because only one of them
  may be declined), `resize_to_avoid_bottom_inset`, `extend_body`,
  `extend_body_behind_app_bar`, a leading `drawer` beside the trailing `end_drawer`, and
  `persistent_footer` with its alignment. A slot the body is told to run under moves to an
  overlay layer rather than being drawn over, so nothing has to be measured; with neither
  flag set the assembled tree is unchanged.
- **`AppBar`, reviewed against the reference** (J287). New: `center_title`, `bottom` (a
  widget under the toolbar, inside the same background), `leading_width`, `title_spacing`,
  `foreground`, `elevation`. The layout policy is now stated and tested: the title keeps
  its natural width up to half the bar, the actions fold into the overflow to fit what is
  left, and truncating the title with an ellipsis is the last resort.
- **Shared-element transitions** (J286). `Hero::new(tag, widget)` on both sides of a route
  change makes the two one element: the transition flies it from where it was to where it
  is going, taking both originals out of the frame for the duration. What travels is the
  destination's own painting, lifted out of the frame — not a widget built for the
  occasion. An unmatched tag, or one used twice on a side, is left alone.
- **Drag and drop** (J285). `Draggable::new(w).payload(id)` and
  `DragTarget::new(w).on_drop(|payload| …)`: a general pair for carrying a thing onto
  another thing, where the two ends need not know of each other. A draggable **yields to a
  scrollable** underneath it, so a list never loses its scroll; inside one, `long_press()`
  is the lift, being the one signal a scroll cannot claim. What floats under the finger is
  the item's own painting, lifted out of the frame; what is left behind is faded, not
  removed. A target paints its own "drop it here" state from `Status::drag_over`, and one
  that refuses the payload is never offered the drop.
- **The constraint boxes** (J284). `SizedBox` (fixed, `expand`, `shrink`),
  `ConstrainedBox` (floors and ceilings on either axis, with `tight`/`loose`), `Intrinsic`
  (a box the size its content would *like* to be, with an optional step), and `OverflowBox`
  (a child laid out to constraints of its own, free to be bigger than its box and painted
  over its neighbours, with an `unconstrained` variant). `Style` gained the ceilings it was
  missing: `max_width` and `max_height`.
- **A paged view** (J283). `PageView::new(count, build)` scrolls panel by panel and never
  comes to rest between two: at the release, a spring to one page replaces the fling. The
  rule is the one paged views everywhere use — any release with speed is a flick and turns
  the page, however short the drag; only letting go slowly rounds to the nearer one. Pages
  are virtualised, `page(n)` and `on_page_changed(msg)` bind both directions to a single
  number held by the application, and past an edge the platform's ordinary physics takes
  the content home.
- **Swipe to dismiss** (J282). `Dismissible::new(row).on_dismiss(msg)` slides a row aside,
  flies it out past 40 % of its width (or on a fling), then collapses its box so the
  neighbours close the gap before the message goes out. Inside a list, the shell arbitrates
  the shared gesture by **direction** at the drag threshold: along the item's axis is a
  swipe, across it is a scroll, and the loser never sees the gesture. A fling must beat the
  other axis by a clear margin, so a hurried scroll cannot throw rows out.
- **Pull to refresh** (J281). `Refresh::new(list).on_refresh(msg).refreshing(flag)` turns a
  drag past a scrollable's top edge into a message, using the movement the physics already
  refuses — no new measurement in the gesture path. The threshold is proportional to the
  scrollable (25 % of its extent, armed at two thirds of that), an armed pull only ends by
  letting go, and the indicator spins for exactly as long as the application says it is
  working. Where a `Refresh` listens, the top overscroll glow stands down.
- **An ambient description of the surface** (J280). `MediaQuery::of()` gives any widget
  built during `view` the surface size, the DPI scale, the app density and the occupied
  edges — system bars, notch, soft keyboard — with nothing threaded down from the
  application. The framework installs it around `view`; `MediaQuery::scope` re-describes it
  for a subtree, nests, and restores itself even through a panic.
- **`SafeArea`** (J280), which insets its child away from those occupied edges. Per-edge
  (`Edges`), with `minimum` as a floor rather than an addition, and the soft keyboard left
  alone unless `avoid_keyboard()` asks for it. `SafeArea::build` consumes the padding it
  applies, so safe areas can nest without avoiding the same notch twice.
- **Widgets that withhold part of the frame** (J280): `IgnorePointer` (invisible to input,
  which falls through), `AbsorbPointer` (invisible to input, which stops there),
  `Visibility` (hidden, optionally keeping its box, its input or its announcements),
  `Offstage` (gone from the layout entirely) and `ExcludeSemantics`. All five go through one
  mechanism, `Widget::barrier`, applied *after* the subtree is walked — so a target
  registered several levels down is withheld just as surely as one at the top.
- **The overscroll glow** (J279). A platform that clamps now answers a refused drag or a
  fling landing on an edge with an arc of light, instead of with silence: `OverscrollGlow`,
  fed by the movement `apply_boundary_conditions` refuses, by a ballistic stopped at an edge,
  and by a wheel notch past the end. Bouncing physics feeds none of it — the bounce is
  already the answer. `Path::oval` and `Curve::Decelerate` in `frus-core` came with it.
- **Real velocity estimation** (J278). `VelocityTracker` keeps the last 20 pointer
  positions and fits a quadratic through them, so a gesture that slows down as the finger
  lifts is still read as the throw it was; the platforms that bounce instead use a weighted
  average of the last three sample velocities, picked by `VelocityTracker::platform_default`.
  `VelocityEstimate` carries the travel alongside the speed, and `PolynomialFit` exposes the
  least-squares solver.
- **Scroll physics per platform** (J277). `ScrollPhysics::{Bouncing, Clamping}` names how a
  scrollable behaves at its edges and after a fling, and defaults to what the running
  platform does. Clamping follows the platform's spline deceleration and stops dead at the
  edge; bouncing resists progressively past the edge (a real rubber band) and springs back.
  Set it app-wide with `Application::scroll_physics`, or per area with `Scroll::physics` /
  `List::physics`.
- `ClampingScrollSimulation` and `BouncingScrollSimulation` in `frus-core`, plus
  `FrictionSimulation::time_at_x`.

### Fixed

- **A wrapping text reserved one line and painted two** (J289), found on a device in J286
  and misdiagnosed there. The cause was half a pixel: the measurement returned the shaped
  width as it came (146.4), the layout rounded the box down to 146, and the text — shaped
  again at 146 when painted — wrapped onto a line the layout had not reserved, so the next
  thing sat on top of it. Text measurements now round **up** (clamped back to the
  constraint when there is one), in `measure_wrapped` and `measure_runs_wrapped` alike. It
  only showed on a text sized to fit, where the box comes from the measurement itself.
- **A persistent footer ignored its alignment** (J288) — the same defect the app bar had in
  J287, and found the same way. The row hugged its content, so there was no free space for
  the alignment to place anything in. An alignment is a claim on free space: whoever aligns
  must first be told how much there is.
- **A scaffold with no bottom bar ran its body under the system navigation bar** (J288).
  The body's bottom clearance falls to whoever is last in the column, and with no bar and
  no footer there was nobody. It is the scroll **viewport** that shrinks, not the content
  that gets padded — otherwise the last field of a form sits under the keyboard with empty
  space behind it, unreachable.
- **`Scaffold::fab` no longer warns that it intercepts clicks** (J288). It never did: only
  a widget that asks for clicks enters the hit registry, so the transparent remainder of an
  overlay layer is not a target. Now tested rather than warned about.
- **A wide empty band above the app bar on Android** (J288). The safe area is derived from
  the space the system leaves the activity, and the default theme reserves an action bar the
  app never draws: the shell read those 56dp as a system inset and padded them away on top
  of the status bar — 143 physical px of nothing on the demo's phone. The manifests (and the
  project template) now ask for `Theme.DeviceDefault.NoActionBar`.
- **The app bar hugged its content instead of occupying its width** (J287) — true since it
  existed. `background(color)` painted a stripe behind the text rather than across the bar,
  and nothing could be centred for want of free space.
- **A long app-bar title pushed the actions off the edge** (J287). It is now cut with an
  ellipsis, and only after the actions have folded.
- **The app bar's leading slot was a fixed 56 px** (J287) whatever widget was in it, so a
  wider one silently broke the folding budget.

- **A wrapper that nests must forward the structural questions too** (J285) — the mirror of
  the `Keyed` bug fixed in J282. A layout leaf (`Dismissible`, `Stack`) wrapped in an
  ordinary container had no content size, so it resolved to zero on the wrapper's main axis
  and vanished silently. `Draggable` and `DragTarget` now forward `stack()` and
  `continuous()` from their child.
- **A stack's layers are given their box** (J285) rather than asked what size they would
  like, which is what "each layer fills the box" always meant. An unsized layer used to hug
  its content and collapse to nothing, invisibly.
- **One hold cannot mean two things** (J285): a long-press *message* and a hold-to-lift on
  the same widget are now arbitrated — the lift wins — instead of both firing.

- **`Keyed` now forwards the structural questions** — `stack`, `continuous`,
  `draws_own_focus`, `repaint_boundary` (J282). A transparent wrapper that answered them for
  itself changed how its content was laid out: any keyed stack had its layers put in flow
  instead of on top of one another, and any keyed continuously-animating widget quietly
  dropped frames. Found on a device.
- **`MediaQuery` reuses `frus_core::Orientation`** instead of the duplicate enum the first
  draft declared (J280).
- The last two French assertion messages in `clip.rs` are English (J280).
- **A scroll offset now has one owner at a time** (J279). The edge spring kept retracting an
  overscrolled offset *while a finger was still holding it*, so a rubber band was dragged
  home as fast as it was stretched — on a slow drag it never appeared at all. Found on a
  physical device, not by a test.

### Changed

- A fling now needs **distance as well as speed**: a fast twitch that covered less than the
  drag threshold no longer throws the content, per axis (J278).
- The drag threshold follows the pointer: 18 logical px for a finger, 1 px for a mouse or
  trackpad, where it used to be 8 px for everything (J278).
- The scroll registry exposes `Ui::scroll_regions()` / `scroll_region(id)` returning a
  `Scrollable` (id, viewport, bounds, physics); `Ui::scroll_hit` returns one too, instead of
  a tuple.
- The wheel's elastic overshoot now exists only where the physics allows overscroll.

### Removed

- `gesture::fling_destination` and its constants, superseded by the physics layer.

## [0.1.0] - 2026-08-08

The first tagged release: a point in the history that can be cited, cloned and diffed. It
is not an API commitment — nothing here is stable, and the public surface will move.

277 milestones went into it, so this entry groups **what exists**, not what changed since a
previous release; there is no previous release.

**What it is not, yet:** nothing is published to crates.io, there is no MSRV, the web target
has no clipboard, accessibility or live reload, iOS compiles but does not run, and `view`
rebuilds the whole tree every frame (no memoisation). See [ROADMAP.md](ROADMAP.md).

### Added

**Platforms**

- **Desktop** (Windows / Linux / macOS) on winit + wgpu — clipboard, screen-reader
  accessibility through AccessKit, live reload in dev.
- **Android** — native activity, Vulkan, a real IME (composition and swipe), window insets,
  and the full application lifecycle. Validated on a physical device (J50).
- **Web** (wasm + WebGPU) — rendering, input, animation, subscriptions, and asynchronous
  effects including `fetch`.
- **iOS** — groundwork only (J276): named platform `cfg`s, a `run()` entry point, and CI
  compiling both Apple targets.

**Application model**

- The **Elm architecture**: `Application { update, view }`, with `update` pure and
  synchronous. Every effect goes through `Command`; every long-lived stream through
  `Subscription`. This is what makes 717 tests run with no GPU and no window.
- **One entry point** (J267) — `frus::main!(App::default())` generates the desktop, Android
  and wasm entry points from a single declaration.
- **The `frus` facade** (J268) — an application declares exactly one dependency.
- **Asynchronous effects** (J270) — `Command::perform_async` / `run_async`, so a real
  `await` works even on the single-threaded web target.
- **Networking** behind the `net` feature (J271–J272) — a cross-platform `fetch` with one
  signature on all three targets, and a `Request` builder for methods, headers, bodies and
  timeouts.
- **Typed JSON** behind the `json` feature (J275), and **`RemoteData<T, E>`** (J274), the
  Elm idiom for the four states of a request.

**Rendering and layout**

- A **wgpu** renderer: rounded rects, borders, shadows, gradients, vector paths tessellated
  with `lyon`, images, opacity, clipping, layer compositing, MSAA, and offscreen rendering.
- **Flexbox and grid** layout over `taffy`, with a relayout-boundary cache and frame phases
  so a hover repaints without relaying out.
- **Text** shaped by `cosmic-text`: rich spans, wrapping, intrinsic sizes fed back into
  layout, caret, hit-testing, selection, and bidirectional scripts.

**Widgets and interaction**

- A widget library of ~80 modules — text fields with IME, data tables, an editable grid,
  charts, date and time pickers, a kanban board, trees, lists, tabs, toasts, modals and
  portals, a drawer, navigation with a scaffold, and a validated multi-step form.
- **Interaction**: focus and keyboard navigation, long press, fling, drag-and-drop
  reordering with a live preview, the back gesture, and spring-driven navigation.
- **Theming** — a Material 3 colour scheme with roles, a type scale, and state layers.
  Every widget's styling and slots are overridable; the defaults are themed, never
  hardcoded.
- **Animation** — a unified `trait Simulation` (spring, friction, clamped) driving scroll
  momentum, sheets and transitions from one path.

**Cross-cutting**

- **i18n / l10n / RTL** via Fluent bundles, locale negotiation, and layout mirroring.
- **Accessibility** — a semantics tree bridged to AccessKit.
- **`frus-test`** — headless rendering, snapshots, and golden-image comparison (87 goldens).
- **Tooling** — a `cargo generate` template, a runtime tree inspector, and state-preserving
  live reload in dev.

### Project

- `README.md`, `README.fr.md`, `CONTRIBUTING.md`, `ARCHITECTURE.md`, `ROADMAP.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, dual MIT/Apache-2.0 licences, issue and PR templates,
  and CI covering the three platforms plus an iOS compile job.
- The repository is entirely English — code, comments, documentation and commit messages —
  with `README.fr.md` as the one deliberate exception.

<!--
For releases, use this shape:

## [0.1.0] - YYYY-MM-DD

### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->

[Unreleased]: https://github.com/KalybosPro/frus/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/KalybosPro/frus/releases/tag/v0.1.0
