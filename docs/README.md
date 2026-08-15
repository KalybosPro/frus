# Design notes

This directory is the project's memory: **304 milestone notes**, one per step of
frus's construction. Each records the objective, the alternatives that were weighed, the
decision and its reasoning, the implementation, how it was verified, and what was
deliberately left for later.

When you find yourself asking *"why on earth is it done this way?"*, the answer is almost
always here — along with the option that was rejected and why. `grep` this directory before
opening an issue.

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
| [0](milestone-0.md) – [4](milestone-4.md) | The foundations: a window, the 2D renderer, layout, the widget tree, interactivity |
| [129](milestone-129.md), [131](milestone-131.md) | The web target, and shrinking the wasm payload |
| [267](milestone-267.md), [268](milestone-268.md) | The single entry point (`main!`) and the `frus` facade crate |
| [270](milestone-270.md) – [275](milestone-275.md) | Async effects, `fetch`, `RemoteData`, typed JSON |
| [276](milestone-276.md) | Named platform `cfg`s — how a new target gets added without breaking the others |

## All milestones

| # | Title |
| --- | --- |
| [0](milestone-0.md) | A window + a coloured quad |
| [1](milestone-1.md) | Minimal 2D renderer (primitives) |
| [2](milestone-2.md) | Layout engine (flexbox via taffy) |
| [3](milestone-3.md) | Declarative widget tree |
| [4](milestone-4.md) | Interactivity: events + state |
| [5](milestone-5.md) | Text |
| [6](milestone-6.md) | Widget identity + interaction states |
| [7](milestone-7.md) | Style: rounded corners, borders, alignment, per-side padding |
| [8](milestone-8.md) | Text input + keyboard focus |
| [9](milestone-9.md) | Vertical scrolling + clipping |
| [10](milestone-10.md) | Caret, navigation, selection and clipboard |
| [11](milestone-11.md) | Animations (implicit transitions) |
| [12](milestone-12.md) | Shadows, gradients, horizontal scrolling, scrollbar & drag, focus animations |
| [13](milestone-13.md) | Opacity + fade-in |
| [14](milestone-14.md) | Fade-out (retaining outgoing widgets) |
| [15](milestone-15.md) | Theme system |
| [16](milestone-16.md) | Library of named (themed) widgets |
| [17](milestone-17.md) | Overlay / portal (floating menus, tooltips, modals) |
| [18](milestone-18.md) | Navigation (screen stack + slide transitions) |
| [19](milestone-19.md) | State transitions · Back gesture · Advanced overlay |
| [20](milestone-20.md) | A real example app: todo list |
| [21](milestone-21.md) | Framework / application split (`run(app)`) |
| [22](milestone-22.md) | Navigation bar (`NavBar`) + animated titles |
| [23](milestone-23.md) | Scrolling with inertia (spring + bounce) |
| [24](milestone-24.md) | `Command` / effects from `update` |
| [25](milestone-25.md) | DPI / scale factor (HiDPI) |
| [26](milestone-26.md) | Subscriptions (continuous message sources) |
| [27](milestone-27.md) | DX / ergonomics (writing a UI faster) |
| [28](milestone-28.md) | Keyed reconciliation (stable identity) |
| [29](milestone-29.md) | Keyboard navigation / accessibility |
| [30](milestone-30.md) | Window robustness |
| [31](milestone-31.md) | Virtualised list (`List`) |
| [32](milestone-32.md) | New widgets (6) |
| [33](milestone-33.md) | New widgets: Collapsible, Menu, Chip |
| [34](milestone-34.md) | New widgets: Avatar, Stepper, Rating |
| [35](milestone-35.md) | Layout: grid (`Grid`) |
| [36](milestone-36.md) | New widgets: Table, SegmentedControl, Toast |
| [37](milestone-37.md) | New widgets: Breadcrumb, Pagination, Skeleton |
| [38](milestone-38.md) | New widgets: Tree, ColorPicker, Timeline |
| [39](milestone-39.md) | Click fix + new widgets: DatePicker, Carousel, Alert |
| [40](milestone-40.md) | New widgets: Popover, Autocomplete, Kbd |
| [41](milestone-41.md) | sRGB / linear colour handling |
| [42](milestone-42.md) | Responsive by default |
| [43](milestone-43.md) | Adaptive layout (navigation & master-detail) |
| [44](milestone-44.md) | Dynamic scale & size |
| [45](milestone-45.md) | Advanced widget responsiveness |
| [46](milestone-46.md) | Drawer animation (slide + fade) |
| [47](milestone-47.md) | Right drawer & permanent drawer |
| [48](milestone-48.md) | Drawer slide on a spring curve |
| [49](milestone-49.md) | Modal sheet (`BottomSheet`) |
| [50](milestone-50.md) | First run on physical Android |
| [51](milestone-51.md) | System insets (safe area / SafeArea) |
| [52a](milestone-52a.md) | Adaptive AppBar (Material app bar) |
| [52b](milestone-52b.md) | Unified `Scaffold` (Material screen skeleton) |
| [53](milestone-53.md) | Unified physics (`trait Simulation`) |
| [54](milestone-54.md) | Reachable animation layer + the demo's transitions on top of it |
| [55](milestone-55.md) | Relayout boundary cache (retained layout on top of taffy) |
| [56](milestone-56.md) | Frame phases: conditional build (build → paint) |
| [57](milestone-57.md) | `BoxDecoration`: the box decoration model (§5) |
| [58](milestone-58.md) | Theme: baked-in Material state layers + extended M3 roles |
| [59](milestone-59.md) | Generalising the Material state layers |
| [60](milestone-60.md) | Typography: `TextStyle` + `TextTheme` (weight and italic rendered) |
| [61](milestone-61.md) | Fully customisable AppBar/NavBar |
| [62](milestone-62.md) | `TextSpan`: rich text, from the styled tree to the GPU |
| [63](milestone-63.md) | `TextLayout`: caret, hit-testing and selection on cosmic-text |
| [64](milestone-64.md) | Measuring under constraints (taffy closures) + wrapping paragraph |
| [65](milestone-65.md) | `RichText::wrap()`: the wrapped rich paragraph |
| [66](milestone-66.md) | `BorderRadius`: **per-corner** radii (SDF) |
| [67](milestone-67.md) | Adopting per-corner (sheet, segments) + the border reserves its space |
| [68](milestone-68.md) | `ColorScheme`: consolidated roles (single source of truth) |
| [69](milestone-69.md) | Gestures, stages 0+1: normalised input + long press |
| [70](milestone-70.md) | Focus: keyboard-only ring + arrow navigation (geometric) |
| [71](milestone-71.md) | Leaf→root key handling (3 states): Escape closes everywhere |
| [72](milestone-72.md) | Focus scopes: the modal traps Tab, arrows and clicks |
| [73](milestone-73.md) | Touch fling: scroll momentum (ballistics) |
| [74](milestone-74.md) | Window insets: `padding` / `viewInsets` split (keyboard avoidance) |
| [75](milestone-75.md) | Text decorations (underline, strikethrough, highlight) |
| [76](milestone-76.md) | `from_seed`: theme generated from a seed colour (HCT) |
| [77](milestone-77.md) | `frus-test`: headless rendering, snapshots and goldens (opening §13) |
| [78](milestone-78.md) | Runtime inspector (§13, stage 1) |
| [79](milestone-79.md) | State-preserving live reload (§13) |
| [80](milestone-80.md) | Android soft keyboard (opening the §6 input work) |
| [81](milestone-81.md) | Android InputConnection bridge (§6, stage 2) |
| [82](milestone-82.md) | IME input, stage 3: styled composition + suggestion context |
| [83](milestone-83.md) | One-command start (`cargo generate`): closing §13 |
| [84](milestone-84.md) | RTL: reading direction and layout mirroring (§14, opening) |
| [85](milestone-85.md) | Accessibility: semantic annotation + AccessKit bridge |
| [86](milestone-86.md) | Localisation (i18n/l10n): Fluent |
| [87](milestone-87.md) | Arabic script (bidi): script rendering + off-screen RTL fix |
| [88](milestone-88.md) | Frame phases & repaint boundary cache |
| [89](milestone-89.md) | Vector paths & icons |
| [90](milestone-90.md) | Images & textures |
| [91](milestone-91.md) | Image decoding (PNG/JPEG) |
| [92](milestone-92.md) | Layer compositing & pipeline precompilation |
| [93](milestone-93.md) | Anti-aliasing (MSAA) |
| [94](milestone-94.md) | GPU reuse of layer textures |
| [95](milestone-95.md) | Implicit animations: per-widget curve & duration |
| [96](milestone-96.md) | Group opacity & `AnimatedOpacity` |
| [97](milestone-97.md) | `AnimatedContainer`: animated background colour |
| [98](milestone-98.md) | `AnimatedContainer`: animated size (at layout) |
| [99](milestone-99.md) | `AnimatedContainer`: animated corner radius |
| [100](milestone-100.md) | Named widgets: `Opacity`, `AnimatedOpacity`, `AnimatedContainer` |
| [101](milestone-101.md) | Explicit animations: `repeat` / `stop` / `reset` |
| [102](milestone-102.md) | `AnimatedContainer`: animated padding |
| [103](milestone-103.md) | `Animatable`: the explicit → live typed value bridge |
| [104](milestone-104.md) | Composed `Animatable`s: `TweenSequence` + box tweens |
| [105](milestone-105.md) | Container: `alignment` + composite `decoration` |
| [106](milestone-106.md) | Fractional `Alignment` + `Tween<Alignment>` (manual placement) |
| [107](milestone-107.md) | Anchoring: virtualised lists + `AlignmentDirectional` |
| [108](milestone-108.md) | `AlignmentGeometry`: unified anchoring |
| [109](milestone-109.md) | Container: outer margin (`margin`) |
| [110](milestone-110.md) | `AspectRatio`: a box with a width/height ratio |
| [111](milestone-111.md) | `FractionallySizedBox`: size as a fraction of the parent |
| [112](milestone-112.md) | `Transform`: paint offset (`translate`) |
| [113](milestone-113.md) | `Transform`: paint scale (`scale`) |
| [114](milestone-114.md) | `Transform`: rotation (rotated composited layer) |
| [115](milestone-115.md) | `Transform`: non-uniform scale (`scale_xy`) |
| [116](milestone-116.md) | `Transform`: composition (translate + scale + rotation) |
| [117](milestone-117.md) | `Transform`: unified affine matrix |
| [118](milestone-118.md) | `Transform`: focus/a11y follow the scale (axis-aligned case) |
| [119](milestone-119.md) | Animated showcase: `frus-transforms` |
| [120](milestone-120.md) | Pixel tests for the transform pipeline |
| [121](milestone-121.md) | Shape clipping: `ClipRRect` / `ClipOval` |
| [122](milestone-122.md) | `InteractiveViewer`: pan + zoom (pinch/wheel) |
| [123](milestone-123.md) | Extended showcase: Clip + InteractiveViewer |
| [124](milestone-124.md) | `FittedBox` + `RotatedBox`: transforms that affect layout |
| [125](milestone-125.md) | **Per-corner** rounded clipping (`ClipRRect` + `BorderRadius`) |
| [126](milestone-126.md) | `InteractiveViewer`: inertia (fling) + pan bounds |
| [127](milestone-127.md) | `ClipPath`: clipping to an arbitrary path (mask pipeline) |
| [128](milestone-128.md) | Showcase: ClipPath + RotatedBox + FittedBox |
| [129](milestone-129.md) | Web target (wasm + WebGPU) |
| [130](milestone-130.md) | Effects & subscriptions on the Web |
| [131](milestone-131.md) | Slimming down the `.wasm` |
| [132](milestone-132.md) | Decorated form field (label, hint, help, error) |
| [133](milestone-133.md) | Password field (masking) + prefix/suffix icons |
| [134](milestone-134.md) | Animated floating label (Material style) |
| [135](milestone-135.md) | Grouped form validation (pure, app-side) |
| [136](milestone-136.md) | Programmatic focus (making `first_invalid` actionable) |
| [137](milestone-137.md) | Multi-line field |
| [138](milestone-138.md) | Automatic text wrapping (word-wrap) |
| [139](milestone-139.md) | Multi-line field scrolling (wheel) |
| [140](milestone-140.md) | Multi-line field scrollbar (+ touch) |
| [141](milestone-141.md) | Up/Down arrows in the multi-line field |
| [142](milestone-142.md) | Remembered goal column + Page Up/Down |
| [143](milestone-143.md) | Word jump (Ctrl+Arrows) & field bounds (Ctrl+Home/End) |
| [144](milestone-144.md) | Label notch (`outlined` style) |
| [145](milestone-145.md) | Table: sortable header & selectable rows |
| [146](milestone-146.md) | Time picker (`TimePicker`) |
| [147](milestone-147.md) | Date + time flow, fine-grained minutes & 12-hour AM/PM |
| [148](milestone-148.md) | Table: multiple selection & variable-width columns |
| [149](milestone-149.md) | Table: indeterminate "check all" & keyboard sorting |
| [150](milestone-150.md) | `Dropdown` / `Autocomplete` audit: bringing them up to standard |
| [151](milestone-151.md) | Table: mouse column resizing |
| [152](milestone-152.md) | Autocomplete: text highlighting & active suggestion |
| [153](milestone-153.md) | Table: column reordering (dragging a header) |
| [154](milestone-154.md) | Autocomplete: scrollable suggestion list |
| [155](milestone-155.md) | Column reordering: sliding preview |
| [156](milestone-156.md) | Range slider (two handles) |
| [157](milestone-157.md) | Range slider: sticky handle & discrete step |
| [158](milestone-158.md) | Reordering: faithful ghost (text included) |
| [159](milestone-159.md) | Reordering: neighbouring columns slide |
| [160](milestone-160.md) | Range slider: value tooltip & keyboard |
| [161](milestone-161.md) | Reordering: keyboard & continuous sliding |
| [162](milestone-162.md) | Range slider: hover, track click & Home/End |
| [163](milestone-163.md) | Reordering: gentle inertia & announced headers |
| [164](milestone-164.md) | Table: widget cells (beyond text) |
| [165](milestone-165.md) | Accessibility: spoken announcements (live region) |
| [166](milestone-166.md) | Table: adaptive row height |
| [167](milestone-167.md) | Accessibility: sort and selection announcements |
| [168](milestone-168.md) | Table: icon headers (+ sorting widget columns) |
| [169](milestone-169.md) | Accessibility: announced row selection |
| [170](milestone-170.md) | Table: action widget in the header |
| [171](milestone-171.md) | Table: fully widget header |
| [172](milestone-172.md) | Table: column menu from the keyboard |
| [173](milestone-173.md) | Table: virtualised rows |
| [174](milestone-174.md) | Focus trap for open menus |
| [175](milestone-175.md) | Focus restored when an overlay closes |
| [176](milestone-176.md) | Virtualised table: widget rows |
| [177](milestone-177.md) | Virtualised table: multiple selection |
| [178](milestone-178.md) | Table: frozen columns |
| [179](milestone-179.md) | Frozen columns: separator shadow & freezing on the right |
| [180](milestone-180.md) | Forms: cross-field validation & error summary |
| [181](milestone-181.md) | Forms: clickable error summary |
| [182](milestone-182.md) | Multi-step form: `Steps` indicator |
| [183](milestone-183.md) | `Steps` indicator: clickable markers |
| [184](milestone-184.md) | DatePicker: selecting a date range |
| [185](milestone-185.md) | Snackbar: action + queue |
| [186](milestone-186.md) | DatePicker: dual calendar (long ranges) |
| [187](milestone-187.md) | TimePicker: time range (start → end slot) |
| [188](milestone-188.md) | ToastHost: positioning, stacking, transition |
| [189](milestone-189.md) | DateTimeRange: date + time range |
| [190](milestone-190.md) | Integrated sign-up wizard (end-to-end demo) |
| [191](milestone-191.md) | Button: disabled state |
| [192](milestone-192.md) | Wizard: per-step validation, programmatic focus, masked passwords |
| [193](milestone-193.md) | Snackbar: animated exit + queue wired in |
| [194](milestone-194.md) | Wizard: revealing the password |
| [195](milestone-195.md) | Steps: "completed" state driven by validity |
| [196](milestone-196.md) | Table: inline cell editing |
| [197](milestone-197.md) | Editable grid: interactive wiring |
| [198](milestone-198.md) | TextInput: clickable suffix (positional click) |
| [199](milestone-199.md) | Charts: bar chart |
| [200](milestone-200.md) | Charts: line chart (LineChart) |
| [201](milestone-201.md) | Editable grid: keyboard navigation + rows |
| [202](milestone-202.md) | Eye icon + password reveal inside the field |
| [203](milestone-203.md) | Charts: y-axis + grid (shared) |
| [204](milestone-204.md) | Grid: header sorting + per-cell validation |
| [205](milestone-205.md) | System cursor per sub-region |
| [206](milestone-206.md) | Charts: filled area under the curve |
| [207](milestone-207.md) | Grid: submission guarded by validation |
| [208](milestone-208.md) | Sub-region highlight on hover |
| [209](milestone-209.md) | Charts: multiple series + legend |
| [210](milestone-210.md) | Grid: Save disabled + jump to the first error |
| [211](milestone-211.md) | Charts: sub-region tooltip on hover |
| [212](milestone-212.md) | BarChart brought up to LineChart's level: grouped + legend + tooltip |
| [213](milestone-213.md) | LineChart: stacked areas |
| [214](milestone-214.md) | Grid: cycling through the errors |
| [215](milestone-215.md) | Charts: clickable legend + hideable series |
| [216](milestone-216.md) | BarChart: stacked bars |
| [217](milestone-217.md) | Charts: animated pulsing halo on hover |
| [218](milestone-218.md) | Demo: "Charts" screen with a clickable legend |
| [219](milestone-219.md) | Charts demo: type selector |
| [220](milestone-220.md) | Charts demo: companion chart sharing the visibility |
| [221](milestone-221.md) | Clicking a chart point → pinned detail |
| [222](milestone-222.md) | Clicking a bar: pinned detail (BarChart::on_point) |
| [223](milestone-223.md) | Pinned point/bar highlighted (persistent halo + ring) |
| [224](milestone-224.md) | 100% stacking (normalised proportions) |
| [225](milestone-225.md) | Unpinning on a second click |
| [226](milestone-226.md) | Percentages in the tooltip in 100% mode |
| [227](milestone-227.md) | Share label (%) in each band (100% bars) |
| [228](milestone-228.md) | Total on top of absolute stacked columns |
| [229](milestone-229.md) | Value in each band (absolute stacked bars) |
| [230](milestone-230.md) | Value/share in each band (stacked areas) |
| [231](milestone-231.md) | Bounded DatePicker (days disabled outside [min, max]) |
| [232](milestone-232.md) | Self-sorting DataTable (reusable widget) |
| [233](milestone-233.md) | The DataTable's internal pagination |
| [234](milestone-234.md) | Bounded-range DatePicker (range + [min, max] window) |
| [235](milestone-235.md) | Blackout days / selection predicate (DatePicker) |
| [236](milestone-236.md) | DataTable: page size + "N–M of T" label |
| [237](milestone-237.md) | Demo: Data table screen (DataTable wired in) |
| [238](milestone-238.md) | Demo: filtered calendar (weekends greyed out) |
| [239](milestone-239.md) | DataTable: selected row (source index ↔ displayed position mapping) |
| [240](milestone-240.md) | DataTable: custom sort key per column |
| [241](milestone-241.md) | DataTable: multiple selection (checkboxes) |
| [242](milestone-242.md) | DataTable: search/filter |
| [243](milestone-243.md) | DataTable: bulk action bar |
| [244](milestone-244.md) | DataTable: empty state ("No results") |
| [245](milestone-245.md) | Demo: confirmation before a bulk delete |
| [247](milestone-247.md) | Kanban: columns + cards, cross-column drag and drop |
| [248](milestone-248.md) | Kanban: vertical drop preview |
| [249](milestone-249.md) | Kanban: rich cards + add/remove |
| [250](milestone-250.md) | Reorderables registry (card dragging works) |
| [251](milestone-251.md) | Drag ghost including a rich card's content |
| [252](milestone-252.md) | Insertion indicator between cards (hovered half) |
| [253](milestone-253.md) | Neighbouring cards shift on vertical insertion (the "gap") |
| [254](milestone-254.md) | Cross-cutting drag-and-drop review: Table + Kanban fixes |
| [255](milestone-255.md) | Drag-and-drop painting moved onto the theme / named constants |
| [256](milestone-256.md) | Consolidation: transformed registries (ui.rs) + shared reorder factor |
| [257](milestone-257.md) | Android keyboard fix: reopening the keyboard when a field is tapped again |
| [258](milestone-258.md) | Respecting the viewport: scrollable Kanban board + wrapped text (end of overflow) |
| [259](milestone-259.md) | Application lifecycle contract |
| [260](milestone-260.md) | Kanban scrolling: a deliberate horizontal axis (end of the 2D pan) |
| [261](milestone-261.md) | DnD polish: themed `Card`/`Toast` shadows + same-column reorder test |
| [262](milestone-262.md) | Overflow sweep across the screens (scrollable tables + wrapped text + vertical bodies) |
| [263](milestone-263.md) | Per-column vertical scrolling: layout blocker + reorderables-inside-Scroll guard |
| [264](milestone-264.md) | Per-column vertical scrolling (Trello style), via an explicit height |
| [265](milestone-265.md) | Vertical drag inertia (spring-loaded insertion line) |
| [266](milestone-266.md) | Fill-then-scroll: per-column vertical scrolling **without an explicit height** |
| [267](milestone-267.md) | A **single** entry point, one entry per platform |
| [268](milestone-268.md) | `frus` facade crate: **a single dependency** |
| [269](milestone-269.md) | `compute_scroll` **fills the constrained axis** (end of the filler container) |
| [270](milestone-270.md) | **Asynchronous** effects (`perform_async` / `run_async`) |
| [271](milestone-271.md) | Cross-platform `fetch` helper (`net` feature) |
| [272](milestone-272.md) | `Request`: POST, headers and timeout on `fetch` (`net` feature) |
| [273](milestone-273.md) | End-to-end network example (`frus-fetch-example`) |
| [274](milestone-274.md) | `RemoteData<T, E>`: the Elm idiom for asynchronous data |
| [275](milestone-275.md) | Typed JSON on `Request` (`json` feature) |
| [276](milestone-276.md) | Clearing the ground for iOS: named platform `cfg`s |
