# Roadmap

Where frus is, where it's going, and — the point of this document — **what is unclaimed and ready for you to pick up**.

Items are tagged:

- 🟢 **Good first issue** — self-contained, clear success criterion, no deep context needed
- 🟡 **Help wanted** — meaty, well-scoped, needs some familiarity with a subsystem
- 🔴 **Design first** — open a discussion before writing code; the shape isn't settled
- 🔵 **Claimed / in progress** — talk to the maintainers before duplicating

If something here interests you, **comment on the matching issue** (or open one) before starting.

---

## Where we are

326 milestones in. The framework runs real, non-trivial applications on desktop and Android, and functional ones on the web. What exists is genuinely built, not stubbed: layout, text with IME, drag-and-drop with live reflow, data tables, charts, pickers, navigation with spring transitions, theming, i18n/RTL, accessibility, animation, async effects with typed JSON, and golden-image testing.

What does not exist is everything around it: distribution, tooling, more platforms, and the ecosystem.

| | Desktop | Android | Web | iOS | macOS native |
|---|---|---|---|---|---|
| Rendering | ✅ | ✅ | ✅ | — | — |
| Input & gestures | ✅ | ✅ | ✅ | — | — |
| Text input / IME | ✅ | ✅ | ⚠️ basic | — | — |
| Animation & subscriptions | ✅ | ✅ | ✅ | — | — |
| Async effects / `fetch` | ✅ | ✅ | ✅ | — | — |
| Clipboard | ✅ | ✅ | ❌ | — | — |
| Accessibility | ✅ AccessKit | ⚠️ partial | ❌ | — | — |
| Lifecycle | ✅ | ✅ | ⚠️ partial | — | — |
| Live-reload (dev) | ✅ | ⚠️ | ❌ | — | — |

---

## Near term

The things standing between frus and someone else being able to use it.

### Distribution

- 🟡 **Publish to crates.io.** Everything resolves through local `path` dependencies today. This needs real versions, per-crate `README`s, metadata (`repository`, `keywords`, `categories`, `documentation`), a publish order that respects the dependency graph, and a release script. Once done, the `cargo generate` template drops `{{frus_path}}` and becomes `frus = "0.1"`.
- 🟢 **Per-crate `README.md`.** Each crate needs a short one for its crates.io page.
- 🟢 **Pin an MSRV.** There is no `rust-version` in any manifest and no minimum supported Rust version has ever been tested. Find the oldest stable that builds the workspace, put `rust-version` in `[workspace.package]`, and add that toolchain to the CI matrix.
- 🟡 **docs.rs-quality rustdoc.** Crate-level docs with a runnable example on every public crate, and `#![warn(missing_docs)]` turned on crate by crate.

### Web parity

The web target renders and animates but is missing its platform integrations. Each of these is an independent, self-contained job.

- 🟡 **Clipboard** via the async Clipboard API, behind the same interface desktop uses.
- 🟡 **Accessibility.** AccessKit has web support; the semantics tree already exists and is populated. This is a bridging job, not a from-scratch one.
- 🟡 **IME / soft keyboard** on mobile browsers — a hidden input overlay, composition events, viewport insets.
- 🟡 **Live-reload** for the wasm target.
- 🟢 **A proper web example page** — the current `index.html` is minimal.

### Size

Milestone 292 took a release APK from 286 MB to 4.9 MB by building `--release` at all. What is left is real work, not settings.

- 🟡 **Subset the bundled faces at build time.** DejaVu covers far more of Unicode than any one application draws, and the four `bundled-*` features are a coarse instrument next to a `pyftsubset` step over the glyphs a build actually references. This is where the next megabyte is, and it needs a tool in the build.
- 🟢 **A size regression check in CI.** Nothing notices today if the floor doubles. Build `frus-hello` for `aarch64-linux-android` in release, compare the stripped `.so` against a committed budget, fail on a jump.
- 🟢 **Document `--split-per-abi`-style packaging.** The examples build `aarch64` only; an application targeting more than one ABI wants a split rather than a fat APK, and nothing says so.

### Quality

- 🟢 **Clear the clippy backlog — done, milestone 298.** 71 warnings, now zero, and CI runs `cargo clippy --workspace --all-targets -- -D warnings` as a **blocking** check. Six `too_many_arguments` carry a targeted `#[allow]` with the reason at the site; everything else was rewritten.
- 🟢 **Format the tree — done, milestone 298.** One `cargo fmt --all` commit over 40 files, on its own, and the fmt check is **blocking** now.
- 🟢 **Widen golden coverage — done, milestones 296 and 297.** 58 of the 86 widget modules had no pixel test at all; **all 86 have one now**. 296 added 27 goldens for the widgets whose picture follows from their arguments; 297 added `frus_test::Stage`, a harness that holds the retained state and steps the frame loop the way the shell does, and 12 more for the ones whose picture is a gesture in flight — a swipe half done, a pull past the top, a glow, a page between two pages.
- 🟢 **An overlay belongs to its screen — done, milestone 326.** The red item found on a device: an app bar's overflow menu, left open while choosing "Settings →", stayed drawn over the screen that replaced it. Neither obvious explanation was right — no retained state leaked, and the outgoing screen had not left the tree (during a transition it is genuinely still in it, and once the spring settles the menu goes with it). Reproducing it through `Application::view` and printing the item's position every frame showed the actual cause: it never moved. `process_overlays` runs after both screens and paints above the **whole window**, so a deferred overlay outranks the screen drawn on top of its owner; and the transition is parallaxed, so the outgoing screen travels only 30 % of the width and the anchor never leaves. A screen on its way out now takes its floating layers with it (`children[1]` is always the destination — on a push, on a pop and under a back gesture). Reading that code turned up a second bug: the auto-flip that keeps a menu inside the window would drag an overlay whose anchor had genuinely left the window back into view, barrier and all, which a portal in a horizontally scrolled row hits with no navigator involved. The report's other half — the menu reappearing on returning home — was the application's: `Msg::Push` dismissed the drawer and the popup menu and missed the overflow.
- 🟢 **A tap on a dismissible row did nothing — done, milestone 327.** Found on the device as "the task row's avatar does not open the task". Of the two candidates, the hit box was cleared first: a probe sweeping the window found the target at exactly the 30 px the avatar is drawn at, topmost, resolving to the right message. The gesture arena was the whole of it. A press on a dismissible row prepares a swipe that engages only past a threshold, and the release told a tap from a drag by matching against a list of the four gestures that behave that way — `Scroll`, `Pan`, `Reorder`, `Item` — with `Dismiss` missing. So the press recorded the widget, `pointer_up` returned before the click path, and nothing was ever told. It shows only when the list is too short to scroll, because a scrolling list claims the press as a `Scroll` first, which is why the bug arrived looking intermittent; reading the routing, a pointer reaches the swipe branch unconditionally, so on desktop every click on such a row was swallowed. The list is now `gesture_was_a_tap`, named and tested in both directions. Confirmed on the device: the avatar opens the task, the checkbox (dead for the same reason, never reported) toggles, a short swipe springs back without navigating and a long one still dismisses. Milestone 321's centring is verified in passing.
- 🟢 **A long label pushed the row's delete button out of the hit registry — done, milestone 334.** Milestone 333 measured the defect (the label refuses to shrink below its content, so the whole 398 px deficit lands on the button: 40 px laid out at 13, then off the card entirely) and named `flex_shrink` as the missing field. Reading the reference first changed the answer: `RenderFlex` hands its inflexible children a constraint with **no maximum on the main axis**, so it never squeezes anyone — the fix is not to let the button refuse, it is that the label should never have been asked how wide it wants to be. `Expanded` is that, and it needs three properties together: `flex_basis: 0` so the child stops sizing the row, `flex_grow` so it takes what is left, and `min_width: 0` to lift the automatic minimum that makes `flex: 1` alone a no-op on a long label. `Style` gained `flex_shrink` as well, with `shrink()`/`no_shrink()` on `Flex` and `Container`, for the other case — a row over budget with every child at its natural size. Two findings came out of doing it: the demo's row never filled its card (a row is content-sized on its own main axis, so an expanding label had nothing to expand into), and clearing the cross axis in the wrapper laid an expanded `Text` out 0 px tall. Both caught by reading the tree, not the picture. Device-confirmed: a task too long for the phone shows an ellipsis, its × sits on the right edge with the others, and tapping it deletes the task.
- ✅ **A row shrink-wraps where the reference's fills — done, milestone 342.** Changing `Flex`'s default would have reached every row in the framework and all 88 goldens; the answer was a `Row` and a `Column` of their own, with the reference's defaults (`MainAxisSize::Max`, centred children, a `start` that follows the reading direction) and the two orderings a flex has there. Filling turned out not to be a property of the row at all but a question about its parent, and one that has to travel: stopping at the parent leaves `Container > Column > Row` as wide as the tile inside it. `Flex` keeps flexbox's defaults and the framework's own widgets keep using it.
- 🟢 **A row that overflows does so silently — done, milestone 335.** The reference calls an overflowing flex an error condition, writes to the console and paints a striped band across the offending edge; here a child simply drew outside its parent and nothing said so, which is why one bug took three milestones and a device report. `Layout::overflows` and `Ui::overflows()` now name every box whose children ran past it, with the edge and the amount, and the shell reports each site once. It needed no exception list: a scrollable, a stack, a page view and a fitter are laid out as leaves with their content computed separately, so the one overflow that is deliberate never reaches the walk. Sixteen of the demo's eighteen screen/width combinations were clean on the first run; the survey is kept as `no_screen_draws_outside_itself`. **What it found:** the chart dashboard's segmented control, 584 px of segments in a 363 px row on a phone, the last one drawn 221 px outside the card — the control now stops at its parent's edge and the segments divide what there is, with the labels ellipsised. A device then showed the half no test had: the hairlines between segments were spaced by the *natural* segment width and no longer fell where the segments met. **The striped band was painted in milestone 345** and found three things in its first minute: four goldens wearing one for a pixel that existed only in taffy's rounding (the overflow is measured on the unrounded layout now, which was also making the console reports untrustworthy), a group-opacity fixture that had been overflowing since it was written, and — separately, chasing the settings screen below — a `Tabs` that sized itself by whatever was on the busiest tab. **The band learnt to write in milestone 348**: `RIGHT OVERFLOWED BY 86 PIXELS`, the reference's words and its 7.5 px dark red on white, turned a quarter turn on the vertical edges — the half of the report that survives being photographed off a device, where the console is not. It cost the scene a `Scene::transformed`, and taught one thing about the compositor: a layer is rendered flat into a window-sized texture, so a group must be painted where it *fits* and carried to where it goes by its transform, not painted where it goes.
- 🟢 **A group-opacity layer from a covered page showed through the page above — done, milestone 350.** Found on the device at the end of milestone 349: the home screen's translucent square painted over the Kanban board, and thin slivers of a swipe background at the left edge of every screen. Every layer was composited after *all* of the content, so anything covered came back on top. The batch planner already answers this question for content — it gives a primitive a level from what it covers — and simply skipped layers; a layer is a batch of its own now, with a footprint of what its contents cover through its clip and its transform, and the render pass interleaves composite draws with content draws. Nested layers inside a group's pre-pass had the same bug and the same fix. **No golden caught it**, because every fixture that draws a group draws it last; `layer_order.rs` covers the four cases at scene level. **Confirmed on the device**: the square and the slivers are gone.
- 🟢 **An Android application opened with a white flash — done, milestone 352.** Reported as *the app does not start the way the reference's does*, and it did not: Android paints the window from the moment it opens it, and the platform theme's background is white on a device in light mode. `am start -W` said 275 ms to the first frame, so nothing was slow — the wrong thing was on screen while it started. A `LaunchTheme` whose `windowBackground` is the application's own colour, shipped in the demo, the three examples and the template, as the reference ships one in its own. Left: it is a **colour**, where the reference lets you put your icon on it; nothing checks that the colour still matches the theme; and desktop and web have never been asked the same question.
- 🟢 **A virtualised list's rows hugged their text — done, milestone 351.** From the device, milestone 349: a `List` row with a background painted a chip around its label rather than a row across the list — 79 px in a 363 px list. The reference *tells* a list's children their cross-axis extent rather than asking; `Constraints::filled` already existed for a `PageView`'s page and the fix is that word. **No golden caught it**, for the same reason as the layer order: the fixture builds its items out of bare text, and text hugged by its box draws the same pixels as text in a box across the list. **Confirmed on the device**: the rows run the width of the list. The three things it left were answered in milestone 353 below.
- 🟢 **A grid's rows were as tall as whatever was in them — done, milestone 353.** The reference's grid delegate derives a main-axis extent for every tile from the track it just sized — `childAspectRatio` defaulting to 1.0, so tiles are square — and hands each child a box tight on both axes; ours let each row follow its content, so a board of tiles came out as ragged bands. `Grid::aspect(ratio)` and `Grid::tile_height(px)` now say which, and `row_gap`/`column_gap` split what was one `gap`, as the reference's two spacings are separate. The ratio is the *container's* say and cannot be asked of a tile — a tile does not know how wide its column came out — so it is imposed during the walk, the third thing resolved there next to the fill request and the shrink grant. The reference's default of square tiles is **not** taken: `Grid` is also the framework's plain CSS grid and four widgets are built on it. Checked in passing and already right: a **scroll** fills its content's constrained axis when exactly one is free, and a **table**'s cells are tight across and free down. Left: the reference's *other* delegate, which derives the column count from a maximum tile width — it needs the grid's computed width before the layout, which is `LayoutBuilder`'s job, and `LayoutBuilder`'s height does not follow its content.
- 🟡 **Settings overflows by 4.5 px at a phone's width.** *(Third measurement, milestone 349: the tab set is 380 px wide **whatever the viewport** — the same 380 at 411 px and at 260 — so it is a hard minimum inside the Controls tab, not a proportion of anything. The panel column sits at 340; the two differ by the tab set's inset and the panel's padding.)* Found by the same survey and pinned by it so it cannot grow. The cause recorded here for ten milestones — `Tabs` shrink-wrapping where the reference's fills — was **disproved in milestone 345**: the tab set fills its box now and the overflow did not move by a pixel. What is left is that something in the Controls tab will not go below about 380 px in a 371 px column, so the row centring the tab set lets it hang out either side. The 4.5 is the unrounded measurement (it read 5 while rounding was in it); the pin came down from 5.5 to 4.6.
- 🟡 **`Flexible` by name.** The loose fit — *at most* your share — exists as `Expanded::loose()`, but the reference has two widgets, `Flexible(fit:)` and `Expanded`, and an application ported from it will type the one that is not here. `shrink()` is still on `Flex` and `Container` only while ten other widgets carry `flex()`; **milestone 349 made that matter much less** by turning the default around — nothing is squeezed unless it says so, which is what those ten widgets wanted from `shrink(0.0)` in the first place.
- ✅ **`Text` could not be aligned, limited or faded — done, milestone 343.** `TextAlign`, `max_lines` and the reference's four `TextOverflow` modes, on the widget every screen is made of. Two things had to be true before any of them could work: an aligned text has to **fill** the box it is offered, because a box exactly as wide as its own words has nowhere to align them to; and a text that has said what to do on overflow has to be **clamped to its parent**, which is the reference's `constraints.constrain` in this layer's vocabulary — without it a narrower box never takes the declared width away and the mode never fires. `Fade` is the first use of milestone 339's mask machinery from a widget's paint. **`RichText` caught up in milestone 346** — the same four answers, and nothing in the layout had to change for it, which is the argument for having put `main_axis_floor`, the fill request and the clamp-to-parent where they went rather than inside `Text`. **`softWrap` followed in milestone 344**, and the flag was never the reason it did not: a `Text` declared its own width, and a box that answers before it is asked cannot be told to wrap. It is measured from the space offered now, which needed the other half of the reference's rule — a row never squeezes its inflexible children, a column hands them a width — as `Widget::main_axis_floor`. Eighteen goldens moved by one or two pixels of vertical shift and all eighteen were read.

- ✅ **`truncate` leaves a zero-width box alone — done, milestone 347.** The exception was inherited from the app bar, whose title room has a floor of 64 px and cannot produce a zero in the first place; what it actually did was let a genuinely collapsed box draw its whole label over whatever was beside it, which is the one thing ellipsising exists to prevent. A box with no room gets an ellipsis and nothing else now, and the test that pinned the old behaviour says so.

---

## Medium term

### New platforms

- 🔵 **iOS shell.** The most valuable single contribution available. `frus-shell` is the only crate that would change: a `UIViewController` host, Metal surface via `wgpu`, touch input, IME, lifecycle, safe-area insets. The layering is designed for exactly this, and Android is the worked example to follow.

  **Groundwork landed** (see `docs/milestone-276.md`): `frus-shell` now has named platform `cfg` aliases (`desktop` / `android` / `ios` / `web`) via its `build.rs`, so iOS no longer falls silently into the desktop branch, and a `run()` entry point exists behind `#[cfg(ios)]`. An advisory CI job builds for `aarch64-apple-ios-sim` and `aarch64-apple-ios`. What remains is the actual platform integration — lifecycle, safe-area insets, IME/soft keyboard, `os_log`, UIKit accessibility, and `.ipa` packaging.
- 🔴 **Native macOS shell.** winit already covers macOS as a desktop target; a native shell would be about menu bar, window chrome, and platform conventions rather than rendering.

### Framework depth

- 🟡 **Text rendering edge cases.** Bidi runs, complex scripts, font fallback chains, emoji, vertical metrics. Real bug reports welcome here.
- 🟢 **`Scaffold` and `body` — reviewed, milestone 288.** `extend_body`, `extend_body_behind_app_bar`, `resize_to_avoid_bottom_inset`, `window_insets`, a leading `drawer` beside the trailing one, and `persistent_footer`. The FAB gained a **location** in milestone 290 — three ends × float or docked.
- 🟢 **A bottom app bar with a notch — done, milestone 291.** `BottomAppBar` in the scaffold's bottom slot, cut around a docked FAB by the scaffold, which is the only party that knows where both are. Left: the bar has no elevation or surface tint, for the same reason the app bar has none.
- 🟢 **The renderer composites in scene order — done, milestones 294 and 295.** It used to draw one pass per kind — rectangles, then images, then paths, then text — so *every* path covered *every* rectangle in the frame, wherever the two sat in the scene. Found on a device in milestone 291 (a filled button on a notched bar); it applied to `CustomPaint`, the charts, `ClipPath` and the overscroll glow just as much. Primitives are now given a **level** from what they cover, and a level costs one draw call per kind it holds — a twelve-row list is 3 draw calls, where ordering primitive-against-primitive would have cost 25. Text was left out of the plan in 294 and folded in by 295, once `Primitive::Text` learned to carry the box it was laid out in. `frus_gpu::draw_calls(scene)` reports the cost. Left: a primitive's footprint is its widget's box, so text over a wide, mostly-empty label costs a level it need not; tightening it to the shaped extents wants the measurement to come back from the text painter.
- 🔴 **A widget cannot measure its children, nor describe anything to its subtree.** Children are built by the application *before* the widget that holds them sees them, so a container can neither ask how big they are nor put an ambient value where they would read it. The reference builds lazily and does both. Three known symptoms, all deferred for this one reason: the FAB's height must be **declared** to dock it (`fab_size`, milestone 290) and its `*Top` placements are missing for want of the app bar's height; `AppBar` is a builder rather than a `Widget` because it needs the available width before it can decide anything; and a scaffold cannot tell an extended body what it is running under, so that body pads its own content by hand. Wants a design — lazy children, or a resolution pass between build and layout — not a parameter.
- 🟢 **`AppBar` is a builder, not a `Widget`** — it must be finished with `.build()` because it needs the available width before it can decide anything. Every other widget in the framework *is* a `Widget`. Revisiting it is an API change with call sites, not a fix.
- 🟢 **The app bar's title carries no `Role::Heading`.**
- 🟢 **`NavBar` shrinks around its back button** when its parent gives it no width (found by milestone 296's first golden of it). Its `paint` centres the title in `bounds`, which only makes sense at full width, but its `style` asks for `Dimension::Auto`, so with nothing to fill it hugs the button and paints the title underneath. Every screen happens to give it a width, which is why it has never shown.
- 🟢 **A wrapping text that is shrunk to fit reported one line's height — fixed, milestone 289.** The diagnosis recorded here (measure-pass ordering) was wrong: the cause was half a pixel. Text measurements now round up, so the box the layout rounds to is one the text still fits in. Left, and general: the layout engine rounds every box to whole pixels while the reference does not, so anything that measures itself must round up too.
- 🟢 **Scrolling physics — done, with two loose ends.** `ScrollPhysics::{Bouncing, Clamping}` with the platform's own fling curves, a real rubber band and per-app / per-area overrides (`docs/milestone-277.md`); a fitted velocity estimate (`docs/milestone-278.md`); the overscroll glow, device-verified (`docs/milestone-279.md`) and rebuilt as an actual **fade** in milestones 301 and 302 after a second device report — it was a flat fill, and a hard curved edge across the page reads as the page being bent rather than as light. Paths carry a gradient now, straight or radial. Left: the **stretch** overscroll effect newer platform versions use instead of a glow (needs a render-target effect), and the second bouncing deceleration profile.
- 🟢 **Cache text measurement — done, milestone 300.** Milestone 299 found that **three quarters of the cost of building a frame was `frus_text::measure`**, re-shaping every string through cosmic-text on every call for strings that had not changed. A cache keyed on `(text, size, weight, italic, max_width)` — with the weight and style **resolved**, so a Medium asked for on a family that only ships Regular hits the same entry as the Regular — took `measure/line` from 16.3 µs to **79 ns**, and took building a twelve-row frame from **4.1×** the cost of the same tree with no strings in it to **1.20×** — text is a sixth of a frame now, where it was three quarters. Eviction is two generations with a promotion on hit, so a string still on screen never falls out and one that has gone leaves with its generation; registering a font empties the cache, an answer from before that being wrong rather than stale. Left: the batch planner is still O(n²), and nothing measures the GPU side of a frame.
- 🔴 **Rebuild memoization.** `view` rebuilds the whole tree each frame. Milestone 299 sized it: the same tree with every string replaced by a fixed box costs a quarter as much, so rebuilding is real but it is the *small* half — measure text first. Wants a design, not a parameter.
- 🟡 **The batch planner is O(n²).** Each primitive scans the levels for something it overlaps: 80 primitives plan in 4.7 µs, 1302 in 597 µs (16× the primitives, 127× the time). About 4% of a 60 Hz frame at 1302, so not urgent — but it grows the wrong way, and milestone 294 should not have implied the plan was free. Wants a spatial index, or levels that carry a bounding box per kind.
- 🟡 **Renderer batching.** Fewer draw calls for scenes with many small primitives; atlas the common shapes.
- 🟠 **The catalogue is 22 widgets short of the reference — counted, milestone 336.** Every class in the reference's `widgets/` and `material/` libraries that extends a widget base class (391 of them) against this framework's exports, with a synonym map for the ones we spell differently and a skip list for what cannot apply (platform views, web embedding, windowing, restoration, the inspector's scaffolding, and slivers). 28 came back unmatched, six of them the script's own fault. Milestone 336 closed `ListTile`, `Flexible` and `Placeholder`; milestone 337 closed five of the six **focus** widgets; milestone 338 closed **shortcuts and actions** and the sixth focus widget with them (`ShortcutRegistrar` deliberately not: the reference needs a runtime registry because its tree is retained, and a view rebuilt each frame does not); milestone 339 closed the three **filters that act on their own subtree** (`ColorFiltered`, `ImageFiltered`, `ShaderMask`), with the blend modes and the separable blur pre-pass they needed; milestone 340 closed **`BackdropFilter` and `BackdropGroup`**, and with them a three-hundred-milestone-old bug where a layer nested in another layer was never drawn at all; milestone 341 closed `Baseline` and `IgnoreBaseline`, and with them the alignment that gives those two a reason to exist — `Align::Baseline` on a row. **The count is closed: all twenty-two** — and milestone 342 added the two the count could not see, `Row` and `Column`, which existed here only as flexbox defaults under another name. Presence in the catalogue is not the same as depth: the rest of this roadmap is depth, and it is the larger half.
  the ambient surface description (`MediaQuery`, `SafeArea`) and the widgets that withhold
  part of the frame (`IgnorePointer`, `AbsorbPointer`, `Visibility`, `Offstage`,
  `ExcludeSemantics`). Still unclaimed, roughly in order of how often they are reached for:

  - 🟢 **Pull-to-refresh — done** (milestone 281): `Refresh`, device-verified under both
    physics. Left: the same machinery on the **bottom** edge for load-more, and a way for
    an application to start the indicator itself.
  - 🟢 **Swipe-to-dismiss — done** (milestone 282): `Dismissible`, with the shared-gesture
    arbitration that lets it live inside a list. Left: a confirmation step (an undo window
    needs the message to be able to refuse) and cross-axis drift. The on-device check is
    **done** (2026-08-12).
  - 🟢 **A paged view — done** (milestone 283): `PageView`, virtualised, snapping on the
    milestone-277 physics, with `page`/`on_page_changed` binding both directions to one
    number, device-verified in both directions. Left: per-page transformations (parallax,
    depth), padded ends below a `viewport_fraction` of 1, and keyboard paging.
  - 🟢 **Shared-element transitions — done** (milestone 286): `Hero`, device-verified.
    The "two trees" half turned out to be free — the navigator already holds both screens
    in one frame. Left: a curved flight path, a cross-fade between the two contents, and
    shared elements that move *within* a screen rather than between routes.
  - 🟢 **The constraint boxes — mostly done** (milestone 284): `SizedBox`,
    `ConstrainedBox`, `Intrinsic`, `OverflowBox`, and `max_width`/`max_height` on `Style`.
    Two are left, and both need something the layout does not yet surface:
    **`LimitedBox`** needs to know whether the incoming constraint was *unbounded*, which
    taffy owns and never tells a widget; **baseline alignment** needs the text measurement
    to report a baseline, which the measure hook cannot return.
  - 🟢 **A general drag-and-drop pair — done** (milestone 285): `Draggable` /
    `DragTarget`, with a `u64` payload, a ghost lifted out of the frame, and a lift that
    yields to any scrollable under it (`long_press()` inside a list). Device-verified.
    Left: auto-scroll at a viewport's edges while carrying, a typed payload, a fly-back
    on a refused drop, and migrating `Kanban`/`Table` onto it.
  - 🟡 Rich text editing, video, maps, virtualized lists for very large datasets.
- 🔴 **Router / deep linking.** `navigator.rs` handles in-app navigation; URLs, deep links, and browser history are not modelled.
- 🟡 **State persistence.** A blessed way to save and restore app state across lifecycle transitions.

### Developer experience

- 🔴 **DevTools.** `inspector.rs` can already dump the widget tree. A live inspector — tree view, layout overlay, rebuild counts, frame timing — is a large but very high-leverage project.
- 🟡 **Better error messages.** Layout and constraint failures should say what went wrong and what to change, not just produce a wrong-looking screen.
- 🔴 **Hot reload** beyond the current live-reload, with state preservation.

---

## Long term

The original brief aims at a full framework, not just a UI toolkit. These are real goals, not yet real work:

- **Plugin system** — a stable ABI for third-party platform integrations (camera, sensors, storage), with the thin-native-adapter rule enforced.
- **FFI story** — embedding frus in an existing native app, and calling native code from frus.
- **Package/ecosystem conventions** — how a community widget library is published and discovered.
- **Static analysis** — lints specific to frus's architecture (impure `update`, hardcoded styling, misplaced platform code).
- **Documentation site** — the milestone notes are excellent raw material, but they are unindexed and unsearchable outside `grep`.

---

## Non-goals

Stated so nobody spends a weekend on them:

- **A DSL or markup language.** Rust with builder methods and the `column!`/`row!` macros is the API. No `.frus` files, no macro that reinvents syntax.
- **A bespoke CLI.** `cargo` is the tool. `cargo generate`, `cargo apk`, `wasm-bindgen` are existing tools; we don't wrap them in `frus doctor`.
- **A software rasterizer.** `wgpu` covers every target we care about.
- **Framework logic in another language.** Non-Rust code is limited to thin platform adapters with no logic of their own.
- **Cloning another framework's API surface verbatim.** We port what works and fix what doesn't. "Framework X does it this way" is a starting point in a discussion, not an argument that ends one.

---

## How to pick something up

1. Find an item here or in [issues](https://github.com/KalybosPro/frus/issues).
2. Comment saying you're taking it. If there's no issue, open one first — especially for 🔴 items.
3. Read the relevant part of [ARCHITECTURE.md](ARCHITECTURE.md) and `grep docs/` for prior decisions in that area.
4. Build it, test it, and follow [CONTRIBUTING.md](CONTRIBUTING.md).

Small first PRs are welcome and are the fastest way to learn the codebase's conventions. So is an issue that just says "this API is worse than the one I'm used to, here's why" — that is useful work.
