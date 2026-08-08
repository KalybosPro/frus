# Design notes

This directory is the project's memory: **276 milestone notes** (*jalons*), one per step of
frus's construction. Each records the objective, the alternatives that were weighed, the
decision and its reasoning, the implementation, how it was verified, and what was
deliberately left for later.

When you find yourself asking *"why on earth is it done this way?"*, the answer is almost
always here — along with the option that was rejected and why. `grep` this directory before
opening an issue.

> **The note bodies are still being translated from French.** The titles below are English;
> the notes themselves are being converted a batch at a time. Helping is a genuinely valuable
> contribution — see [ROADMAP.md](../ROADMAP.md).

## Other documents

| Document | What it is |
| --- | --- |
| [getting-started.md](getting-started.md) | Write and run your first frus application |
| [brief.md](brief.md) | The original brief: vision, philosophy, working method |
| [prior-art.md](prior-art.md) | Ideas from mature UI toolkits, evaluated for porting — what to take, what to fix |
| [status.html](status.html) | A visual snapshot of the framework's state |

## Where to start

If you're new to the codebase, these are the notes worth reading first:

| Milestone | Why it matters |
| --- | --- |
| [0](jalon-0.md) – [4](jalon-4.md) | The foundations: a window, the 2D renderer, layout, the widget tree, interactivity |
| [129](jalon-129.md), [131](jalon-131.md) | The web target, and shrinking the wasm payload |
| [267](jalon-267.md), [268](jalon-268.md) | The single entry point (`main!`) and the `frus` facade crate |
| [270](jalon-270.md) – [275](jalon-275.md) | Async effects, `fetch`, `RemoteData`, typed JSON |

## All milestones

| # | Title |
| --- | --- |
| [0](jalon-0.md) | A window + a coloured quad |
| [1](jalon-1.md) | Minimal 2D renderer (primitives) |
| [2](jalon-2.md) | Layout engine (flexbox via taffy) |
| [3](jalon-3.md) | Declarative widget tree |
| [4](jalon-4.md) | Interactivity: events + state |
| [5](jalon-5.md) | Text |
| [6](jalon-6.md) | Widget identity + interaction states |
| [7](jalon-7.md) | Style: rounded corners, borders, alignment, per-side padding |
| [8](jalon-8.md) | Text input + keyboard focus |
| [9](jalon-9.md) | Vertical scrolling + clipping |
| [10](jalon-10.md) | Caret, navigation, selection and clipboard |
| [11](jalon-11.md) | Animations (implicit transitions) |
| [12](jalon-12.md) | Shadows, gradients, horizontal scrolling, scrollbar & drag, focus animations |
| [13](jalon-13.md) | Opacity + fade-in |
| [14](jalon-14.md) | Fade-out (retaining outgoing widgets) |
| [15](jalon-15.md) | Theme system |
| [16](jalon-16.md) | Library of named (themed) widgets |
| [17](jalon-17.md) | Overlay / portal (floating menus, tooltips, modals) |
| [18](jalon-18.md) | Navigation (screen stack + slide transitions) |
| [19](jalon-19.md) | State transitions · Back gesture · Advanced overlay |
| [20](jalon-20.md) | A real example app: todo list |
| [21](jalon-21.md) | Framework / application split (`run(app)`) |
| [22](jalon-22.md) | Navigation bar (`NavBar`) + animated titles |
| [23](jalon-23.md) | Scrolling with inertia (spring + bounce) |
| [24](jalon-24.md) | `Command` / effects from `update` |
| [25](jalon-25.md) | DPI / scale factor (HiDPI) |
| [26](jalon-26.md) | Subscriptions (continuous message sources) |
| [27](jalon-27.md) | DX / ergonomics (writing a UI faster) |
| [28](jalon-28.md) | Keyed reconciliation (stable identity) |
| [29](jalon-29.md) | Keyboard navigation / accessibility |
| [30](jalon-30.md) | Window robustness |
| [31](jalon-31.md) | Virtualised list (`List`) |
| [32](jalon-32.md) | New widgets (6) |
| [33](jalon-33.md) | New widgets: Collapsible, Menu, Chip |
| [34](jalon-34.md) | New widgets: Avatar, Stepper, Rating |
| [35](jalon-35.md) | Layout: grid (`Grid`) |
| [36](jalon-36.md) | New widgets: Table, SegmentedControl, Toast |
| [37](jalon-37.md) | New widgets: Breadcrumb, Pagination, Skeleton |
| [38](jalon-38.md) | New widgets: Tree, ColorPicker, Timeline |
| [39](jalon-39.md) | Click fix + new widgets: DatePicker, Carousel, Alert |
| [40](jalon-40.md) | New widgets: Popover, Autocomplete, Kbd |
| [41](jalon-41.md) | sRGB / linear colour handling |
| [42](jalon-42.md) | Responsive by default |
| [43](jalon-43.md) | Adaptive layout (navigation & master-detail) |
| [44](jalon-44.md) | Dynamic scale & size |
| [45](jalon-45.md) | Advanced widget responsiveness |
| [46](jalon-46.md) | Drawer animation (slide + fade) |
| [47](jalon-47.md) | Right drawer & permanent drawer |
| [48](jalon-48.md) | Drawer slide on a spring curve |
| [49](jalon-49.md) | Modal sheet (`BottomSheet`) |
| [50](jalon-50.md) | First run on physical Android |
| [51](jalon-51.md) | System insets (safe area / SafeArea) |
| [52a](jalon-52a.md) | Adaptive AppBar (Material app bar) |
| [52b](jalon-52b.md) | Unified `Scaffold` (Material screen skeleton) |
| [53](jalon-53.md) | Unified physics (`trait Simulation`) |
| [54](jalon-54.md) | Reachable animation layer + the demo's transitions on top of it |
| [55](jalon-55.md) | Relayout boundary cache (retained layout on top of taffy) |
| [56](jalon-56.md) | Frame phases: conditional build (build → paint) |
| [57](jalon-57.md) | `BoxDecoration`: the box decoration model (§5) |
| [58](jalon-58.md) | Theme: baked-in Material state layers + extended M3 roles |
| [59](jalon-59.md) | Generalising the Material state layers |
| [60](jalon-60.md) | Typography: `TextStyle` + `TextTheme` (weight and italic rendered) |
| [61](jalon-61.md) | Fully customisable AppBar/NavBar |
| [62](jalon-62.md) | `TextSpan`: rich text, from the styled tree to the GPU |
| [63](jalon-63.md) | `TextLayout`: caret, hit-testing and selection on cosmic-text |
| [64](jalon-64.md) | Measuring under constraints (taffy closures) + wrapping paragraph |
| [65](jalon-65.md) | `RichText::wrap()`: the wrapped rich paragraph |
| [66](jalon-66.md) | `BorderRadius`: **per-corner** radii (SDF) |
| [67](jalon-67.md) | Adopting per-corner (sheet, segments) + the border reserves its space |
| [68](jalon-68.md) | `ColorScheme`: consolidated roles (single source of truth) |
| [69](jalon-69.md) | Gestures, stages 0+1: normalised input + long press |
| [70](jalon-70.md) | Focus: keyboard-only ring + arrow navigation (geometric) |
| [71](jalon-71.md) | Leaf→root key handling (3 states): Escape closes everywhere |
| [72](jalon-72.md) | Focus scopes: the modal traps Tab, arrows and clicks |
| [73](jalon-73.md) | Touch fling: scroll momentum (ballistics) |
| [74](jalon-74.md) | Window insets: `padding` / `viewInsets` split (keyboard avoidance) |
| [75](jalon-75.md) | Text decorations (underline, strikethrough, highlight) |
| [76](jalon-76.md) | `from_seed`: theme generated from a seed colour (HCT) |
| [77](jalon-77.md) | `frus-test`: headless rendering, snapshots and goldens (opening §13) |
| [78](jalon-78.md) | Runtime inspector (§13, stage 1) |
| [79](jalon-79.md) | State-preserving live reload (§13) |
| [80](jalon-80.md) | Android soft keyboard (opening the §6 input work) |
| [81](jalon-81.md) | Android InputConnection bridge (§6, stage 2) |
| [82](jalon-82.md) | IME input, stage 3: styled composition + suggestion context |
| [83](jalon-83.md) | One-command start (`cargo generate`): closing §13 |
| [84](jalon-84.md) | RTL: reading direction and layout mirroring (§14, opening) |
| [85](jalon-85.md) | Accessibility: semantic annotation + AccessKit bridge |
| [86](jalon-86.md) | Localisation (i18n/l10n): Fluent |
| [87](jalon-87.md) | Arabic script (bidi): script rendering + off-screen RTL fix |
| [88](jalon-88.md) | Frame phases & repaint boundary cache |
| [89](jalon-89.md) | Vector paths & icons |
| [90](jalon-90.md) | Images & textures |
| [91](jalon-91.md) | Image decoding (PNG/JPEG) |
| [92](jalon-92.md) | Layer compositing & pipeline precompilation |
| [93](jalon-93.md) | Anti-aliasing (MSAA) |
| [94](jalon-94.md) | GPU reuse of layer textures |
| [95](jalon-95.md) | Implicit animations: per-widget curve & duration |
| [96](jalon-96.md) | Group opacity & `AnimatedOpacity` |
| [97](jalon-97.md) | `AnimatedContainer`: animated background colour |
| [98](jalon-98.md) | `AnimatedContainer`: animated size (at layout) |
| [99](jalon-99.md) | `AnimatedContainer`: animated corner radius |
| [100](jalon-100.md) | Named widgets: `Opacity`, `AnimatedOpacity`, `AnimatedContainer` |
| [101](jalon-101.md) | Explicit animations: `repeat` / `stop` / `reset` |
| [102](jalon-102.md) | `AnimatedContainer`: animated padding |
| [103](jalon-103.md) | `Animatable`: the explicit → live typed value bridge |
| [104](jalon-104.md) | Composed `Animatable`s: `TweenSequence` + box tweens |
| [105](jalon-105.md) | Container: `alignment` + composite `decoration` |
| [106](jalon-106.md) | Fractional `Alignment` + `Tween<Alignment>` (manual placement) |
| [107](jalon-107.md) | Anchoring: virtualised lists + `AlignmentDirectional` |
| [108](jalon-108.md) | `AlignmentGeometry`: unified anchoring |
| [109](jalon-109.md) | Container: outer margin (`margin`) |
| [110](jalon-110.md) | `AspectRatio`: a box with a width/height ratio |
| [111](jalon-111.md) | `FractionallySizedBox`: size as a fraction of the parent |
| [112](jalon-112.md) | `Transform`: paint offset (`translate`) |
| [113](jalon-113.md) | `Transform`: paint scale (`scale`) |
| [114](jalon-114.md) | `Transform`: rotation (rotated composited layer) |
| [115](jalon-115.md) | `Transform`: non-uniform scale (`scale_xy`) |
| [116](jalon-116.md) | `Transform`: composition (translate + scale + rotation) |
| [117](jalon-117.md) | `Transform`: unified affine matrix |
| [118](jalon-118.md) | `Transform`: focus/a11y follow the scale (axis-aligned case) |
| [119](jalon-119.md) | Animated showcase: `frus-transforms` |
| [120](jalon-120.md) | Pixel tests for the transform pipeline |
| [121](jalon-121.md) | Shape clipping: `ClipRRect` / `ClipOval` |
| [122](jalon-122.md) | `InteractiveViewer`: pan + zoom (pinch/wheel) |
| [123](jalon-123.md) | Extended showcase: Clip + InteractiveViewer |
| [124](jalon-124.md) | `FittedBox` + `RotatedBox`: transforms that affect layout |
| [125](jalon-125.md) | **Per-corner** rounded clipping (`ClipRRect` + `BorderRadius`) |
| [126](jalon-126.md) | `InteractiveViewer`: inertia (fling) + pan bounds |
| [127](jalon-127.md) | `ClipPath`: clipping to an arbitrary path (mask pipeline) |
| [128](jalon-128.md) | Showcase: ClipPath + RotatedBox + FittedBox |
| [129](jalon-129.md) | Web target (wasm + WebGPU) |
| [130](jalon-130.md) | Effects & subscriptions on the Web |
| [131](jalon-131.md) | Slimming down the `.wasm` |
| [132](jalon-132.md) | Decorated form field (label, hint, help, error) |
| [133](jalon-133.md) | Password field (masking) + prefix/suffix icons |
| [134](jalon-134.md) | Animated floating label (Material style) |
| [135](jalon-135.md) | Grouped form validation (pure, app-side) |
| [136](jalon-136.md) | Programmatic focus (making `first_invalid` actionable) |
| [137](jalon-137.md) | Multi-line field |
| [138](jalon-138.md) | Automatic text wrapping (word-wrap) |
| [139](jalon-139.md) | Multi-line field scrolling (wheel) |
| [140](jalon-140.md) | Multi-line field scrollbar (+ touch) |
| [141](jalon-141.md) | Up/Down arrows in the multi-line field |
| [142](jalon-142.md) | Remembered goal column + Page Up/Down |
| [143](jalon-143.md) | Word jump (Ctrl+Arrows) & field bounds (Ctrl+Home/End) |
| [144](jalon-144.md) | Label notch (`outlined` style) |
| [145](jalon-145.md) | Table: sortable header & selectable rows |
| [146](jalon-146.md) | Time picker (`TimePicker`) |
| [147](jalon-147.md) | Date + time flow, fine-grained minutes & 12-hour AM/PM |
| [148](jalon-148.md) | Table: multiple selection & variable-width columns |
| [149](jalon-149.md) | Table: indeterminate "check all" & keyboard sorting |
| [150](jalon-150.md) | `Dropdown` / `Autocomplete` audit: bringing them up to standard |
| [151](jalon-151.md) | Table: mouse column resizing |
| [152](jalon-152.md) | Autocomplete: text highlighting & active suggestion |
| [153](jalon-153.md) | Table: column reordering (dragging a header) |
| [154](jalon-154.md) | Autocomplete: scrollable suggestion list |
| [155](jalon-155.md) | Column reordering: sliding preview |
| [156](jalon-156.md) | Range slider (two handles) |
| [157](jalon-157.md) | Range slider: sticky handle & discrete step |
| [158](jalon-158.md) | Reordering: faithful ghost (text included) |
| [159](jalon-159.md) | Reordering: neighbouring columns slide |
| [160](jalon-160.md) | Range slider: value tooltip & keyboard |
| [161](jalon-161.md) | Reordering: keyboard & continuous sliding |
| [162](jalon-162.md) | Range slider: hover, track click & Home/End |
| [163](jalon-163.md) | Reordering: gentle inertia & announced headers |
| [164](jalon-164.md) | Table: widget cells (beyond text) |
| [165](jalon-165.md) | Accessibility: spoken announcements (live region) |
| [166](jalon-166.md) | Table: adaptive row height |
| [167](jalon-167.md) | Accessibility: sort and selection announcements |
| [168](jalon-168.md) | Table: icon headers (+ sorting widget columns) |
| [169](jalon-169.md) | Accessibility: announced row selection |
| [170](jalon-170.md) | Table: action widget in the header |
| [171](jalon-171.md) | Table: fully widget header |
| [172](jalon-172.md) | Table: column menu from the keyboard |
| [173](jalon-173.md) | Table: virtualised rows |
| [174](jalon-174.md) | Focus trap for open menus |
| [175](jalon-175.md) | Focus restored when an overlay closes |
| [176](jalon-176.md) | Virtualised table: widget rows |
| [177](jalon-177.md) | Virtualised table: multiple selection |
| [178](jalon-178.md) | Table: frozen columns |
| [179](jalon-179.md) | Frozen columns: separator shadow & freezing on the right |
| [180](jalon-180.md) | Forms: cross-field validation & error summary |
| [181](jalon-181.md) | Forms: clickable error summary |
| [182](jalon-182.md) | Multi-step form: `Steps` indicator |
| [183](jalon-183.md) | `Steps` indicator: clickable markers |
| [184](jalon-184.md) | DatePicker: selecting a date range |
| [185](jalon-185.md) | Snackbar: action + queue |
| [186](jalon-186.md) | DatePicker: dual calendar (long ranges) |
| [187](jalon-187.md) | TimePicker: time range (start → end slot) |
| [188](jalon-188.md) | ToastHost: positioning, stacking, transition |
| [189](jalon-189.md) | DateTimeRange: date + time range |
| [190](jalon-190.md) | Integrated sign-up wizard (end-to-end demo) |
| [191](jalon-191.md) | Button: disabled state |
| [192](jalon-192.md) | Wizard: per-step validation, programmatic focus, masked passwords |
| [193](jalon-193.md) | Snackbar: animated exit + queue wired in |
| [194](jalon-194.md) | Wizard: revealing the password |
| [195](jalon-195.md) | Steps: "completed" state driven by validity |
| [196](jalon-196.md) | Table: inline cell editing |
| [197](jalon-197.md) | Editable grid: interactive wiring |
| [198](jalon-198.md) | TextInput: clickable suffix (positional click) |
| [199](jalon-199.md) | Charts: bar chart |
| [200](jalon-200.md) | Charts: line chart (LineChart) |
| [201](jalon-201.md) | Editable grid: keyboard navigation + rows |
| [202](jalon-202.md) | Eye icon + password reveal inside the field |
| [203](jalon-203.md) | Charts: y-axis + grid (shared) |
| [204](jalon-204.md) | Grid: header sorting + per-cell validation |
| [205](jalon-205.md) | System cursor per sub-region |
| [206](jalon-206.md) | Charts: filled area under the curve |
| [207](jalon-207.md) | Grid: submission guarded by validation |
| [208](jalon-208.md) | Sub-region highlight on hover |
| [209](jalon-209.md) | Charts: multiple series + legend |
| [210](jalon-210.md) | Grid: Save disabled + jump to the first error |
| [211](jalon-211.md) | Charts: sub-region tooltip on hover |
| [212](jalon-212.md) | BarChart brought up to LineChart's level: grouped + legend + tooltip |
| [213](jalon-213.md) | LineChart: stacked areas |
| [214](jalon-214.md) | Grid: cycling through the errors |
| [215](jalon-215.md) | Charts: clickable legend + hideable series |
| [216](jalon-216.md) | BarChart: stacked bars |
| [217](jalon-217.md) | Charts: animated pulsing halo on hover |
| [218](jalon-218.md) | Demo: "Charts" screen with a clickable legend |
| [219](jalon-219.md) | Charts demo: type selector |
| [220](jalon-220.md) | Charts demo: companion chart sharing the visibility |
| [221](jalon-221.md) | Clicking a chart point → pinned detail |
| [222](jalon-222.md) | Clicking a bar: pinned detail (BarChart::on_point) |
| [223](jalon-223.md) | Pinned point/bar highlighted (persistent halo + ring) |
| [224](jalon-224.md) | 100% stacking (normalised proportions) |
| [225](jalon-225.md) | Unpinning on a second click |
| [226](jalon-226.md) | Percentages in the tooltip in 100% mode |
| [227](jalon-227.md) | Share label (%) in each band (100% bars) |
| [228](jalon-228.md) | Total on top of absolute stacked columns |
| [229](jalon-229.md) | Value in each band (absolute stacked bars) |
| [230](jalon-230.md) | Value/share in each band (stacked areas) |
| [231](jalon-231.md) | Bounded DatePicker (days disabled outside [min, max]) |
| [232](jalon-232.md) | Self-sorting DataTable (reusable widget) |
| [233](jalon-233.md) | The DataTable's internal pagination |
| [234](jalon-234.md) | Bounded-range DatePicker (range + [min, max] window) |
| [235](jalon-235.md) | Blackout days / selection predicate (DatePicker) |
| [236](jalon-236.md) | DataTable: page size + "N–M of T" label |
| [237](jalon-237.md) | Demo: Data table screen (DataTable wired in) |
| [238](jalon-238.md) | Demo: filtered calendar (weekends greyed out) |
| [239](jalon-239.md) | DataTable: selected row (source index ↔ displayed position mapping) |
| [240](jalon-240.md) | DataTable: custom sort key per column |
| [241](jalon-241.md) | DataTable: multiple selection (checkboxes) |
| [242](jalon-242.md) | DataTable: search/filter |
| [243](jalon-243.md) | DataTable: bulk action bar |
| [244](jalon-244.md) | DataTable: empty state ("No results") |
| [245](jalon-245.md) | Demo: confirmation before a bulk delete |
| [247](jalon-247.md) | Kanban: columns + cards, cross-column drag and drop |
| [248](jalon-248.md) | Kanban: vertical drop preview |
| [249](jalon-249.md) | Kanban: rich cards + add/remove |
| [250](jalon-250.md) | Reorderables registry (card dragging works) |
| [251](jalon-251.md) | Drag ghost including a rich card's content |
| [252](jalon-252.md) | Insertion indicator between cards (hovered half) |
| [253](jalon-253.md) | Neighbouring cards shift on vertical insertion (the "gap") |
| [254](jalon-254.md) | Cross-cutting drag-and-drop review: Table + Kanban fixes |
| [255](jalon-255.md) | Drag-and-drop painting moved onto the theme / named constants |
| [256](jalon-256.md) | Consolidation: transformed registries (ui.rs) + shared reorder factor |
| [257](jalon-257.md) | Android keyboard fix: reopening the keyboard when a field is tapped again |
| [258](jalon-258.md) | Respecting the viewport: scrollable Kanban board + wrapped text (end of overflow) |
| [259](jalon-259.md) | Application lifecycle contract |
| [260](jalon-260.md) | Kanban scrolling: a deliberate horizontal axis (end of the 2D pan) |
| [261](jalon-261.md) | DnD polish: themed `Card`/`Toast` shadows + same-column reorder test |
| [262](jalon-262.md) | Overflow sweep across the screens (scrollable tables + wrapped text + vertical bodies) |
| [263](jalon-263.md) | Per-column vertical scrolling: layout blocker + reorderables-inside-Scroll guard |
| [264](jalon-264.md) | Per-column vertical scrolling (Trello style), via an explicit height |
| [265](jalon-265.md) | Vertical drag inertia (spring-loaded insertion line) |
| [266](jalon-266.md) | Fill-then-scroll: per-column vertical scrolling **without an explicit height** |
| [267](jalon-267.md) | A **single** entry point, one entry per platform |
| [268](jalon-268.md) | `frus` facade crate: **a single dependency** |
| [269](jalon-269.md) | `compute_scroll` **fills the constrained axis** (end of the filler container) |
| [270](jalon-270.md) | **Asynchronous** effects (`perform_async` / `run_async`) |
| [271](jalon-271.md) | Cross-platform `fetch` helper (`net` feature) |
| [272](jalon-272.md) | `Request`: POST, headers and timeout on `fetch` (`net` feature) |
| [273](jalon-273.md) | End-to-end network example (`frus-fetch-example`) |
| [274](jalon-274.md) | `RemoteData<T, E>`: the Elm idiom for asynchronous data |
| [275](jalon-275.md) | Typed JSON on `Request` (`json` feature) |
