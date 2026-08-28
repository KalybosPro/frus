# Changelog

All notable changes to frus are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) — with the usual 0.x caveat that
any release may break.

> frus is **pre-alpha** and **not on crates.io**. Releases are tagged source releases:
> depend on them by `path` or by git revision. For the reasoning behind any individual
> decision, the milestone notes in [`docs/milestone-*.md`](docs/) remain the authoritative
> record — one per step, 439 so far, each documenting the objective, the alternatives
> weighed, and the decision.

## [Unreleased]

### Changed

- **A time picker's selected AM/PM takes the tertiary container** (J439), where it took the
  accent the selected hour takes. Picking an hour picks the value; picking AM says which half
  of the day it is in, and the reference gives the two different families for that reason.

- **An errored field deepens under the pointer** (J439): its border and label move from
  `error` to `on_error_container` while hovered, and back once focused. The message below the
  field does not — it is a sentence, not a control. `TextFieldStyle::error_hover_color` and
  the theme slot beside it replace the colour.

### Added

- **`SnackBar::close_icon`** (J438): the cross at the end of the bar. It takes the message a
  click emits rather than a `bool`, because the application owns the queue here and a cross
  with nothing to call would be a way out that is not one. With `close_icon_color` and
  `close_icon_label`, and `SnackBarTheme::close_icon_color` beside them.

### Fixed

- **A navigation destination's state layer was a translucent pill** (J437) — `muted` at
  12 %, handed to the GPU and therefore blended in linear light, where it paints like a
  third. It is the theme's own state rule now, resolved opaquely over the ground the
  destination stands on, with the reference's `primary` as its ink.

- **A selected destination did not respond to the pointer at all** (J437): the state layer
  lived in the `else` of "is it selected", so the one destination a pointer is most likely
  to be over was the one that never lit. The layer goes over the indicator.

- **`NavigationRail::background` and `BottomBar::background` did not invalidate their
  destinations** (J437). Harmless until the destinations began reading the background, which
  they must, a state layer being a lerp from the ground up.

### Added

- **A destination answers focus and press**, not only hover (J437) — they came with the
  theme's state rule.

### Added

- **A destination can be `disabled`** (J436), on `NavigationRail`, `BottomBar`, `Scaffold`
  and `NavScaffold`: its glyph and label take the disabled ink, nothing lights under the
  pointer, it emits no message and the keyboard steps over it. The indicator stays — greying
  a destination says you cannot go there now, not that you are not there.

- **`selected_icon`** (J436): the glyph a destination shows while it is selected, where that
  differs from its resting one. Unset, the resting one serves for both.

- **`indicator_color` per destination** (J436), over the theme's and the scheme's.

### Added

- **`NavScaffold::rail` and `::nav_labels`** (J435), the door `Scaffold` got in J434: a
  function run over the rail the shell built, and the label mode for whichever widget the
  size class chose.

### Changed

- **A `NavScaffold` at `SizeClass::Expanded` shows an extended rail** (J435) — labels beside
  the glyphs, 256 wide — where it showed the same glyph-only rail as `Medium`. Three size
  classes had two presentations between them, so the widest window got the portrait tablet's
  navigation. **This changes what an existing expanded window looks like**; `.rail(|rail|
  rail.extended(false))` declines it.

### Fixed

- **Describing a `NavScaffold`'s navigation after its `body` was silently ignored** (J435),
  `destination` included. `body` is what builds the navigation; the four builders that
  describe it now assert, and name themselves in the message.

### Added

- **`Scaffold::rail`** (J434): a function the shell runs over the `NavigationRail` it built
  for you — `.rail(|rail| rail.extended(true))`. One door instead of a pass-through per
  property, so everything the rail learns later is reachable the day it learns it. It runs
  after the destinations and after `nav_labels`, and is silent when the navigation is a bar.

- **`Scaffold::nav_labels`** (J434): the label mode, applied to whichever navigation widget
  the placement chose. Unsaid, each keeps the opposite default the reference gives it.

### Fixed

- **A persistent footer beside an extended rail was pushed off the screen** (J434). Its row
  is given its width, and that width subtracted the rail's *constant* rather than the rail's
  actual width — 176 pixels too wide once a caller extended it. The shell asks the rail now,
  after the caller has finished with it.

### Added

- **`NavigationRail::extended`** (J433): 256 across instead of 80, with every label beside
  its glyph instead of under it. The glyphs keep the column they had, so extending a rail
  widens it and moves nothing; the row is as tall as the taller of glyph and label rather
  than as tall as both; and every destination is labelled, whatever `RailLabels` says.

- **`NavigationRail::group_alignment`** (J433): where the destinations sit between the
  rail's two ends, `-1.0` (the default, against the top) to `1.0`, **continuously** — a
  third of the way down is a thing you can ask for.

- **`NavigationRail::leading` and `::trailing`** (J433), with `leading_boxed` and
  `trailing_boxed` for a slot that is already built: the slots above and below the
  destinations, where an application puts a floating action button or an account switcher.
  `leading_at_top` and `trailing_at_bottom` say which of them travels when the group moves;
  as the reference has it, the leading slot is pinned and the trailing one travels.

### Added

- **`RailLabels`** (J432): `None`, `Selected` or `All`, on both `NavigationRail::labels` and
  `BottomBar::labels`. The reference keeps two names for the one idea and gives them
  different defaults, which these follow.

### Changed

- **A `NavigationRail` shows no labels by default** (J432), as the reference's does — glyphs
  alone until it is told `.labels(RailLabels::All)`. A `BottomBar` still shows all of them,
  which is also the reference's default. **This changes what an existing rail looks like**;
  the one-line fix is on the rail.

- **A rail is 80 wide** (J432), the reference's `minWidth`, where it was 76.

### Fixed

- **The layout and the paint disagreed about a destination's label gap by two pixels**
  (J432) after milestone 431 moved the painted one to the reference's 4 and left the
  reserved one at 2. Hidden until now because the row's constant floor won every time.

### Changed

- **A navigation destination takes the roles the reference names** (J431). The selected
  indicator is an **opaque** `secondary_container` where it was `primary` at 16 % — the
  wrong role, and a translucent fill that never painted at the alpha it was written at. The
  glyph and the label stop sharing a colour: the glyph is drawn on the indicator and takes
  its content colour, the label sits below it and takes the surface's. An unselected label
  differs between a rail and a bar, as it does in the reference.

- **The rail's badge is the `Badge` widget's badge** (J431): the scheme's `error` and
  `on_error` through `BadgeTheme`, where it carried a red of its own. Recolouring badges now
  recolours both.

### Added

- **Six `NavRailTheme` slots** (J431) — the indicator, the selected and unselected glyph and
  label colours, and the glyph's size. All six had been hard-coded.

### Changed

- **`SnackBar` is the reference's inverted bar** (J430): `inverse_surface` with no border,
  `on_inverse_surface` text, an `inverse_primary` action, corner 4 and elevation 6. The
  scheme has carried the inverted pair since it was written, documented as being for toasts
  and snack bars, and the bar had never used it. Two of its three kinds now name a role;
  the third keeps a colour of this crate's own, because Material 3 has `error` and nothing
  that means "it worked".

### Added

- **Seven `SnackBarTheme` slots and four `SnackBar` builders** (J430) — background, text,
  action text, accent, the success colour, radius and elevation. All of those had been
  hard-coded, reachable by neither a caller nor a theme.

### Added

- **Ten colour roles the scheme was without** (J429): the `tertiary` four, `error_container`
  and `on_error_container`, `inverse_primary`, `surface_tint`, `surface_dim` and
  `surface_bright`. The seeded scheme generates the tertiary palette a sixth of the wheel
  from the seed at chroma 24, as the reference does; the hand-written schemes' literals were
  read off this crate's own HCT rather than chosen by eye. The contrast test now covers
  tertiary and all four containers, since a container carries text too.

### Changed

- **`Switch` takes the reference's whole off state** (J428), which is a design of five
  parts rather than a colour: a `surface_container_highest` track with a 2px `outline` edge
  that only the off end has, an `outline` thumb where the on one is `on_primary`, and a
  thumb that **grows** from radius 8 to 12 as the switch is flipped, on a 52×32 track. The
  growing thumb is what tells the two states apart before either colour is read.

- **A switch's two thumb colours are now two** (J428). `inactive_thumb_color` used to
  default to whatever the on thumb was; it defaults to the scheme's `outline`. It is still
  one thumb sliding — one that changes colour as it travels, the way the track under it
  does.

### Added

- **A surface for `NavigationRail` and `BottomBar`** (J427). Neither painted one: they drew
  a hairline and let whatever was behind them show through, so a bottom bar on a page was
  the page with a line above it. The reference gives them different rungs and the difference
  says what each is — a rail stands *beside* the page (`surface`), a bar stands *on* it
  (`surface_container`). Both take a `background(…)`, and `NavRailTheme` carries the two
  colours.

- **`BottomSheetTheme` and `BottomSheet::background`** (J427), the sheet's surface having
  been a hard-coded read that neither a caller nor a theme could reach. **`BottomAppBarTheme`**
  for the same reason on the theme side.

### Changed

- **Five more panels take the rung the reference names them** (J427): `Drawer` and
  `BottomSheet` to `surface_container_low`, `BottomAppBar` and the dropdown and autocomplete
  panels to `surface_container`. All five were filled from the flat `surface`.

### Added

- **The scheme's full container ladder** (J426): `surface_container_lowest`,
  `surface_container_low` and `surface_container_highest` join the two rungs that already
  existed, so the five roles the reference names together exist here too. The two old rungs
  keep their exact values; the new ones sit at the reference's own tonal steps measured from
  them, because this scheme's surface deliberately sits apart from the spec's and a ladder
  anchored elsewhere would put "more emphasis" below the page it stands on.

### Changed

- **Six call sites now take the rung the reference names them** (J426): an elevated card, an
  elevated button and a banner move to `surface_container_low`; a filled card and a filled
  text field to `surface_container_highest`; a menu to `surface_container`. Each was
  standing on the nearest rung the scheme had, and two of them said so in a comment.

### Added

- **`MaterialBanner`** (J425): a message across the top of the screen with the actions that
  answer it, staying until one is taken — the middle of the three ways of saying something,
  between a snack bar's few seconds and a dialog's barrier. Its actions are required, as in
  the reference: a message that stays until it is dismissed and offers no way to dismiss it
  stays for ever. One action rides on the message's line and two take a line of their own,
  and the rule along the bottom is drawn only where the banner is flat.

- **`SimpleDialog` and `SimpleDialogOption`** (J425), on the same surface as `AlertDialog`.
  The difference is what they are for: an alert dialog asks a question and puts the answers
  in a row of buttons; a simple dialog lists them, and each row **is** an answer — which is
  why an option's ink runs the full width of the dialog.

- **`BannerTheme`**, so a theme can set the banner's colour, elevation, tint, shadow, rule,
  paddings and text style.

### Added

- **`Dialog` and `AlertDialog`: the modal frus did not have** (J424). A rounded, elevated
  surface over the screen — 28 corner, elevation 6, `surfaceContainerHigh`, held off the
  window's edges, never narrower than 280, all of it the reference's numbers and all of it
  overridable on the instance and on the theme. `AlertDialog` adds the icon / title /
  content / actions column with the reference's conditional paddings, and an icon centres
  the title as the reference's does. Controlled like every other overlay here: `open` is the
  application's field, and a dialog told nothing about dismissal has an inert scrim, which is
  `barrierDismissible: false`.

- **`DialogTheme`**, so a theme can set the surface, the elevation, the shadow, the tint, the
  corner, the inset padding, the icon's colour and the two text styles.

### Changed

- **`AlertDialog` was renamed `Alert`** (J424) — a breaking rename, and the reason is that it
  was never a dialog: no actions, no barrier, and no message type. It is unchanged in every
  other respect, and the name now means what it means in the reference.

### Changed

- **A `Scaffold` no longer spends the system's bottom intrusion on the body's behalf** (J423),
  which is the reference's arrangement. With a bottom bar or a persistent footer below it,
  nothing changes — they hold the edge off as before. With **neither**, the body now reaches
  the screen's edge and is *told* that the gesture bar is there; a body whose content must be
  clear of it says `SafeArea::new(…)` and is answered. A screen that declines
  `resize_to_avoid_bottom_inset` likewise keeps the whole window, the keyboard being an
  overlay it has asked to ignore.

  This is a behaviour change for screens with no bottom slot. The one line that restores the
  old picture is `.body(SafeArea::new(content))`; the reason not to do it everywhere is that
  a full-bleed background, a hero image, and a list scrolling under the gesture bar were all
  impossible while the shell decided for them.

### Added

- **`AppBar::automatically_imply_leading` and `AppBar::automatically_imply_actions`** (J422),
  both `true` by default as in the reference. A bar with an empty leading slot, on a screen
  that has a drawer, grows the button that opens it; a bar with an empty trailing end grows
  the one for an end drawer. Neither ever adds a button beside what the caller put there.

- **`ScaffoldInfo` and `ScaffoldScope`: what the shell knows and its slots do not** (J422) —
  the third inherited thing, beside the theme and the surface, and the first to carry a
  *message* rather than plain data. A slot is handed to a shell already built, so the shell
  says what it is and the walk carries it down. `Widget::scaffold_override` is the hook.

### Fixed

- **A `SafeArea` inside a `Scaffold`'s body padded for intrusions the shell had already dealt
  with** (J421). The body was told nothing, so a safe area in it read the ambient description
  — the whole notch — and held its content off a second time: under an app bar, a whole
  status bar of empty space. The slot is handed a description now, as the reference does, and
  a body that wants the notch avoided can finally ask for it and be answered correctly.

### Changed

- A `Scaffold` lays its body out **full width** and tells it about the side intrusions rather
  than padding its content for them, as the reference does. A body that says nothing reaches
  the screen's edge — which is what a background or a hero image wants — and one that says
  `SafeArea` is held clear. Side intrusions are zero in portrait, so this is a landscape and
  display-cutout change.

### Fixed

- **A navigation rail's rule stopped at the notch** (J420). The shell padded the slot from
  outside, so the rail was a shorter box floated inside the intrusions and everything it
  painted — the rule down its trailing edge included — stopped with it. The rail now takes
  the leading side, the top and the bottom into its own box, as the reference's does, and its
  surface reaches the screen's edges while its destinations stay clear of them.

### Changed

- A `Scaffold`'s rail slot is handed a **description** rather than a padding, the trailing
  side removed. Beside a rail, the persistent footer no longer pads for a leading intrusion
  that is inside the rail's box.

### Fixed

- **Persistent footer buttons sat on the gesture bar when nothing was below them** (J419).
  The shell leaves the bottom clearance to whatever is bottom-most and steps the body aside
  when there is a footer; the footer then took nothing, because the bottom it passed its own
  padding was a literal zero. It is a real safe area now, with the top edge freed, and the
  shell tells the slot what there is to consume — nothing when a navigation bar below it
  already holds the edge off, as the reference does.

### Fixed

- **A bottom bar's surface stopped short of the screen's edge** (J418). The shell padded the
  slot from outside, so the bar's background ended above the gesture bar and a strip of the
  scaffold showed through underneath it. The reference keeps the colour outside the safe area
  and the safe area inside the bar; the shell now hands the slot a description and the bar
  consumes it, so the background runs behind the gesture bar and only its destinations are
  held clear.

- **A notched bottom app bar cut its notch a display cutout away from its button** (J418).
  The notch is cut in the bar's own coordinates and the bar used to start at the left
  intrusion, so it was moved back by it; the bar starts at the window's edge now. With no
  side intrusion the two readings agreed, which is why nothing caught it before.

### Changed

- A `Scaffold`'s navigation slot is handed a **description** rather than a padding, the top
  intrusion removed and the bottom left in, as the reference does. A widget in that slot that
  does not consume intrusions is no longer insetted for them.

### Added

- **`MediaScope`: a surface for one subtree** (J417), the counterpart of `Themed`. Until now
  a description could only be narrowed where a widget was *constructed* (`SafeArea::build`),
  which is the wrong end for a shell handed slots that are already built. Backed by a new
  `Widget::media_override`, applied by the layout walk, the deferred build, the relayout
  fingerprint and the paint walk alike.

- **`AppBar::primary`** (J417), `true` by default as in the reference. A bar keeps its own
  toolbar out of the status bar; its surface still runs behind it, which is what a Material
  bar looks like.

### Fixed

- **An `AppBar` used outside a `Scaffold` drew under the status bar** (J417). Nothing insetted
  it and it would not inset itself, because the shell owned that switch. The shell now says
  what there is to consume and the bar consumes it — the reference's arrangement, and the two
  halves can finally be told apart.

- **`SafeArea` answered with the surface in force when it was built**, not the one in force
  when it was asked (J417). Its reason for that expired at milestone 408, when the shell began
  holding one description across the build, the layout and the paint.

### Changed

- A `Scaffold`'s app-bar slot is handed a **description** rather than a padding. A widget in
  that slot that does not consume intrusions is no longer insetted for them — the reference
  behaves the same way, and the slot is meant for a bar.

### Fixed

- **Nothing inside an app bar ever faded in or out** (J416). The frame counts a new tree's
  identities with `collect_ids` *before* the layout pass has built its deferred subtrees, and
  an `AppBar` is built on one. The count returned the root and nothing else, so the mount and
  leave bookkeeping never saw a single widget in the bar. A widget moving from outside such a
  subtree to inside one was also snapshotted as leaving, and a ghost faded out over a widget
  still on the screen.

  This is the same defect J415 fixed in the shell's *other* build path, found by the tripwire
  J415 added. Both paths go through one `build_view` now.

### Added

- The first test that drives an `Application` through the shell's own code: an interface that
  lives entirely inside a deferred subtree, asserted to have identities to mount and to be
  reachable. It needs no window.

### Fixed

- **Typing fast into a field under an app bar stopped the caret following it** (J415). Keys
  can arrive faster than a frame, so the shell rebuilds the tree from `view` alone and reads
  it straight away. That tree had never been through a layout pass, and a `ThemeBuilder` —
  which an `AppBar` is built on — has no children until one has run over it. Every traversal
  into such a subtree returned nothing: no field found, no caret revealed, no focus resolved.

  Nothing logged and nothing looked wrong, because all thirty-eight call sites are `and_then`
  chains that shrug at `None`.

### Added

- `build_deferred(tree, &theme)` runs a tree's deferred builds the way the layout pass does,
  theme scoping included, for anything that reads a tree before laying it out.

- A **tripwire**: reading an unbuilt `ThemeBuilder`'s children panics in debug rather than
  answering "no children". The rule had been a comment for three milestones.

### Changed

- **The last seven widgets no longer decide their own type** (J414). `Breadcrumb`,
  `TimePicker`, `TimeRange`, `Kanban`, `ErrorSummary`, `Slider`'s value bubble, and the
  `AppBar` / `NavigationBar` titles resolve through the same chain as the rest — what the
  caller said, then `theme.widgets.<widget>`, then the step of `theme.text`.

  **This changes how they look.** An app bar's title is regular where it was medium, a
  navigation bar's is 22 px where it was 20, a slider's bubble is `labelLarge`, a time
  picker's cells `bodyLarge` and its help lines `labelMedium`, a breadcrumb `bodyMedium`, a
  board's cards `bodyLarge`, an error summary's heading a heading.

- `AppBar::title_style` is stored as an `Option<TextStyle>`; the `title_style_default`
  boolean beside it is gone.

### Fixed

- **A helper line's height was recomputed from its size** (J414). `TextField::sub_block`
  called `frus_text::line_height(FIELD_SUB_SIZE)` with `sub_style()` two methods away — the
  second survivor of J412's sweep, and written the one way that sweep could not find.

- **An app bar's title was medium where the reference's is regular** (J414). The test that
  covered it compared the title against the constant the title came from, so it would have
  passed at any value. `NavigationBar`'s default title — 20 px medium, both halves wrong —
  had no test at all.

### Changed

- **Twelve widgets no longer decide their own type** (J413). `Kbd`, `SnackBar`, `Tree`,
  `Timeline`, `Steps`, `NavigationRail`, `DatePicker`, `Alert`, `PopupMenuButton`,
  `DropdownButton`, `Autocomplete` and `Table` each held a private `const SIZE: f32` that no
  theme and no caller could reach. They resolve like every other widget now — what the caller
  said, then `theme.widgets.<widget>`, then the step of `theme.text` — and each gained the
  matching builder.

  **This changes how they look.** Read against the reference rather than against their own
  current values, eleven of the eighteen styles were wrong: a snackbar's content is 14 px and
  not 16, a menu and a dropdown are `titleMedium`, a data table's headings and cells are named
  apart (`titleSmall` / `bodyMedium`), a date picker's days are `bodyLarge`, a rail's labels
  carry a medium weight. `Kbd` is monospaced.

- `TextTheme::M3` is a `const`, and `Default` returns it. The un-themed `Widget::style` path
  reads its step from the same scale the themed one does, rather than from a fallback constant
  beside it.

### Added

- `TextStyle::height` and `TextStyle::family` builders (J413). Milestones 409 and 410 added
  both as public fields and neither as a builder, so a caller could name a font family only by
  writing the struct out.

### Fixed

- **A sort arrow landed inside the word it followed** (J413). `Table` measured its header
  label at a bare size rather than at the style it drew with, so any text scale above 1 put the
  indicator over the text. The same cell also centred its label from a recomputed line height —
  a survivor of J412, written against a constant instead of a style, which is the one
  formulation that sweep could not find.

- **Two paragraphs alike but for their leading shared a geometry** (J413). `Text::measure_key`
  hashed the resolved size, weight and slant, and neither `height` (J409) nor `family` (J410),
  both of which those milestones had put into the measurement. `AlertDialog::measure_key` had
  the same hole from the other end, hashing its text and nothing about its styles.

- **A calendar clipped its last column** (J413). `DatePicker` declared its width from the
  cell constant while its cells sized themselves from that constant *grown by the reader's font
  setting*. Seven cells against a box built for seven smaller ones.

- **A timeline's second line sat at a fixed offset** (J413), and its dot at half the row
  constant rather than half the row. The detail follows the title's own line box now, and a row
  is the floor *or what its two lines need*.

### Fixed

- **A named line height was ignored wherever text is centred** (J412). Milestone 409
  threaded `TextStyle::height` through the measurement and the paint but left twenty-four
  places computing `frus_text::line_height(style.size)` — the 1.2 default — while holding a
  style that said otherwise. A text with `height: 2.0` was measured tall, painted tall, and
  centred as though it were short.

  They ask `style.line_height()` now, which milestone 409 had already provided.

  One of the twenty-four is reachable by a test — a `max_lines` cap counts lines of the
  height that was asked for. The other twenty-three live in widgets whose text style is a
  private constant no caller can change, so a `height` cannot be set on them to prove it is
  honoured. That is recorded as the next step.

- **`Ui::wants_animation` answered for the whole process** (J411). It folded in
  `images_in_flight()`, a process-global count, so any tree built while an unrelated part
  of the program was loading an image asked for the next frame. In the test suite, where
  everything shares one process, that made a refresh area with nothing pulled flake about
  one run in ten.

  A `Ui` describes what **its own widgets** want. What the process is fetching is the
  shell's business, and the `||` now lives there. The reasoning for asking a *count* rather
  than reading a flag off `Image` — the fetch outlives the widget that started it, because
  showing a placeholder takes the image out of the tree — moved with it; it justified the
  count, never the place.

### Added

- **`TextStyle::family`** and `FontFamily` (J410), the reference's `fontFamily`.
  `Text::new("fn main()").family(FontFamily::Monospace)`, or `FontFamily::Named("Inter")`
  for a face registered with `add_font`. Fonts could already be registered globally; no
  widget could ask for one.

  `FontFamily` is `Copy` and `Named` borrows a `&'static str`, so `TextStyle` stays `Copy`
  and travels down a subtree by value like every other field.

  **A named family does not always win.** A run containing Arabic keeps the registered
  Arabic face, because cosmic-text does not fall back across families on Android — a family
  without Arabic coverage renders nothing at all, and text in an unexpected face is a
  smaller failure than a blank screen. Naming the Arabic family itself still works.

  Measure and paint call the same `family_for_style`, and the measurement cache is keyed on
  the family: different faces set the same words to different widths.

### Added

- **`TextStyle::height`** (J409), the reference's line height: a **multiple of the font
  size**, not a length. `Text::height(1.6)` opens a paragraph up; unset, it inherits from a
  surrounding `DefaultTextStyle` and falls back to `DEFAULT_LINE_HEIGHT`.

  A ratio because of the reader: at 1.5 a 20 px line is 30 px, and when the reader turns
  their font size up and that 20 becomes 40, the line becomes 60. A length would have stayed
  at 30 and closed the paragraph up exactly when it needed opening.

  It reaches the measurement, the one-line box floor and the paint through the one
  `ResolvedTextStyle::line_height()`.

### Changed

- **One line-height constant instead of two** (J409). `LINE_HEIGHT_FACTOR` lived in
  `frus-text`, which measures, and again in `frus-gpu`, which paints. Both were 1.2 so
  nothing was broken — but a measure and a paint disagreeing about how tall a line is puts
  the second line of every paragraph where the layout reserved nothing, and that is the
  shape two milestones have just been spent on. Both now import `DEFAULT_LINE_HEIGHT` from
  `frus-core`.

- **The measurement cache is keyed on the line height** (J409). It recorded text, size,
  weight, italic and width; two paragraphs of the same words at different leadings would
  have shared one answer, and the second would have been quietly wrong.

### Added

- **`MediaQuery::install`** and `SurfaceGuard` (J408): the surface installed until a guard
  is dropped, rather than for a closure. It holds the description **and** the reader's font
  size — one call, one drop, so the two can never be held for different lengths of time.
  `scope` remains, reimplemented on top of it, for the subtree case where a closure is the
  right shape.
- **A tripwire against half an installed surface** (J408). `build_ui` panics in debug builds
  when a text scale away from 1 is in force with no surface described, which means somebody
  installed one half and not the other.

  It caught its own subject's tests first: the three written in J406 to prove widgets follow
  the reader's font size used `MediaQuery::of().with_text_scaler(…)`, and `of()` outside a
  scope is `UNSET` — a surface of no size. They were right about their result and wrong
  about their setup, which is the gap that let J403 ship broken.

### Added

- **The platform reports what its user asked for** (J407). Android answers
  `Configuration.fontScale`, the night setting, `fontWeightAdjustment`, touch exploration,
  the animator duration scale and the clock format; desktop answers winit's theme. Until
  now nothing did, and milestones 403 and 406 were correct in tests and inert on a device.
- **`AccessibilityOverrides`** (J407), every field an `Option`. BREAKING:
  `Application::accessibility` returns it instead of `Accessibility`. The platform answers
  and the application overrides only what it chose to speak for — these settings belong to
  the person using the device. `None` per field is what makes "no opinion" expressible; a
  plain `Accessibility` could not say it, a `false` in it being the same as silence.
- **`install_text_scale`** and `TextScaleGuard` (J407): the reader's font size installed
  for as long as a guard lives, rather than for a closure.

### Fixed

- **The reader's font size never reached a real application** (J407). `MediaQuery::scope`
  wrapped `view` — the construction of the widgets — but a size becomes a number three
  times, and the two that decide how big text actually is, measuring and painting, ran at
  scale 1. The layout measured one size and the renderer drew another.

  This had been true since J403 landed. Nothing in the suite could see it: those tests wrap
  `build_ui_inspected` themselves, so the harness installed the condition the shell forgot.
  A device at `font_scale` 1.30 rendered pixel-identically to 1.0.

- **A crash on launch on every Android below API 31** (J407). A failed JNI call leaves the
  Java exception *pending on the thread*; discarding the Rust `Err` does not discard it, and
  the next JNI call aborts the runtime. `Configuration.fontWeightAdjustment` is API 31, so
  reading it on an API 29 device threw `NoSuchFieldError` and killed the process two calls
  later — in code that compiled perfectly, JNI resolving names at runtime.

- **The framework's animations ignored a platform that asked for less motion** (J407).
  `runtime.still` read the application's answer rather than the resolved one.

### Changed

- **`Scene::text` takes a resolved style; `Scene::text_styled` is gone** (J406). BREAKING.
  The two were the same primitive with two spellings, and the shorter one took a bare
  `f32` size — a size that never passed through `TextStyle::resolved()`, which is the one
  place a reader's font setting is applied. Forty-seven call sites across twenty-three
  widgets used it, so their text was the one thing on the screen a reader could not
  enlarge. No test could tell: the measurement went through the matching raw door, so
  paint and layout agreed with each other perfectly — on the wrong number.

  Deleting the door made every one of the forty-seven a compile error, and each had to say
  which it meant: `TextStyle::new(SIZE).resolved()` for anything a reader reads,
  `ResolvedTextStyle::exact(SIZE)` for a glyph that is an icon — a tick, a chevron, a star,
  the figure in a step marker — which lives in a box that does not move.

- **A component's default height is a floor, not a ceiling** (J406). `frus_text::line_box`
  is the reference's own rule written once: `max(theDefault, whatTheLineNeeds)`. `Chip` was
  a flat 32 px and cut its glyphs at a text scale of 2, where they need 34.

### Added

- **`TextStyle::clamp_scale`** (J406) and `APP_BAR_MAX_TITLE_SCALE`. The reference's second
  answer, for **chrome**: a toolbar cannot grow — it would push every screen down — so it
  keeps its height and caps the title's scaler at 1.34 instead. Below the cap the title
  follows the reader like anything else.
- **`ResolvedTextStyle::exact`**, `frus_text::measure_wrapped_resolved`,
  `frus_text::line_box` (J406).

### Fixed

- **`AlertDialog` measured and painted its body at different sizes** (J406), painting
  through `resolved()` and measuring raw — so above a text scale of 1 the box and the words
  disagreed. Introduced by J403, found by J406's sweep.
- **`TextField` placed its caret from an unresolved size** (J406). Multi-line went through
  `resolved()`, single-line did not, and `layout()` — which the caret, the hit-test and the
  selection all read — used the raw number. Above a scale of 1 the cursor landed where the
  glyphs were not. Everything in the field now goes through one `text_style()`.
- **The layout cache ignored the reader's font size** (J406). `signature_of` hashed styles,
  structure and measure keys but not the scale, so a reader who moved the system slider
  would have been served the previous frame's geometry under the new frame's glyphs.

### Changed

- **`Widget::main_axis_fill` is now `Widget::fill_axes`, answering with `FillAxes`** (J405)
  — BREAKING for anyone implementing `Widget` by hand, and mechanical: `Some(Row)` becomes
  `FillAxes::WIDTH`, `None` becomes `FillAxes::NONE`.

  The old hook returned one `FlexDirection`, so a widget wanting the room it is offered on
  **both** axes had no word for it. Milestone 404 had to leave `NavScaffold`,
  `ScaffoldMessenger` and `TwoPane` on the `width: 100%` that makes them vanish under a
  shrink-wrapping parent — the very bug it was fixing. They answer `FillAxes::BOTH` now.

  `FillAxes` is deliberately not the walk's internal `Fills`: that one is an accumulator
  travelling up the tree, this one is a single widget's answer. Same shape, different
  meaning.

  One golden moved: the two-pane content sits 2 px higher, `height: 100%` having claimed
  the whole column's height where a fill takes what is left.

### Fixed

- **A widget that fills the width now does it in a column too** (J404). Found by milestone
  403's scaling probe and nothing to do with scaling: a `ListTile` **alone** laid out
  correctly and **in a column** — the most ordinary thing anybody does with one — came out
  as wide as its own padding, with its title ellipsised away.

  The cause is `width: Dimension::Percent(1.0)`. A percentage resolves against the parent's
  **resolved** width, and a parent that shrink-wraps has not got one yet — it is waiting on
  this very child. Both readings are "full width" in English and only one can be computed
  in time: the parent's own width is known on the way back *up*, the room being offered on
  the way *down*. `Widget::main_axis_fill` is the second, answered by the walk.

  **It was not a `ListTile` bug**: fifteen widgets declared it and a probe found every one
  of the seven it could build collapsing. Nine impls converted — `ListTile`, `BottomAppBar`,
  `SheetPanel`, `BottomSheet`, `Drawer`, `Steps`, `BarChart`, `LineChart`, `Bullet`,
  `ErrorSummary`. `AspectRatio` and `ConstrainedBox`'s overflow case keep theirs, needing a
  width taffy *knows*; that limit is now documented rather than latent.

  No test caught it because a percentage against a *definite* parent is correct, and every
  fixture gives its widgets a width. Alone is where the bug hid.

  **Five goldens moved**, each read before being accepted: a steps bar and an error banner
  now span the content box instead of stopping at the widest sibling — which is what the
  reference gives, its `Column` handing children `maxWidth` as a loose constraint.

### Added

- **The reader's font size is obeyed** (J403) — the largest accessibility gap the framework
  had. A phone's *Font size* slider goes to 1.3 on Android and past 3 with iOS's larger
  accessibility sizes. Milestone 399 carried the number and said plainly it was not being
  spent; milestone 402 built the one place it could be spent safely.

  The hazard was never the multiplication — it was that 69 call sites read a size and the
  renderer read another, so a scale applied in 68 of them draws text the layout never
  measured. `TextStyle::resolved()` is now the only point where a size becomes a number, so
  that is where the scale goes and the measurement and the paint agree **by construction
  rather than by vigilance**.

  `frus_core::with_text_scale` is **ambient, not threaded**, deliberately: passing a scaler
  down would mean every widget remembering to apply it, and there is no diagnostic for the
  one that forgets. `MediaQuery::scope` installs it — the reader's font size travels with
  the description because it is part of the description.

  Not scaled: a scale of zero or less (disbelieved, not obeyed), weight and slant, the
  debug overlays, and anything outside a described surface — which is why all 91 goldens
  are unchanged.

  **Known limit, measured rather than assumed**: component *widths* follow the type, and
  fixed *heights* do not. `BUTTON_HEIGHT` is 40 and `CHIP_HEIGHT` is 32 whatever the reader
  asked for, so at 2.0 a chip's glyphs need 34 px in a 32 px box.

### Removed

- **`ResolvedTextStyle::to_style`** (J403). Unused, and with a scale inside `resolved()` it
  became a trap: a resolved size turned back into a style and resolved again is a size
  scaled twice.

### Changed

- **Every field of `TextStyle` is an `Option`** (J402) — BREAKING, and the point of it is
  what it makes *sayable*. `TextStyle::new(20.0)` used to name a size **and** a weight
  **and** a slant, because the type had no way to withhold them; *size 20, inherit the
  weight* could not be written however anybody wrote it. The reference writes it
  `TextStyle(fontSize: 20)` and always could.

  Three workarounds collapsed into one operation: a private `Overrides` struct for
  rich-text spans, milestone 400's `Chosen` record of eight booleans beside a `Text`'s
  style, and half of `DefaultTextStyle`. `TextStyle::merge` is field-by-field now instead
  of replacing the typography wholesale, which is why all three existed.

  Two behaviour changes fell out, both in the right direction. `TextSpan::style(s)` no
  longer forces every field to *answered*. And a merge no longer erases a decoration
  nobody replaced — merging a plain style over an underlined one used to silently remove
  the underline.

  The whole routine suite passed on the first run after the conversion and all 91 goldens
  are unchanged: `TextStyle::new(20.0)` still resolves to the same three numbers, it just
  no longer claims all three.

- **`ResolvedTextStyle`** (J402) is where the chain stops — concrete `size`, `weight`,
  `italic` and `decoration`, and the type `Scene::text_styled`/`text_wrapped`/`text_block`
  take now. A shaper needs a number; the type system enforces what a convention used to.
  The colour stays optional even here, its last word belonging to a theme `frus-core`
  cannot see.

- **`DefaultTextStyle` holds a `TextStyle`** (J402) plus the four questions about the box
  rather than the type (`align`, `soft_wrap`, `overflow`, `max_lines`) — the reference's
  shape exactly, now that the style can carry its own "unset".

### Added

- **`frus_text::measure_style(text, style)`** and **`measure_resolved`** (J402), replacing
  the three-argument `measure_styled(text, size, weight, italic)` at most call sites.
  Three arguments is three chances to pass a size from one style and a weight from
  another, which draws text the layout never measured.

- **A `Semantics` widget** (J401) — states a role, a name or a state for a child that
  cannot state it for itself, the reference's widget of that name. Every widget in the
  crate answers for *itself*; this is for the case a caller is handed a built widget and
  knows something about it that the widget does not.

  It **adds** a node by default and leaves the child's alone; `merging` drops the subtree's
  annotations and speaks for all of them, carrying over what they said. That default is
  *not* the reference's, deliberately: the destructive behaviour cannot be undone by the
  caller, so a merging default would collapse a whole screen into one node the first time
  somebody wrapped one to name it. `Semantics::heading(child)` and `Semantics::merge(child)`
  are the two common cases spelled out, the second being the reference's `MergeSemantics`.

  Merging joins two labels one line each, as the reference does — picking one would drop
  the other with nothing to say which. Note that this framework's accessibility tree is
  **flat**, so merging really discards where the reference keeps the subtree's shape.

- **`SemanticsProperties::over`** (J401), the field-by-field merge behind it.

### Changed

- **`frus_core::Semantics` is renamed `SemanticsProperties`** (J401) — BREAKING. The
  reference's `Semantics` is the *widget* and `SemanticsProperties` the data bag; ours had
  the data holding the name the widget needed.

- **The `AppBar`'s title is a widget** (J401) — BREAKING. `AppBar::title_widget` is renamed
  `AppBar::title`, and the internal `Title` enum is gone. The reference's `title` is a
  `Widget?` with no string form (`app_bar.dart:1067`); `AppBar::new("Inbox")` is now a
  convenience that builds a plain `Text` with **no style on it** and hands it to the same
  path.

  Three consequences. **Every title is a heading**, not only a string one — the milestone
  397 asymmetry, where a bar's accessibility depended on which constructor was used, is
  closed. **The type is handed down rather than applied**, so a `Text` inside a caller's
  widget picks up the bar's `title_large` while one that chose a size keeps it. And the
  **manual truncation is gone**: `soft_wrap: false` and an ellipsis come down with the
  style, and the words are cut by the box they are actually given instead of by a width
  computed before the layout ran.

- **`Widget::describes`** (J401) is a new hook with a default, so implementations need no
  change unless they want it.

### Added

- **A text style a subtree hands down** (J400). `DefaultTextStyle` — the reference's
  inherited widget, a `WidgetThemes` entry here — lets an app bar, a dialog or any
  subtree dress every run of words inside it, including the ones it never sees because a
  caller handed them over already assembled. It resolves **field by field**: setting a
  colour leaves the sizes alone.

  The rule is `what the caller said ?? what the subtree handed down ?? what the framework
  ships`, and **a default the caller never picked does not count as having said
  something**: `Text::new("x")` is 16 px because nobody chose a size, so a subtree asking
  for 20 gets 20; `Text::new("x").size(16.0)` chose one and keeps it. The reference gets
  that distinction free from nullable style fields; ours is a `Chosen` record carried
  beside the style, because an `f32` cannot say whether anybody picked it.

  It reaches the **layout**, not only the paint. An inherited size resolved at paint draws
  24 px glyphs in a box measured for 16 — every row on the screen the wrong height at once,
  with nothing in the picture to say which of the two numbers was the mistake.

  `DefaultTextStyle::around(child)` wraps a subtree with it, the reference's widget of that
  name.

- **`AppBar::toolbar_text_style`** (J400), the reference's `toolbarTextStyle`: the type worn
  by the words in the bar that are **not** the title. Recorded as blocked in milestone 396
  and now unblocked by the above. It does not touch the title, which has its own
  `title_style` — a bar that resized the title would be resizing the one line whose width
  decides how many actions still fit inline.

- **Nine widget theme structs are now reachable** (J400) — `ButtonTheme`, `CheckboxTheme`,
  `ChipTheme`, `IconButtonTheme`, `IconTheme`, `RadioTheme`, `SegmentedTheme`,
  `SliderTheme`, `SwitchTheme`, plus the new `DefaultTextStyle`. They were `pub` inside a
  **private** module and re-exported nowhere: public and unnameable. `AppBar::icon_theme`
  had therefore taken a type no caller could build since milestone 396 — a property that
  shipped and could not be used.

### Changed

- **`Widget::measure`, `measure_key`, `main_axis_floor` and `main_axis_fill` take a
  `&Theme`** (J400) — BREAKING for anyone implementing `Widget` by hand. The four hooks
  decide a *size*, and a size that ignores the theme answers for a font nobody is drawing.
  `main_axis_fill` is the one that would have hurt most: alignment is also a request for the
  parent's width, so a handed-down `align` would have resolved correctly everywhere except
  where it takes effect.
  Every call site already had a theme in scope, so implementations only need the extra
  parameter. `measure_key` now hashes the resolved style: a cache key that ignored half its
  inputs is a stale layout waiting for a theme to change.

### Added

- **The settings a screen reports about its user** (J399). `MediaQuery` gains
  `text_scaler`, `platform_brightness`, `system_gesture_insets` and an `Accessibility`
  struct — `bold_text`, `high_contrast`, `disable_animations`, `invert_colors`,
  `accessible_navigation`, `always_use_24_hour_format` — one struct rather than six fields
  because they arrive from one platform query and are read together.

  **`disable_animations` is honoured by the framework**, not merely reported. The implicit
  animations — the ones a widget starts by itself when a value it was given changes —
  complete at once instead of over time: the change still happens, it stops moving.
  Skipping the change instead would leave the interface *wrong* rather than *still*, which
  is the failure this setting is most often given, and the test checks both halves —
  nothing left animating **and** the value actually arrived. It does not touch scrolling: a
  fling is physics answering a finger, not a decoration, and the reference draws the same
  line. `Runtime::still` is set from `Application::accessibility` every frame, so a
  *reduce motion* switch changes it while the application runs.

  Which fields have a consumer is stated rather than implied: `disable_animations` is
  obeyed, the rest are reported for the application to act on. A field that looks obeyed
  and is not is exactly what milestone 397 found behind a green tick.

  **`text_scaler` carries the number and does not yet spend it**, deliberately.
  `MediaQuery::scaled(size)` is a *function* for the reason the reference made `TextScaler`
  one — a platform may scale non-linearly so a heading does not run off the screen when body
  text is made readable. Making the framework honour it needs measurement and paint to agree
  (69 direct `frus_text::measure*` call sites on one side, `Primitive::Text { size }` on the
  other); if they ever disagree the result is text measured at one size and drawn at
  another, which is why it is its own milestone rather than a paragraph in this one.

### Fixed

- **A `Navigator`'s pages went past the edge of the thing holding them** (J398). A navigator
  slides one screen out while another slides in, so both start or end **outside** its box —
  that is what sliding is — and nothing stopped them there. The only bound was whatever clip
  they had inherited, which for a full-window navigator is the window: so it looked fine,
  and a navigator that was **not** the whole window painted its pages straight over whatever
  sat beside it, while even a full-window one spent every transition frame drawing a screen
  nobody could see. The reference clips by default (`Clip.hardEdge`,
  `navigator.dart:1601`). `Widget::navigator_clips` is `true` by default and consulted only
  inside the navigator branch of the walk, so it costs nothing for every other widget;
  `Navigator::clip_behavior(false)` is the way out for a transition genuinely meant to
  spill.

  **How it was found**: not by a test, but by a comment milestone 393 had to write in the
  demo's safe-area check to explain why it was ignoring text painted at x = 452 in a
  400-wide window. Writing down *why* a test has to ignore something is how the something
  gets noticed.

  The framework's own guard caught the follow-up:
  `transparent::the_macro_forwards_every_hook_the_trait_declares` failed with
  `a transparent wrapper would answer these for itself: ["navigator_clips"]` — a `Keyed`
  around a navigator would have answered for itself, `true` by luck. It earned its keep.

- **The app bar's title was not a heading, and the roadmap said it was** (J397). A screen
  reader's user moves through a screen by its headings; the one every screen has is its
  bar's title, and ours carried `Role::Label` — one more piece of text with nothing to jump
  to. Two failures were stacked: **nothing in the framework emitted `Role::Heading` at all**,
  and the one place that handled it threw it away —
  `Role::Heading => AkRole::Label, // no distinct Heading role is used here`. That comment
  was true of an older AccessKit and false of the one we depend on, and nothing had gone
  back to look. Either failure alone is invisible; together they read as a working feature,
  which is how the roadmap came to have it marked done. The mapping names `AkRole::Heading`
  now, `Text::heading()` marks a text as a landmark rather than prose (it changes nothing
  that is drawn), and the bar's title is one — as the reference says with
  `Semantics(header: true)` (`app_bar.dart:1079`). The rest of the role mapping was checked
  at the same time: no other arm was flattening anything.

### Added

- **`AppBar::exclude_header_semantics`** (J397). The reference's switch, `false` by default.
  For a bar whose title is decorative, or one of two on a page where only the outer one
  names the screen — announcing both as headings gives the user two landmarks where there is
  one screen. Only a **text** title becomes a heading: a widget title keeps whatever
  semantics it brought, this framework having no `Semantics` wrapper to put a role on
  something already assembled. That gap is on the roadmap rather than half-answered here.

- **`Icon` takes its colour and its size from the theme** (J396). `WidgetThemes::icon` —
  the reference's `IconTheme`, which is an *inherited* widget there for the reason this is
  scoped here. `Icon` read `theme.on_surface` and nothing else, so recolouring the glyphs in
  a subtree meant changing the foreground colour and recolouring the words beside them. Both
  resolve as everything else does, `caller ?? theme ?? the framework`, and the **size** goes
  through `style_themed` rather than only `paint`, so a bar that makes its glyphs smaller
  has them take less room instead of the same room with a smaller drawing in it.

- **`AppBar::flexible_space`, `icon_theme` and `actions_icon_theme`** (J396).
  `flexible_space` is a widget stacked **behind** the toolbar, filling the bar's box — an
  image or a gradient the title sits on, and what makes a collapsing header possible at all.
  A layer, not a slot: a 400 px child on a 64 px bar does not move the title by half a
  pixel. `icon_theme` reaches every glyph in the bar, delivered as a theme for the subtree
  (`Themed::tweak`) rather than as an argument — the only thing that reaches an icon nested
  inside a button the bar was handed already assembled, which is exactly why the reference's
  `IconTheme` is inherited. `actions_icon_theme` lets the actions part company with the
  leading slot: a back arrow in the foreground colour beside actions in a muted one.

- **Material 3's surface tint, in the core** (J395). `Color::surface_tint` and
  `surface_tint_opacity`. Material 3 does not show height with a shadow but by moving the
  surface towards a tint colour in proportion to its elevation — which matters most exactly
  where a shadow fails, on a dark background. The strength is the specification's table
  (0, 0.05, 0.08, 0.11, 0.12, 0.14 at elevations 0, 1, 3, 6, 8, 12), interpolated between
  levels and clamped outside them, so a bar at 40 is tinted exactly as much as one at 12.
  In `frus-core` rather than in the app bar because `Card`, `Drawer` and `BottomAppBar`
  all want it. The blend is a plain channel mix and that is correct **here**: the result is
  an opaque colour painted directly, so nothing is composited and there is no linear-space
  step to get wrong — laying the tint on as a translucent layer would go through
  compositing and come out darker.

- **The `AppBar`'s surface** (J395). **`shape`** (which clips as well as rounds — a
  surface stopping short of its own corner would square off the one the shadow curved),
  **`shadow_color`** (ours was a constant near-black), **`surface_tint`**,
  **`force_material_transparency`** (no background, no tint, no shadow, for a bar over an
  image — it *overrides* rather than argues, a caller asking for transparency having
  already decided), **`toolbar_opacity`** and **`bottom_opacity`** (independent group
  opacities: the contents fade, the surface stays), and **`actions_padding`** (an icon
  button's hit area already reaches the bar's edge, so a design wanting the *glyphs* inset
  had nowhere to say so). `shadow_color`, `surface_tint` and `shape` are on `AppBarTheme`
  too.

- **Four `Scaffold` properties the reference has and we did not** (J394).
  **`Scaffold::primary`** (default `true`) decides whether the app bar's height is its own
  or its own **plus the status bar** — the reference computes `primary ?
  MediaQuery.paddingOf(context).top : 0.0` and adds it to the bar's preferred height
  (`scaffold.dart:3049`). Ours added the intrusion unconditionally, which insets twice for
  a shell nested in a page or sitting beside another. **`persistent_footer_divider`**
  (default `true`) draws the line along the footer's top that the reference draws by
  default (`Divider.createBorderSide`, `scaffold.dart:3136`) and ours never did, as a
  `Divider` so its colour and thickness follow the theme; **`persistent_footer_color`** is
  the other half of that decoration. **`drawer_scrim_color`** reaches a `Drawer` setting the
  shell had no way to pass through, and **`drawer_barrier_dismissible`** (default `true`)
  is for a panel holding something that has to be answered — the way out is a control
  inside it, and the screen behind stays unreachable.

  The footer was **restructured**, not merely decorated: the reference puts the decoration
  on the outer container and the `SafeArea` inside it, so the rule and the background run
  the full width and the *content* keeps clear of the side intrusions. Ours inset both, and
  a border inset by the notch is a rule that stops short of the edge it rules off.

### Changed

- **Nobody hands the framework the screen** (J393). **Breaking.** `Application::view` takes
  no size, and neither do `Scaffold`, `AppBar` and `Navigator`:

  ```rust
  fn view(&self, theme: &Theme) -> Box<dyn Widget<Msg>>;   // was (theme, width, height)
  Scaffold::new()                                          // was ::new(width, height)
  AppBar::new(title)                                       // was ::new(title).width(width)
  Navigator::new(screen)                                   // was ::new(screen, width, height)
  ```

  Checked against the reference rather than remembered: its `Scaffold` (`scaffold.dart:1688`)
  and its `Navigator` (`navigator.dart:1587`) take **no size parameter at all**, and its
  `AppBar` declares a height and an *infinite* width (`_PreferredAppBarSize`, built with
  `Size.fromHeight`, `app_bar.dart:75`) — infinite meaning *fill what you are given*. Size
  travels as constraints, never as arguments.

  The framework already installed a description of the surface around every call to `view`;
  these three read it now. `Scaffold::new()` takes the size **and the three insets**, so an
  application no longer measures the window, subtracts the notch and the bars, builds at the
  remainder and wraps the result in a padded background — twelve lines of the demo's `view`
  became one, and `width`/`height` came off ten screen functions and the router feeding them.
  Each keeps an override — `Scaffold::size`, `AppBar::width`, `Navigator::size` — for what is
  genuinely not the whole screen, and for a test that would rather state a size than install
  a description.

  Milestone 392 is why this matters and not merely why it is tidier: a number carried by
  hand gets arithmetic done on it, and one of those subtractions is eventually wrong.

- **`SafeArea` fills what it is given** (J393). `flex_grow: 1.0`. The reference's is a
  `Padding` under the screen's own tight constraints — it *is* the box it was handed — while
  a flex node that grows nothing hugs its content, so a screen wrapped in one came out the
  width of its widest line: milestone 392's failure, one level up.

### Added

- **`MediaQuery::is_described`** (J393). Whether a surface is actually described, or
  `MediaQuery::of()` is answering `UNSET` outside every scope. A widget that sizes itself
  from the ambient description needs the distinction: a width of zero is not a narrow
  screen, it is the absence of one, and a bar folds everything at zero and nothing at all
  when there is no screen to fold against.

### Fixed

- **One child was deciding how wide the screen was** (J392). An application came back with
  a screenshot of a shop whose banner, search field, filter row and product grid were all
  cut off at the same edge — a page laid out to the wrong width, not a widget overflowing.
  `Layout::allow_shrink` promised, in its own doc comment, that a lone child is bounded by
  the box it was given; what it did was set `flex_shrink = 1.0` and stop. That buys nothing
  on its own, because **a flex item's automatic minimum is its min-content size**: the item
  agrees to shrink and then refuses to go below the widest thing inside it. So one
  over-wide row made its column that wide, the padded box around it followed, and
  `Align::Stretch` stretched every sibling to match. It zeroes the automatic minimum now —
  the `min-width: 0` idiom — and only that one: a box that names a floor, or a tight one,
  keeps it. Width only; a height is what scrolling is for, and bounding it shortened a
  two-pane golden until the list's background no longer covered its content. The over-wide
  child still overflows, which is the reference's behaviour and the honest one, but it no
  longer drags its parent and its siblings out with it.

- **Every demo screen drew eight pixels past itself, and the guard was green** (J392). The
  overflow guard has run on every screen since milestone 335; it was **asserting the bug**,
  because each parent grew to fit and nothing ever measured as overflowing. Three places
  computed a content width by hand — `(width - 88.0)`, the body's padding plus the card's —
  and none counted the card's own **margin**, which the caller never set. They no longer
  subtract anything: the content fills its card, and the only number left is a `max_width`
  a designer would give. The field takes the room the button leaves (`Expanded`), the
  progress bar and the horizontal showcase stretch, and the card fills the body on a phone
  and is capped and centred on a wide window.

- **The Settings overflow pin came down** (J392). A **known** 4.5 px, measured in milestone
  335 and left on the roadmap with a cause milestone 345 disproved. Under the corrected
  rule it measured 9, and the diagnosis was one look: a slider asking for a fixed 220
  beside a label measuring 108, in a card of 331. It asks loosely now — `Expanded::loose`,
  the reference's `Flexible` — so 220 is what it would like and the room left is what it
  takes. Every screen, at a phone's width and at a desktop's, draws inside itself.

- **A strip of nothing above the keyboard** (J391). The reference describes a screen's
  intrusions with **three** numbers; we had two, and derived them wrongly. `padding` is
  `view_padding` less `view_insets`, floored per side — what is **left** to avoid. Ours
  reported the navigation bar's height at the bottom even while the keyboard covered it,
  so a `SafeArea` reserved room for a bar nobody could see.

  `view_padding` — the intrusion that does **not** move when the keyboard opens — was not
  merely missing: the shell was already computing it and throwing it away, since the
  inset baseline it keeps per physical size to tell a keyboard from a bar *is* the
  keyboard-free intrusion. `from_baseline` returns all three now.

  It matters for the case `padding` is wrong for. A screen with a flexible child grows by
  exactly the bar's height the moment a field is tapped and shrinks again when the
  keyboard closes — a whole layout twitching because somebody started typing.
  `SafeArea::maintain_bottom_view_padding` pads by the intrusion that does not move, and
  a `Scaffold` that **declines** to resize now reads `view_padding` too: declining says
  the keyboard is an overlay, so the geometry it wants is the keyboard-free one, and
  reading `padding` there would drop the bottom bar and the floating button onto the
  window's edge the moment a field was tapped.

  `remove_padding` and `remove_view_insets` both take the same amount off `view_padding`,
  as the reference's do: a descendant asking for the intrusion that does not move would
  otherwise be told about one its parent had already dealt with.

  `WindowInsets::bars` exists because five tests hand-built `{ padding, view_insets }`
  literals, and under the corrected rule `padding.bottom = 16` beside
  `view_insets.bottom = 320` is a state no platform can report — a test asserting against
  it asserts against nothing. The keyboard cases go through `from_baseline` now, the same
  function the shell uses. That is how the wrong `padding` survived: the test that should
  have caught it was asserting the bug.

### Added

- **Where the tabs sit in a bar with room to spare** (J390). Two tabs in a window meant
  for six were two tabs three hundred pixels wide with their labels marooned in the
  middle of a lot of nothing, and that was the only answer the bar had.
  `TabAlignment::{Auto, Start, StartOffset, Fill, Center}`, on the widget or on
  `TabBarTheme`.

  `Auto` is the reference's `isScrollable ? start : fill` said out loud, so the default is
  expressible rather than merely being what happens when nobody speaks.

  **The reference throws; this does not.** `fill` on a scrollable bar and `start` on one
  that is not are assertion failures there, and a layout that throws is a crash on
  somebody's phone for a combination with an obvious reading. Fill on a scrolling bar
  reads as `Start`, sharing a width being the opposite of exceeding it; and `Start` on a
  bar that shares its width does exactly what it says — natural-width tabs packed at the
  leading edge, which the reference forbids for reasons that are its own history.

  **`Center` works on a bar that does not scroll and reads as `Start` on one that does,
  and that was measured rather than assumed.** A centred strip takes the bar's whole width
  so there is something to centre in; inside a horizontal scroll there is no such thing,
  since the strip is measured by its own tabs — that being what gives the scroll something
  to scroll through — so `Percent(1.0)` resolves against an indefinite parent and the strip
  comes out exactly as wide as the tabs, with nothing left over. Milestone 368's and 377's
  trap, paid for a third time. The test asserts the two alignments give **identical**
  output on a scrolling bar and that the same bar without the scroll does centre, so the
  boundary is pinned rather than described.

  The alignment is resolved **against the theme at layout time**, not once at build time:
  the first version asked `Theme::default()` in `rebuild_bar`, which would have read an
  application's `TabBarTheme::alignment` for the strip's box and ignored it for the tabs'
  widths — a row laid out under one rule with an indicator drawn under another. A test
  asserts the indicator moved by exactly as much as the label it marks.

### Added

- **What a field lets through, and what the keyboard offers** (J389). A quantity field
  took letters, a code field took lower case, and a field that refused everything but
  digits still opened a full QWERTY keyboard, because nothing connected the two.
  `input_filter`, `digits_only`, `capitalization`.

  `input_filter` is `Fn(char) -> Option<char>`: `None` drops the character, `Some(c)` puts
  `c` in instead of what was typed. **One character at a time, and that is the decision.**
  The reference's `inputFormatters` reshape the whole value after every keystroke, which
  buys grouping — spaces every four digits of a card number — and costs a caret, since
  the formatter has to say where the cursor went and getting it wrong puts the cursor in
  the middle of a group the reader never touched. A filter working one character at a time
  cannot group and cannot lose the caret either: a dropped character never arrives and a
  substituted one takes exactly the place of the one typed. That covers digits-only,
  letters-only, no-spaces and forced case, which is nearly every field that filters at all.

  A **refused keystroke is not an edit**. The insertion arm used to mark the value changed
  unconditionally, which with a filter would emit `on_input` with the value already there
  and rebuild the tree for a key that did nothing. A selection counts even when what
  replaces it is empty — the reference filters the value *after* the replacement, so the
  selection really is gone — and a bare refused keystroke does not. The caller's own value
  is never filtered: it is the application's state, the same rule `max_length` follows.

  `digits_only()` also names `KeyboardType::Number`, but only when no keyboard was named,
  so the two builders can be written in either order.

  `Capitalization::{Auto, None, Sentences, Words, Characters}` **replaces** the keyboard
  type's own capitalisation rather than adding to it — two capitalisation bits at once is
  a keyboard being told two things — and `Auto`, the default, leaves it alone, so every
  existing type answers exactly what it always did. Only a text class capitalises: `0x1000`
  is *signed* on a number class, so asking a phone field for capitals would quietly turn on
  a minus key. The composition moved to `Ime::android_input_type`, being a question about
  the pair; the checked-in dex still takes two integers and needs no rebuild.

### Added

- **A tab that can show as well as say** (J388). `TabBar` took a string per tab and drew
  it; a navigation bar wants an icon over the word and a compact one wants the icon alone,
  and neither was reachable. Three builders now: `tab`, `icon_tab`, `icon_only_tab`.

  **One tall tab makes the whole row tall.** The reference has two heights — 46 for an
  ordinary tab, 72 for one stacking an icon over a label — and they are heights of the
  *row*: tabs of two heights in one bar would put their labels on two different lines,
  with the indicator ruling under whichever happened to be lower. The question is asked
  once, of the whole row, and recorded on `TabStyle`, which the bar and every tab already
  share precisely so a measurement cannot be answered two ways. An icon **on its own**
  does not raise the row, which is the reference's rule and what a compact bar wants.

  **An icon-only tab still has to have a name.** `icon_only_tab` takes a label it does not
  draw. The reference leaves such a tab nameless and expects the caller to wrap it in
  something that names it, which is a hole with a lid: it works when somebody remembers.
  Asking for the word where the tab is declared costs a parameter and cannot be forgotten.

  `content_width` is one function again: a tab's text, its icon, or whichever is **wider**
  when it has both, since the two are stacked rather than side by side. The tab's own
  width comes from it and so does the primary indicator's — the file already carried the
  warning that a tab measured one way and an indicator measured another agree on every
  label until they do not, and the failure is an underline creeping away from its tab. The
  two are painted as **one block**, centred once, because two separately centred pieces
  drift apart as the text's height changes and the labels of neighbouring tabs would stop
  sharing a line.

### Fixed

- **A scrolling tab bar never moved to the selected tab** (J387). Eight tabs on a phone
  stop being eight crushed columns when the bar scrolls, which milestone 345 made
  possible — but selecting the ninth tab from anywhere other than a tap on the tab
  itself left the bar showing the first three. The panel below changed; the bar did not,
  and nothing said where the selection had gone.

  `Widget::keep_visible` is how a widget asks the scroll region around it to keep a box
  in view, and it returns a **key** with the box. The key is the whole mechanism: the box
  moves as the region scrolls, so a region that chased the box itself would pin the
  content in place and no finger could move it. `Runtime::sync_visible` acts when the key
  changes, the same shape `sync_pages` already had, and a test pins it — the reader
  drags the bar elsewhere, the selection has not changed, nothing pulls it back.

  The box reaches the region **as it would sit at offset zero**, and that was a bug
  before it was a decision. Recorded where it is drawn, its position already contains the
  current offset, so the offset that would centre it can only be worked out relative to
  wherever the region is — and on the first frame there is no wherever: the walk had
  already placed the strip on the selected tab, the box came back centred, the arithmetic
  answered "nothing to do", and the offset was never retained, so the frame after put the
  bar back at the start. At rest, the answer is the same number on the first frame and
  the hundredth.

### Added

- **`Scrollable::centre`, beside `reveal`** (J387). Two policies, and the widget that asks
  says which it means. A tab bar centres — the reference's behaviour, and the readable
  one, since a selected tab flush against the window's edge reads as the end of the row
  when it is only the edge of the window — while keyboard traversal keeps the
  least-movement rule, because a form that re-centred on every Tab would be motion nobody
  asked for. The clamp keeps both honest: the first tab cannot be centred without
  scrolling backwards, so it stays at the start.

  The request rides **on** `Scrollable` rather than in a registry of its own, which would
  have needed a slot in `BoundaryData`, in `Snapshot` and at each capture and replay site,
  and a repaint boundary caching a tab bar would have dropped it on every hit.

### Fixed

- **Tab could not reach what was scrolled out of sight** (J386). A form taller than its
  window had most of its fields **unreachable from the keyboard** — not hard to reach,
  unreachable: the walk registered a focus stop inside the same guard as the click
  targets, `visible.width > 0.0 && visible.height > 0.0`, so Tab walked the two and a
  half fields on screen and wrapped round to the first.

  That guard is right for a **click** — a tap on empty space must not land on something
  scrolled away under it — and wrong for focus, which the reference keeps whether or not
  any pixels of it are showing. The focus registration moved out of it, and only the
  focus registration.

  Not *always*, though. A widget clipped away by something that does **not** scroll
  cannot be brought into view by anything, so focus would land where the eye can never
  follow. The condition is about rescue rather than visibility: a stop counts when it is
  visible, or when a scroll region encloses it and could reveal it. `scroll_host` is
  threaded through the walk the way `refresh_host` already was.

  `Focusable` carries **two** boxes now, because they are two questions: `rect`, clipped,
  which the click test uses; and `bounds`, unclipped, which is where the widget actually
  is. Keeping only the clipped one puts an off-screen stop in the top-left corner as far
  as arrow navigation is concerned; keeping only the unclipped one lets a click on empty
  space focus whatever is scrolled underneath.

### Added

- **A view that follows the focus** (J386). `Scrollable::reveal` gives the offset that
  brings a box into the viewport and how far the content moves to get there — the
  **least** movement that does it, since centring would make Tab through a form scroll on
  every stop, including the ones already in the middle of the window. A target too big for
  the viewport gets its leading edge, which falls out of asking about the near edge first
  rather than being a case of its own. The reversed axis is not a second branch:
  `offset_delta` already turns a movement of the content into a movement of the offset and
  is its own inverse, so the same function turns the offset that was allowed back into the
  movement it bought.

  Regions nest — a sideways strip inside a page that scrolls down — and identities here
  are hashes that carry no ancestry, so each region records its own `host` during the walk
  and `Ui::reveal` follows the chain outwards, telling each one where the target **ended
  up** rather than where it started. The shell calls it only where the keyboard moves the
  focus: a click already landed on something the reader could see, and a focus restored
  because an overlay closed should put the page back where it was left. It sets a scroll
  **target**, so the view glides rather than cuts.

### Fixed

- **A carousel could not reach its last page** (J385). Set `viewport_fraction` below one
  and the neighbouring pages show at the edges — but ours opened with page 0 flush
  against the leading edge, all the slack on the other side, and the reference centres it.
  Centring is not decoration here. Fifty pages at 0.8 of a 300 px viewport are 240 px each,
  so page 49 rests at 11 760; unpadded, the content is 12 000 and the travel stops at
  11 700. **Sixty pixels short of where the last page rests**: the snap would aim at it,
  the edge would refuse, and the spring would pull it back — every time, for ever, on the
  last page of every carousel.

  With `(viewport - extent) / 2` at each end the travel works out to `extent × (count - 1)`
  exactly, which *is* the last page's resting offset. The test asserts both sides:
  padded, `offset_of(49) == max_x`; unpadded, `offset_of(49) > max_x`. `pad_ends(false)`
  keeps the flush version for a carousel that wants it, and at the default fraction of one
  the padding is nil, so nothing that existed before this moves.

### Added

- **A page view that runs the other way, and one that need not snap** (J385).
  `PageView::reverse` puts page 0 at the end the axis finishes at, the same question
  `ListView::reverse` answers: not a mirrored picture, but which end an *index* is. The
  window arithmetic is untouched — a reversed view counts its indices from the far end
  and a reversed offset counts its pixels from the same one, so the two agree about which
  way forward is, and only where a page lands differs. The region carries `reverse_x` /
  `reverse_y` now instead of two hardcoded `false`s, so the drag, the overscroll glow and
  the scroll-to-offset all read the axis the same way round.

  `page_snapping(false)` is a scrollable that happens to know where its pages are. The
  easy version drops the `PageSnap` from the region, and that would take the pages with
  it: `on_page_changed` goes quiet and `page(3)` stops working, because both read the snap
  to know what a page even is. So the flag rides **on** the snap and exactly one place
  reads it, the release. A test pins the difference there and nowhere else — the same
  view left a fifth of a page along with no speed behind it stays put when snapping is
  off and springs back to page 0 when it is on — and a second walks the rest and finds
  it unchanged.

### Fixed

- **A `LayoutBuilder` is as big as what it built** (J355). The reference sizes it to its
  child — `size = constraints.constrain(child.size)` — while ours was a layout leaf whose
  size came from its style, so one dropped into a column with nothing set laid out 0 px
  tall and its own documentation told you to go and find a height for it. It is a
  **measured** leaf now: the layout engine calls back during the computation with the
  space actually available, which is the same moment the reference runs its layout
  callback, and the closure builds the subtree, lays it out and returns its size. An axis
  the style pins is still the style's, so every existing use behaves identically and all
  91 goldens are unchanged.

  It cost a lifetime on `Layout` (a measurement that calls back into the widget layer has
  to borrow the tree, and `MeasureFn` was `'static`) and a documented hole in the relayout
  cache: a closure has no fingerprint, so a root holding one is recomputed every frame
  rather than risk a stale box. It also unblocks the responsive grid delegate left open by
  J353.

- **An Android application opens in its own colours** (J352). Android paints the window
  from the moment it opens it, long before the first frame exists, and the platform theme's
  background is white on a device in light mode — so a dark application opened with a
  full-screen white flash. The reference ships a `LaunchTheme` in its template for exactly
  this; frus ships one now, in the demo, the three examples and the `cargo generate`
  template, with `getting-started` explaining which colour to put in it.

- **A list hands its children a box** (J351). The reference gives a list's children a
  tight cross-axis extent, and a fixed-extent list a tight main-axis one too; ours merely
  *constrained* them, so an item that set no width hugged its content — a list of coloured
  rows painted a column of chips down the left instead of rows across the list. Found on a
  device on the journal screen: rows 79 px wide in a 363 px list.

- **A layer is drawn where the scene put it** (J350). A group opacity, a fade, a rotation
  or a non-rectangular clip is rendered flat into its own texture and composited — and
  every one of them was composited after *all* of the frame's content, so a group that
  something had covered came back on top of it. Found on a device: the demo's translucent
  square, belonging to the home screen, painted over the Kanban board that had replaced
  it. Layers are ordered by the batch planner now, like everything else, and the render
  pass interleaves composite draws with content draws. Nested layers inside a group's own
  pre-pass had the same bug and the same fix.

### Changed

- **`IconName` is `Icons`** (J369). Milestone 367 corrected twenty widget names against
  the reference's and stopped at the widgets; this is not a widget, which is why the sweep
  did not reach it — and it is a name typed more often than most of the twenty, since
  every button, chip and tile with a picture on it goes through this enum. The reference
  calls the set `Icons`; ours said `IconName`, and the argument from 367 applies unchanged.

  It leaves a stutter — the module is `icons`, so the full path is `crate::icons::Icons`
  — and that costs nothing, because the module is private and the only path anybody
  writes is `frus_widgets::Icons`. Keeping a name nobody would search for in order to tidy
  a path nobody sees is the wrong trade, and it is the same one milestone 367 made when it
  left the twenty module files alone.

  Checked before the sweep ran: `IconName` appears in no string literal and is a prefix of
  no longer identifier, both of which 367 had to unpick by hand. The historical record
  keeps the old name — milestone notes and the CHANGELOG and ROADMAP entries that used
  it describe what was true when they were written.

### Fixed

- **The fetched-image store never let anything go** (J383). It was a map that only ever
  grew: every distinct URL an application showed stayed in it, **decoded**, for the life
  of the process. A `Ready` entry holds the whole bitmap — a 1000×1000 picture is four
  megabytes — so a feed of five hundred of them is two gigabytes the process never gives
  back, and on a phone that is not slow but dead. Milestone 374 shipped the store and
  recorded the gap in its own notes; this closes it.

  `DEFAULT_IMAGE_CACHE_BYTES` is 100 MB, the reference's figure, and `set_image_cache_budget`
  takes another — `0` keeps nothing nobody is holding, which is a fair answer for a device
  with almost no memory. The sweep runs when a fetch **lands**, the one moment the store
  grows, so the cost is paid once per picture that arrives rather than once per frame per
  picture on screen.

  **Two entries are never dropped**, and both exclusions are the difference between a cache
  and a bug. A `Loading` one is in flight: dropping it cancels nothing — nothing here can
  — it only forgets a request was made, so the next frame starts a second, `images_in_flight`
  falls to zero, and the redraw loop keeping frames coming until the picture arrived stops.
  The picture then lands in a store nobody looks at again. And one whose handle is held
  **elsewhere** is on screen: dropping it is not unsafe, since the `Arc` keeps the pixels
  alive for whoever holds them, but the next `view` finds nothing, asks again, and the image
  flickers through a placeholder on its way back to where it already was.
  `Arc::strong_count == 1` is how that is asked. When everything left is one or the other the
  sweep **stops** rather than spinning: over budget is the right answer there.

  Recency is a monotonic counter, not a timestamp. `frus-core` compiles for the Web, where
  `Instant::now` is not a thing; a counter needs no platform time source, cannot go backwards
  when a clock is corrected, and *least recently used* only ever asks which of two numbers is
  smaller.

### Added

- **A field whose text can sit somewhere else** (J384). `TextField` drew its value from
  the left edge of the content box and had no other option. An amount field wants its
  figures on the right, where the decimal points line up down a column; a code field wants
  them centred. `text_align` is the builder.

  The offset is zero unless the text is **narrower** than the box: a right-aligned line
  longer than its field would otherwise be shoved off the left edge, the one edge whose
  text must stay put, since that is where reading starts and where the horizontal scroll
  brings the caret back to.

  **One function, or the click stops matching the glyphs.** The paint draws the selection,
  the text, the underline and the caret from a single origin; the hit test rebuilds the
  same geometry from scratch. A caret placed from unaligned geometry on centred text
  appears several characters from the tap — the kind of wrongness nobody reports
  precisely, because it does not look like a bug, it looks like the field is broken. There
  is one `align_offset` and both call it, and a test taps a hair before the first glyph and
  a hair after the last, for every alignment.

  It takes **no reading direction**, and that is the decision rather than an oversight. The
  first version resolved `Start` and `End` against the theme — but `cursor_at` is handed
  a rectangle and nothing else, so the paint could apply that push and the click could not,
  which would have put the caret several characters from the tap in **every field of every
  right-to-left application**, including every field that never asked to be aligned. A push
  both sides can compute is worth more than one that is right in a place the other cannot
  reach; following the reading direction belongs to the text layout, where the caret and
  the hit test already live.

  The placeholder moves with it, since a centred field whose hint hugs the left edge jumps
  the moment the first key lands. **Single-line only**: aligning wrapped text means moving
  each line by its own width, and the caret and the click would have to be told about an
  offset that differs line by line — that belongs inside the text layout, not in a widget
  nudging a block sideways behind their backs.

### Added

- **A wrap where the lines are a decision too** (J382). `Wrap` had one spacing: `gap`
  went into both axes, so eight pixels between the chips on a line meant eight between
  the lines. That is the wrong shape often enough to be worth two numbers — a wrap of
  chips usually wants them close side by side and further apart line to line, because the
  eye reads a line as a unit and the vertical break is what tells it where one ends.

  `Flex::run_gap` is the second number, the reference's `runSpacing` to `gap`'s `spacing`.
  Untold, the lines are still spaced by `gap`, so nothing that says nothing changes.

  **Which axis the runs stack on is not a constant.** `Style` already had `row_gap` and
  `column_gap` for the grid, so writing `run_gap` into `row_gap` was the obvious move and
  would have been wrong for every wrapping **column**, whose lines stack sideways — and
  worse than wrong, it would have worked perfectly on every wrapping row anybody happened
  to test. It follows the direction instead.

  **And the lines had no alignment at all.** `align` places each child within its line;
  nothing placed the lines. `Style` had no `align_content`, so a wrapping container with
  cross-axis room to spare always got flexbox's default — the lines stretch to fill it
  — and no caller could say otherwise. `AlignContent` is the new enum and
  `Flex::align_lines` the builder, with `Stretch` still the default so no existing layout
  moves.

  The tests measure the **laid-out rectangles** rather than reading the field back off the
  style: a field set and never reaching the layout engine is the failure worth catching,
  and a test that asks the style what the style says will never catch it.

### Added

- **A drag with a beginning and an end** (J381). A `Slider` sent `on_change` and nothing
  else — on every pixel of the movement, sixty times a second while the finger is down,
  which is correct and is also the only thing the application ever heard. So an
  application that seeks a video, writes a setting to disk, re-renders a preview or asks
  the network did **all of it** sixty times a second, or did none of it: there was no
  other moment to choose, because nothing told anyone the drag had stopped.

  Not a slider problem. `Widget::draggable` is a framework-level contract and it had
  exactly one hook, `on_drag(fraction)`; every value-dragging widget was in the same
  position. `on_drag_start` and `on_drag_end` now bracket it, opened by the shell where
  it begins a widget drag and closed where it takes it back.

  The start goes out **before** the first value — a press on a track jumps the value at
  once, and a start arriving afterwards would hand over a change before saying one was
  coming. The end goes out even when the press never moved: a tap on a track is still a
  change, and a caller deferring its work to the release would wait forever for a release
  nobody mentioned.

  `Slider` gains `on_change_start` and `on_change_end`, `RangeSlider` the same pair in
  `(low, high)`. Both speak in the **caller's units**: a slider set to `range(0.0, 100.0)`
  is owed a hundred at each end as well as in the stream, and a bracket in fractions
  inside a signature that looks symmetrical would be a trap that type-checks. The
  divisions apply at the ends too — a start landing between two stops would name a value
  the stream itself can never produce.

  Not to be confused with `on_dropped`, which belongs to drag-and-**drop**: that one moves
  a thing and reports whether a target took it, this one changes a number and reports
  where it settled.

### Added

- **A keyboard that knows what it is typing into** (J380). The Android input bridge set
  the same `EditorInfo` for every field in every application — sentence-cased prose and
  a *Done* key — because that was the only thing on offer. A phone-number field opened a
  full QWERTY keyboard; an email field had no `@` within reach and had its first letter
  capitalised, which is an address that does not work; every field in a five-field form
  said *Done* where the next thing to do was go to the next field; and a field taking
  several lines had a key that ended the editing instead of adding a line.

  `KeyboardType` says which keys, `TextInputAction` says what the action key does, and
  together they are `Ime`, which the new `Widget::ime` hook hands over when a field takes
  focus — found the same way the caret is, so the two cannot disagree about which field
  is being typed into. `TextField` gains `keyboard_type` and `action`.

  Untold, a field works it out from what it already is: a masked field is a `Password`, a
  multi-line one is `Multiline` with a `Newline` key, and everything else is the text
  keyboard with *Done* — exactly what was hardcoded, so nothing that says nothing
  changes.

  **The numbers are computed in Rust**, not chosen in Java, and the mapping is not behind
  `cfg(android)`: a mapping only exercised on a device is a mapping nobody checks, and the
  bridge's dex is checked in, so a Java file that only copies two integers onto the
  `EditorInfo` never has to be rebuilt for a keyboard type nobody has thought of yet.
  Eight tests run over it on every platform.

### Fixed

- **A password field told the keyboard it was ordinary prose** (J380). `obscure` draws
  dots on our side and says nothing to the IME, so an Android keyboard treated a password
  as sentence-cased text: **learning it into its personal dictionary and offering it back
  as a suggestion later**, on whatever screen came next. The masking was never the part
  that protected anything — `TYPE_TEXT_VARIATION_PASSWORD` is, and an obscured field now
  carries it unless the caller names another type.

### Added

- **A drawer nobody could recolour** (J379). `DrawerTheme` had exactly one field,
  `width`. Everything else the panel painted was written into `paint` and stayed there:
  the fill was the theme's surface, the hairline was `outline_variant` at 1 px, the
  corners were square, and the scrim behind a modal drawer was the `scrim` role at a fixed
  half opacity, painted centrally in the walk where no drawer could reach it. The same
  breach milestone 368 found on `ExpansionTile` and 378 on `Badge` — and the first of
  the three where the hardcoding was hiding a bug.

  It gains `background_color`, `border_color`, `border_width`, `radius`, `elevation` and
  `scrim_color`, each resolved instance → theme → the scheme's role, plus the same six
  on `DrawerTheme`.

  **The shape.** The inner edge — the one facing the content — is rounded by
  `DRAWER_RADIUS`, and the outer one stays square against the window, which is what the
  reference's own shape resolves to. `BorderRadius` could not express it: there was `top`
  and `bottom` for the horizontal edges and nothing for the vertical pair, so `left` and
  `right` complete the set.

  `elevation` casts its shadow **sideways**, along the inner edge. A drawer is as tall as
  the window, and a shadow dropped below it falls outside the window and is never seen.

  `overlay_scrim(&theme)` is the new `Widget` hook behind `scrim_color`. It takes the
  theme rather than nothing, because a drawer's untold scrim is a `DrawerTheme` entry and
  a hook without one could only ever read the instance. The alpha is the caller's — a
  scrim is the one colour whose *transparency* is the point — so an opaque value hides
  the body entirely and `Color::TRANSPARENT` is an overlay that darkens nothing. The
  progress still multiplies it, so the fade stays with the slide.

### Fixed

- **A drawer ruled its hairline down the window's own edge in Arabic** (J379). The panel
  drew on the side it was *asked* for — `border_left` came straight from
  `Drawer::right`, the **logical** side — but a leading drawer is on the left of the
  screen in English and on the **right** in Arabic, since the walk mirrors the whole frame
  and the modal placement flips with it. So in RTL the panel landed against the right edge
  of the window and ruled its line down that same edge, leaving the edge that actually
  faces the content bare. The screen side is worked out in `paint` now, from the direction
  in force; the rounding needed the identical answer, which is why the bug was worth
  finding before the shape went in rather than after.

- **A `Drawer` builder called after `body()` did nothing, silently** (J379). `body()`
  finalised the drawer, wrapping the panel there and then, so anything set afterwards
  changed a field that was never read again — `width` had carried this since it was
  added, and the six builders above would each have inherited it. The two pieces the
  caller hands over sit in `Cell`s now and the tree is assembled once, the first time the
  walk asks for the children. Order stops mattering, which is what every other widget in
  the catalogue already promised.

### Added

- **A badge that is more than one colour** (J378). `Badge` had **one** builder:
  `Badge::new(text)`. No colour, no text colour, no size, no padding, no type — a pill in
  `primary`, a label at a hardcoded 13 px, and no entry in `WidgetThemes`, so neither the
  caller nor the theme could change any of it. The same breach milestone 368 found on
  `ExpansionTile`, hidden in the shortest file in the catalogue.

  It gains `background_color`, `text_color`, `text_style`, `small_size`, `large_size`,
  `padding` and `label_visible`, plus a `BadgeTheme`, resolved instance → theme → the
  scheme's role like everything since `Chip`.

  **A dot is a shape, not an empty label.** The reference draws two badges: a pill carrying
  a number, and a dot without one — and a dot is what most notification marks are, since
  *something happened* is the whole message. Ours could not draw one at all. `Badge::dot()`
  is it, and `label_visible(false)` falls back to it rather than to an empty pill, which is
  what a count of zero wants.

  A one-character badge is never narrower than it is tall: a lone digit in a wide pill
  reads as a mistake, and the reference rounds a single character to a circle for the same
  reason. The floor is a minimum, not a size — `1024` still widens it. And a count is
  announced to a screen reader where a dot is not, since a dot has nothing to say beyond
  the mark itself.

### Changed

- **A badge's untold fill is the scheme's `error`** (J378), where it was `primary`. A badge
  is an **alert**, not an accent: it says *look here*, and painting it the same colour as
  every selected tab and pressed button makes it one more thing in the accent colour. This
  changes the default appearance, and it is a correction rather than a preference — the
  previous colour was not a decision, it was the first role to hand.

### Added

- **A tab bar with more tabs than fit** (J377). `TabBar` shared its width between its
  tabs, always — the reference's rule for a bar that is **not** scrollable, and the code
  said so in a comment, but there was no other kind. Eight tabs on a phone were eight
  columns of forty-odd pixels: not a tab bar with more tabs, a tab bar with none you can
  read. `TabBar::scrollable(true)` is the other kind.

  A tab paints its own label rather than holding a `Text` child, so `width: Auto` would
  give it nothing and its width has to be **stated**. `scrolled_tab_width` is that number,
  and the indicator is placed by the same function — which is the point of there being
  one: a tab measured one way and an indicator placed by another would agree on every
  label until they did not, and the failure would be an underline creeping away from its
  tab as the bar filled up. The test checks the spans against a hand-computed prefix sum
  rather than against `tab_spans`, since a test that asks the implementation what it
  thinks agrees with a mistake.

  A bar that says nothing is untouched: equal columns, no scrollable registered, the same
  pixels. Everything that makes the bar a bar — indicator, ink, tap, disabled state —
  stays in the strip and does not know it is being scrolled.

### Fixed

- **The hairline belonged to the bar and was drawn by the strip** (J377). Its own comment
  said what it is — *it divides the bar from the panel, and belongs to neither tab* —
  and once the strip could scroll, that stopped describing where it lived: the line would
  slide off the side with the labels. `min_width: Percent(1.0)` on the strip was the first
  attempt and does nothing, for the reason milestone 368 found on `ExpansionTile`: a
  percentage resolves against its parent, and a parent inside a horizontal scroll has no
  definite width to resolve against — two tabs in a scrollable bar ruled an 84 px stub
  across a 400 px bar. The line is `TabBar`'s now, at the same offset the strip used, and
  all 91 goldens are unchanged: it did not move, it changed hands.

### Added

- **A grid that builds what you can see** (J376). `GridView` had two forms and both built
  **every** cell every frame — fine for a dozen colour swatches, wrong for two thousand
  photographs, which is the argument milestone 375 had just made about lists and which does
  not stop being true because the cells are arranged in rows.
  `GridView::builder(columns, count, build)` is the third form.

  **A windowed grid is a list of rows**, and that is the implementation rather than an
  analogy: the grid reports a `VirtualList` whose item is a row of cells, so nothing new
  windows, measures or scrolls. Its scrolling test asserts on `scrollable_maxes` and on
  where a tile lands after a scroll, and neither the grid nor this milestone contains any
  scrolling code.

  `virtual_list` takes the **viewport** now. The row height comes from the tile shape and
  the width the grid was given, and the window comes from the row height — so a grid
  cannot say how many rows fit until it knows how wide its columns came out. Two call sites
  ask only *whether* a widget windows its children and pass `Size::ZERO` with a comment
  saying so, since a size that will not be read is the honest argument.

  The row factory is composed **once** and captures the column count, the spacing and the
  cells — none of which depend on the width. The row height deliberately is not in it: it
  is the item extent, recomputed every call, and the gap below a row is the row's own
  bottom padding. A factory that knew the height would be wrong the moment the window was
  resized, and a `OnceCell` cannot be rebuilt.

  Three decisions in the arithmetic: tiles are **square** when neither `aspect` nor
  `tile_height` is given, because division needs a number and a square is what a photo grid
  means when it says nothing; a **short last row keeps its columns**, since "three columns"
  did not stop meaning three on the last row; and the **gap goes below the row**, because
  the list hands its item the whole extent and a row that stretched would eat the spacing.

### Changed

- **`GridView`'s `Widget` implementation asks for `Msg: Clone`** (J376), and `ColorPicker`
  with it, since it holds one. It breaks nothing: `build_ui` has always required
  `Msg: Clone`, so the looser bound described a widget that compiled and could never be
  shown.

### Added

- **A list that runs the other way** (J375). `ListView` scrolled down the screen and only
  down the screen, so a row of cards, a strip of thumbnails or a shelf of covers was not
  possible — and each of them is this same list turned on its side. `ListView::axis`
  says which way.

  A horizontal row could already be built as a `Flex::row` inside a
  `SingleChildScrollView`, and that builds **every** child every frame. Fine for six
  chips, wrong for two hundred covers — and two hundred covers is exactly what a shelf
  is. The whole reason `ListView` exists is that the per-frame cost should follow what is
  visible rather than what exists, and that does not become less true when the axis
  changes.

  The walk's virtual-list branch was seventy lines of `offset_y`, `item_height` and `top`;
  it is a **main** axis and a **cross** axis now, with two `match` at the end that know
  which is which. A second copy of the branch would have read more plainly and is how the
  two would drift: the window arithmetic is where a reversed list gets its signs right,
  and milestone 361 already had to correct that arithmetic once. `item_height` is
  `item_extent`, since a field called `item_height` on a horizontal list is the same kind
  of lie milestones 367 and 369 went round correcting.

  Padding follows the axis — the leading inset is the one at the end the items start
  from, so `left` leads across and `top`/`bottom` become cross insets — and so does
  `reverse`. The cross axis is handed over **whole**, as milestone 351 established: a card
  whose height nobody set is as tall as the shelf rather than hugging its contents.
  `Axis::Both` reads as vertical, because a list virtualises along one axis and that is
  what lets it place item `n` without building the ones before it.

  Scrolling came free — the same `Scrollable` every other scrolling surface registers,
  with `max_x` where `max_y` used to be — and is tested anyway: seven of the eight new
  tests are layout at rest, and the eighth drives the offset and watches the window move,
  because a shelf that lays out correctly and does not move is the failure every other
  assertion would miss.

- **An image from somewhere else** (J374). `Image::network(url)` — the last of the
  reference's four image sources, and the one that needed the two steps before it: bytes
  over HTTP from milestone 373, and a process-wide store from 372.

  `frus-widgets` has no runtime, no socket and no dependency on `frus-shell` — the
  dependency runs the other way — so the shell **registers** a fetcher on the way up and
  the widget layer only asks. The shape the decoder took a step earlier, for the same
  reason: the layer that knows *how* is not the layer that knows *when*. A host with its
  own HTTP client points `set_image_fetcher` at that instead.

  A view runs sixty times a second, so the first call starts the work and reports
  `Loading` while every later one is a lookup — without that, one picture is sixty
  requests a second. `Image::state()` answers `Ready`, `Loading` or `Failed(why)`: three,
  because *not here yet* and *will never be here* are shown differently, one a placeholder
  and one a message.

  There is deliberately **no** `loading` or `error` slot. The reference needs them because
  its load happens inside a stateful widget the application cannot see into; here the
  state is readable and the application writes the `match` it would have written anyway. A
  closure stored in the widget would say the same thing where the application cannot see
  it, and would force `Image` to carry the message type for a branch the view can already
  take. That is this framework's model rather than a gap in it.

  An image in flight keeps the interface drawing, or the frame that would show it never
  happens — counted once when the interface is built, not read off the widget, because
  showing a placeholder means taking the image *out* of the tree while the fetch goes on.

### Fixed

- **A tally beside the store can lie** (J374). The in-flight count began as an
  `AtomicUsize` incremented at the start of a fetch and decremented in its callback — a
  second copy of a fact the store already held, and the two can disagree. It survives a
  `forget_fetched_images` that empties the store, and a callback arriving after one
  decrements it below zero and wraps. Either way the interface is left redrawing for ever
  over work that finished. Counting the entries that say `Loading` cannot drift, because
  there is nothing to drift from.

- **A hung request would pin the screen redrawing** (J374). In flight means redrawing, so
  a dead server would leave an application repainting at full rate until someone closed it
  — on a phone, on a battery. The shell's image fetcher carries a 30-second deadline:
  long enough for a photograph on a slow connection, short enough that a hung socket
  becomes a failure the interface can show and settle from. A request without an answer
  has to become one with a failure.

### Added

- **A response is bytes** (J373). `fetch` returned a `String` and so did
  `Request::send`; there was no other way out of the HTTP layer, so the framework could
  fetch a JSON document and could not fetch a picture — an image over the network was
  blocked on a missing **verb**, not on a missing widget. That is backwards: a response
  *is* bytes, and text is a conversion applied to them.

  `Request::send_bytes` and `fetch_bytes` are that shape, and `send` is now them plus
  `String::from_utf8`. Native reads `into_reader()` where it read `into_string()`; the Web
  reads `arrayBuffer()` where it read `text()` — the same body without the browser
  deciding it is a string. A non-UTF-8 body is still a `FetchError::Decode`, so nothing
  that used `send` behaves differently. One transport with two endings, rather than a
  second copy of an implementation that arms an `AbortController`, a `setTimeout` and a
  closure outliving the request.

  `MAX_RESPONSE_BYTES` is **32 MiB**, because a client that reads an unbounded body from a
  server it does not control can be made to run out of memory by that server — the
  reason `ureq`'s own text reader has a limit. Large enough for a photograph, a font or a
  document; small enough to be a limit rather than a formality. The native reader takes
  `MAX + 1`: `take(MAX)` fills exactly `MAX` for a body at the cap and for one far over
  it, so the check would otherwise pass on a body already silently cut.

  The test **moves bytes** — a one-shot HTTP server on a loopback port, four assertions
  over a real socket — because what changed here is exactly what a builder test cannot
  see.

### Fixed

- **The HTTP helper had no CI coverage** (J373). A step called *"Test with optional
  features (net + json)"* ran `cargo test -p frus --features json`: no `net`, and on the
  facade rather than on `frus-shell`, where the tests live. Its 55 unit tests sit behind
  `#[cfg(feature = "net")]` and the routine `cargo test --workspace` compiles them out, so
  the step whose name promised to cover them covered nothing — and ran green throughout,
  because a test that is compiled out passes by being absent. It now runs what its name
  says, on both crates.

### Added

- **An image an application can actually load** (J372). `Image::new` took decoded pixels
  the caller already held, and it was the only constructor there was — so the framework
  answered the second question about images and left the first one, *how do I show my
  logo*, entirely to the application. The demo's own answer was a `OnceLock`, an explicit
  `frus_image::decode`, a `map`, an `unwrap_or_else` and a decoder dependency it had to
  add itself. When the framework's own demo has to write the caching, the piece is missing.

  `asset!("../assets/logo.png")` is that piece, over `Image::memory(bytes)` underneath. A
  **macro** is the honest shape rather than a shortcut: the reference needs an asset
  bundle — a manifest, a directory convention, a runtime loader — because its language
  cannot put a file into the program at compile time, and Rust can. `include_bytes!` takes
  a literal path resolved against the file that writes it, which is `Image.asset`'s
  ergonomics and more: nothing to find at run time, no path to get wrong on another
  machine, no manifest to keep in step.

  `frus_core::cached` decodes **once per process**, keyed by the slice's **address**
  rather than its contents — exact for `include_bytes!`, whose bytes are fixed for the
  life of the process, and a pointer comparison where hashing would cost re-reading the
  file on every frame that shows it. The `'static` bound is what makes that sound: bytes
  that can be freed could have their address reused, and the cache would hand back the
  wrong picture.

  A failure is a **value** and is cached too. `Image::error()` says why, the widget paints
  nothing, and a broken image takes no room unless it was given some — anything the
  caller did say is honoured, which keeps a page from jumping over one bad asset. There is
  no `errorBuilder` yet, deliberately: `Image` is not generic over the message type, and
  making it so to hold a replacement widget is a change worth making on purpose. The demo
  now `match`es on `error()` and supplies its own fallback, which reads as what it is.

  Decoding arrives as the droppable `images` feature, on by default. `frus-widgets` still
  does not force the decoder's dependency tree on anyone, and dropping the feature reports
  a missing decoder rather than panicking — the rule the bundled fonts already follow.

- **An image nobody could hear** (J371). `frus_core::Role::Image` had been in the
  accessibility vocabulary a long time and `frus-shell` mapped it to the platform's;
  **nothing in the framework ever emitted it**. Every picture in every application — a
  photograph, a chart, a logo, an avatar in a list of people — was silent to a screen
  reader, which met a gap where the content was and was told nothing at all, not even that
  something was there. It went unnoticed because a `Role` nobody constructs compiles
  perfectly and no test asks for it.

  `Image::semantic_label` is what a reader is told instead of the picture. Unlabelled, it
  still announces its **role**, which is a decision rather than a fallback: a reader who
  meets it learns something is there and can move past it. Being left out entirely is the
  application's call, and `exclude_from_semantics` is how decoration says so — winning
  over any label given, which is the reference's rule and the only unambiguous one.

  The test drives `build_ui` and reads what the walk **collected**, rather than calling
  `semantics()` and believing it: a hook nobody calls is a shape of bug this project has
  been bitten by before.

  `match_text_direction` is the reference's opt-in mirror for a picture that **points** —
  an arrow, a speech bubble with a tail. Off by default there and here, because a
  photograph of a person does not want flipping because the interface is in Arabic. The
  reference calls it "a scaling factor of -1 in the horizontal direction"; here it is a
  **sign**, since the shader reads `uv.xy + unit_pos * uv.zw` and a negative width walks
  the same span backwards. No transform, no layer, and nothing in `frus-gpu` changed. The
  alignment stays physical either way: those are separate questions on purpose.

  Left: `repeat` and `filter_quality` share one piece of work — a sampler chosen per
  draw, where there is one hardcoded `ClampToEdge`/`Linear` today. The asynchronous
  builders are a **design difference** rather than a gap: `ImageHandle` is an
  `Arc<ImageData>`, decoded pixels the application already holds, so there is no load in
  flight for the widget to report on.

- **A checkbox that can answer "some of them"** (J370). `Checkbox` had two answers; the
  reference's has three, and the third is the one a real list needs — a "select all"
  above five rows of which three are ticked. Drawn unticked that header says *nothing here
  is selected*, which is false; ticked, *everything is*, which is also false. There is no
  honest two-state answer, so the control has a third: the reference calls it `tristate`,
  the platform's accessibility vocabulary calls it `mixed`, and both mean the same thing.

  `Checkbox::maybe(Option<bool>)` is that control, `None` being partly on, and a click
  cycles off → on → partly on → off, which is the reference's order. It needed a
  second callback: `on_toggle` takes `Fn(bool) -> Msg` and there is no value of `bool`
  that means partly, so `on_change` takes `Fn(Option<bool>) -> Msg` and wins when both are
  given. A tristate box wired only to the old one is **not** left silent — it reports the
  two answers that type can carry, with partly-on reading as on, which is what a click on
  it moves away from; emitting nothing would be a widget that looks live and is not.

  Both **on** and **partly on** fill the box and only the mark differs, which is the
  reference's drawing and the right one: the filled surface is what says *this is not
  simply off*, and the mark says which of the two it is. The tick stays a `✓` glyph,
  which is what it has always been; the partly-on mark is **drawn** — a bar, two numbers
  on the box the border already uses — for the reason milestone 368 gave about the
  expansion tile's arrow: a glyph is a font's opinion and can be missing outright.

  `frus_core::Toggled` gained `Mixed` and the shell maps it to AccessKit's, so a screen
  reader is told `mixed` rather than handed a lie in one of the two directions;
  `Semantics::maybe_toggled` takes the `Option<bool>` whole. Left: the reference
  **animates** between the tick and the bar, the tick retracting into it and back — a
  `CustomPaint` and a curve rather than a new concept, and it wants its own step alongside
  the switch's thumb.

- **An expansion tile that is a tile** (J368). `ExpansionTile`'s whole surface was
  `new(title, open, on_toggle)` and `content(widget)`. No leading widget, no subtitle, no
  trailing widget, no colours, no measurements — a title as a `String`, a header painted by
  hand at a hardcoded 40 px with hardcoded 18 px text, and `▾`/`▸` as **text glyphs**. It
  was the last widget in the catalogue still written that way, and a plain breach of the
  standing rule.

  The reference's `ExpansionTile` **is** a `ListTile` with a chevron and a body underneath,
  and ours is one now. That single decision brings everything milestone 336 already built
  into a tile — the Material 3 heights, the leading slot's minimum width, the text column
  that gives way while the slots either side keep their size, the state layer, the selected
  colour — instead of doing it a second time, worse. On top: `subtitle`, `leading`,
  `trailing`, `show_trailing_icon`, `control_affinity`, `dense`, `tile_padding`,
  `children_padding`, and four colour pairs each with a *collapsed* counterpart.

  Two questions the slots forced. The chevron and a widget can want the same place:
  `control_affinity(Leading)` gives it to the chevron and drops the other, because two
  widgets in one slot is a bug a row of fixed slots cannot report. `trailing` goes the
  other way and **replaces** the chevron — a row whose end carries a switch is not also
  carrying an arrow. And the tile is assembled in `build_themed`, under the theme of the
  subtree it sits in, or one inside a `Themed` would come out in the wrong palette.

  `IconName::ChevronDown` and `ChevronUp` join the icon set, on the same 24×24 grid as the
  rest: the old header's `▾` was a font's opinion — a different size and weight from the
  icons beside it, and missing outright from a font without those code points.

  **It found two bugs in `ListTile`.** The chevron came out against the last letter of the
  title rather than at the end of the row: the tile was the right width, the `Flex::row()`
  inside it was not, so it hugged its slots and the `Expanded` text column between them had
  nothing to push against. Every tile with a trailing widget has drawn it halfway across
  the row since the tile was written, and no golden had one until this milestone put a
  chevron there. Fixing that turned up the second: a row that only *grows* fills the tile
  and then runs straight through it when its content is wider, so a long title pushed the
  trailing slot **265 px outside a 200 px tile** — reported by the overflow band, and still
  drawn off the side of the list. `Expanded::new(row)` says both at once: grow into the box,
  give way rather than run past it, and no content-sized floor underneath.

### Changed

- **An inflexible child of a row or a column is no longer squeezed** (J349).
  `flex_shrink` defaults to `0.0` — the reference's rule, where a child that did not ask
  to give way keeps the size it asked for and a line that does not fit overflows and says
  so. Flexbox's default of 1 is what let a 40 px delete button be laid out at 13 and drawn
  off the card, silently, for three milestones. `shrink(1.0)` asks for the old behaviour.

  The exception is a box with a **single** child, which is handing its own constraints
  down rather than dividing a line up: the walk grants it the right to give way. That is
  the same exception the fill request (J342) and the main-axis floor (J344) both make.

  It exposed six real layouts that were one to twenty-five pixels short and had been
  paying for it by quietly crushing something: a card footer that now wraps, a journal
  header that now ellipsises, an arithmetic slip of four pixels, a text fixture, a
  navigation rail and a two-pane that meant `Expanded`.

### Changed

- **The names the reference uses** (J367). Twenty widgets were the reference's widget
  under a different name — `TextInput`, `List`, `Grid`, `Scroll`, `Toast`, `NavBar`,
  `NavRail`, `Spinner`, `Collapsible`, `Popover` and the rest. The whole proposition here
  is that someone who knows the reference already knows this framework, and a synonym
  breaks that at the first line they write: they type what they know, it does not exist,
  and they go looking for a widget that was there all along. It breaks search too — a
  reader after "how do I limit a text field" searches for `TextField`.

  `TextInput → TextField`, `List → ListView`, `Grid → GridView`,
  `Scroll → SingleChildScrollView`, `Collapsible → ExpansionTile`, `Alert → AlertDialog`,
  `Toast → SnackBar`, `ToastHost → ScaffoldMessenger`,
  `Spinner → CircularProgressIndicator`, `ProgressBar → LinearProgressIndicator`,
  `SegmentedControl → SegmentedButton`, `NavBar → NavigationBar`,
  `NavRail → NavigationRail`, `Tabs → TabBar`, `Avatar → CircleAvatar`,
  `Dropdown → DropdownButton`, `Menu → PopupMenuButton`, `Popover → MenuAnchor`,
  `Refresh → RefreshIndicator`, `Barrier → ModalBarrier`, `Portal → OverlayPortal`,
  `Carousel → CarouselView` — with `widgets.text_input` and `widgets.tabs` following as
  `widgets.text_field` and `widgets.tab_bar`. Twelve widgets keep their names because the
  reference has nothing to match them to.

  Four exclusions, each a class rather than a one-off: `IconName::Menu` is a picture of
  three lines, not this widget; a word **in quotes** is a label on a screen rather than a
  name in the API; `Role::TextInput` and `Role::ProgressBar` belong to the screen reader's
  vocabulary; and `Drag::Scroll` in the shell is an internal state. The one real collision
  was `Tabs`, whose module already had a private `TabBar` — the composite takes the public
  name, since that is what somebody types, and the strip inside is now `TabStrip`.

### Added

- **A field with a limit, and one that is read but not written** (J366).
  `TextInput::max_length()` and `TextInput::read_only()`. The limit is **enforced**, as
  the reference's default is, and enforced one character at a time: a keystroke past it
  goes nowhere, and a paste that crosses it lands the part that fits — dropping a whole
  paste for being one character too long loses work the user can see they had. It counts
  **characters**, not bytes, and puts a `5/10` counter at the end of the line below the
  box, reserving that line even with no helper text to share it with. A value the *caller*
  supplied over the limit is left alone: it is the application's state, not something
  typed, and the counter shows it over.

  `read_only` is not `enabled(false)`, and the difference is the point. A disabled field
  is greyed out and inert — out of the tab order, no caret, nothing to select. A read-only
  one behaves like any other field except that typing does nothing: everything that only
  *moves* still happens, so a generated reference number can be focused, selected and
  copied. Greying it out would have said unavailable where the truth is fixed.

- **The selection controls, themed and overridable** (J365). `Switch`, `Checkbox`,
  `RadioGroup` and `Slider` had **no way at all** to change a colour — not a builder, not
  a theme entry. Whatever the scheme's `primary` was, that is what they were, so an
  application whose brand colour is not its accent could not have a green checkbox. That
  breaches this project's own standing rule (themed defaults fine, hardcoded-only never),
  and `WidgetThemes` had carried eleven widgets before these four were brought in.

  Four theme structs and one builder per colour, resolved the way everything since `Chip`
  resolves: instance → `theme.widgets.<widget>` → the scheme's role. The defaults are what
  was painted before, and the goldens are the proof.

  Three things the resolution forced. A **partial** override must not tear a control in
  half: a caller who names one outline colour means the outline, so the pointer state
  falls back to the resting override before the scheme — otherwise a green checkbox turned
  grey the moment a finger came near it. A switch interpolates between two **resolved
  ends**, not two resolved colours, or recolouring the track would leave the animation
  running through the old one. And a `RadioGroup`, the only one of the four that builds
  its children in advance, hands the *unset* colours down so each option resolves against
  the theme it is painted under — a group inside a `Themed` subtree takes that subtree's
  palette.

- **A slider over a range, in steps, saying where it is** (J364). `Slider::range()`,
  `divisions()`, `value_label()`, and a keyboard. Its whole surface was `new`, `width`,
  `on_change` and `enabled`: a `0.0..=1.0` control, so every caller with a real range — a
  price from 20 to 200, a font size from 8 to 72 — divided on the way in and multiplied on
  the way out, in two places that only agreed by luck. Nothing in the framework knew what
  the number meant, so the accessibility node reported a **percentage of a range the
  reader was never told**. It hands over the caller's units now, on all three sides:
  `on_change`, `Semantics::range`, and the tooltip.

  It was also not a keyboard control at all — no `focusable`, no `on_key`, so Tab passed
  it by and the arrows did nothing, on the one widget arrows are the obvious way to move.
  Its own `RangeThumb`, in the same file, has had both for milestones. An arrow moves one
  division, or 5 % of the travel when there is no division, which falls out for free once
  a step is a fraction rather than a number of units.

  Two things are held rather than rejected: a backwards range is sorted (an empty travel
  is a worse answer than the obvious one), and a value outside the travel is clamped, so
  an app rebuilding its view from state it is midway through editing does not panic. Left:
  `onChangeStart`/`onChangeEnd`, which need drag-edge hooks the widget trait does not
  have.

- **Room around what scrolls** (J363). `Scroll::padding()` and `List::padding()`, the
  last item on milestone 357's audit list and the one every scroll view in the reference
  takes. The room is **inside** the viewport and scrolls with the content, which is the
  only reading that is any use: a list padded 88 at the bottom has that room at the end of
  its content, reached by scrolling to it, so the last row clears a floating button —
  where room taken out of the window would sit there permanently and the last row would
  still slide underneath. Along the cross axis it simply insets the content.

  One hook, `Widget::scroll_padding`, read by both the scroll branch and the virtualised
  list — which could not have wrapped its items in a padded box even if we had wanted to,
  since there is no box holding them. A reversed list clears its **bottom** inset first,
  because that is the end its items start from and it is still where it looks; that is
  `lead = if reverse { pad.bottom } else { pad.top }` and nothing else, and it is the
  reference's answer too — `SliverPadding` resolves before and after in the axis's own
  direction, which for a reversed vertical list runs upwards.

- **A decoration painted over the child** (J362). `Container::foreground()`: the last
  thing milestone 357's depth audit named on `Container`, and the one it said had no
  workaround. A background goes behind the content, which is useless when the whole point
  is that the content does not cover it — an outline over a photograph, a wash across a
  tile that is out of stock, a hairline over a card. The only way to say it was a `Stack`
  with a second layer the size of the first, built twice and kept in step by hand.

  `Widget::foreground` is the one place in the walk where a widget paints after its own
  subtree. It is data rather than a second `paint` because a general paint-over would need
  a second hook to stay cheap — the walk cannot know whether a body is empty — and a
  decoration is what the feature means. It sits in `walk_node`, which puts it inside the
  opacity group, transform and clip the container asks for, under its own theme override,
  and inside the repaint-boundary capture. A foreground naming no radius takes the
  container's, which the reference makes you write twice.

- **The edge an axis starts at** (J361). A reversed area glowed at the **top** when it
  refused to go further back, and pull-to-refresh above one still listened at the top:
  both read the sign of a refused delta in *offset* space and assumed offset zero was the
  top of the screen, which reversing is precisely the act of denying.
  `Scrollable::refused_edge` makes that assumption explicit and corrects it in one place,
  with `Scrollable::start_edge` for the widget that wants the edge an axis *begins* at
  rather than the one a refusal happened at. The refresh guard now reads "the edge this
  axis starts at" instead of naming the top — which is what it always meant.

- **A box that confines what it holds** (J361). `Container::clip()`: the container's own
  radius becomes the clip its child is held to, so a box with rounded corners no longer
  leaves a photograph inside it square. Off by default, as in the reference, because it
  costs a compositing layer — wrapping the child in a `ClipRRect` remains the idiom when
  the corners belong to the child rather than to the box. Still missing:
  `foregroundDecoration`, which wants a paint-after-children hook the walk does not have.

- **A list that counts from the bottom** (J360). `List::reverse()`: item 0 at the bottom,
  item 1 above it, resting there. The other half of a conversation view — a scroll can
  anchor its content to the end, but only a list decides which end an *index* is, and with
  index 0 the newest message, appending one leaves every other item where it was. The
  virtualisation window needed no change at all: a reversed list counts indices from the
  end and (since J359) a reversed scroll counts pixels from the end, so the two agree
  about which way forward is.

- **A scroll that runs from the far end** (J359). `Scroll::reverse()`: content shorter
  than the viewport sits at the bottom, and — the point — the view **stays** at the end
  when content arrives, because offsets are counted from the end an axis starts at. A
  conversation resting at offset 0 is resting at its newest message however many arrive.
  Nothing changes for the user's hand: a finger pushes the content the way it moves, and
  the thumb rests where the content is.

  The sign change lives in **one** function, `Scrollable::offset_delta`, called by all
  five places a screen delta becomes an offset (wheel, drag, release fling, on two axes)
  — a minus sign at each would have been five chances to be wrong in a way only a device
  shows, and this one is unit-testable. `Scrollable::content_origin` does the same for the
  paint. New: `Widget::scroll_reverse`, `Scrollable::reverse_x`/`reverse_y`.

- **An image knows how big it is** (J358). `Image::new` required a width *and* a height,
  which meant a picture could not be shown by anyone who did not already know its pixel
  dimensions — the one thing a bitmap always knows about itself. Both are optional now,
  and the reference's rule fills in the rest: both given is that box, one given derives the
  other from the image's ratio, neither is the bitmap's own size. `alignment` says where a
  letterboxed image sits **and** which part of a cropped one survives — one anchor, running
  opposite ways, with a test for each — and `opacity` fades the tint rather than costing a
  layer. `BoxFit::apply_aligned` is the new entry point; `BoxFit::apply` is it, centred.

  **Breaking**: `Image::new(handle, w, h)` becomes `Image::new(handle).size(w, h)`.

- **`Positioned`, and a `Stack` that can place a layer** (J357). A badge on a corner, a
  caption over an image, a bar across the bottom of a photo: all of them are a stack with
  one layer pinned somewhere, and none of them could be written — `Stack`'s whole surface
  was `new/width/height/flex/layer`. `Positioned` pins any of the four edges, and what is
  pinned decides the layer's size as well as its place, as in the reference. `StackFit`
  and `Stack::alignment` cover the rest of the reference's surface; the default fit stays
  `Expand` where the reference's is loose, deliberately and with the reason written where
  the choice is made. New: `Widget::positioned`, `Widget::stack_loose`,
  `Layout::compute_tight` — a box the content is *forced* into, next to the ones it is
  asked about and handed.

  Found by auditing the reference's constructor parameters against our builder methods,
  now that the widget catalogue counted in J336 is closed and what is left is depth. The
  audit also points at `Scroll` (no `padding`, no `reverse`) and `Image` (no
  `width`/`height`/`alignment`/`repeat`).

- **A grid can work its column count out from a tile width** (J356). `Grid::extent(160.0,
  n, |i| tile(i))` is the reference's other grid delegate: as many equal columns as it
  takes for none to exceed the maximum — four of 125 in a 500 px grid at `extent(150.0)`,
  where CSS `auto-fill`, which looks like the same idea, would give three of 166. The
  count needs the width before the layout, which J355 made possible; the cells come from
  a factory because there is no grid to put them in until the count is known, with
  `LayoutBuilder`'s caveats (the factory runs more than once a frame, and the cells have
  no retained state). `Grid::new` is unchanged.

  Its first tests found a defect one widget along from J351: a `LayoutBuilder`'s content
  was **constrained** rather than **handed** its box, so a root with no width of its own
  hugged — a grid built there laid its columns out at nothing. Fixed in both the
  measurement and the paint. One golden moved and was read: a builder's `flex(1.0)` child
  now fills the box it was built for instead of hugging its label.

- **`Flexible` by name** (J354). The loose fit has existed since milestone 334 as
  `Expanded::new(child).loose()`, but the reference calls the widget `Flexible` and makes
  `Expanded` its tight subclass, so an application ported from it types a name that was
  not here. `Flexible` and `FlexFit` now are, with the reference's defaults on both — an
  `Expanded` is tight, a `Flexible` is loose — and `Flexible::fit(FlexFit::Tight)` takes
  the fit as a value. The two build the **same box**: the three properties that make a
  flex item live in one place, not two, and the tests assert the same row built both ways
  lays out identically.

- **A grid's tiles can have a shape** (J353). `Grid::aspect(ratio)` gives every cell the
  same `width / height` — `1.0` for a square board, at any width, since the height follows
  the column's — and `Grid::tile_height(px)` gives every row an exact extent, which is the
  reference's per-tile main-axis extent. `row_gap`/`column_gap` split what was one `gap`,
  as the reference's two spacings are separate. Without one of them a row is still as tall
  as its content, which is what a grid of forms wants and what the framework's own colour,
  date and time pickers are built on; the reference's default of square tiles is not taken,
  deliberately. New: `Widget::tile_shape`, `Layout::set_tile_shape`,
  `Style::grid_row_height`, `Style::row_gap`, `Style::column_gap`.

  Checked at the same time, and already right: a **scroll** fills its content's constrained
  axis when exactly one axis is free (and leaves both alone for a two-dimensional scroll),
  and a **table**'s cells are tight across and free down, which is the reference's rule.

- **The overflow band says which edge and by how much** (J348): `RIGHT OVERFLOWED BY 86
  PIXELS` across the stripes, in the reference's words and at its metrics, turned a
  quarter turn on the vertical edges. The console has said as much since milestone 335,
  but the console is on the developer's machine and the band is on the device — and a
  photograph of a phone is what half the bug reports in the world are made of. New:
  `Scene::transformed`, a group composited through an arbitrary affine.

- **`frus_text::line_spans`** (J347) replaces `visual_lines`: the visual lines a text breaks
  into, as **byte ranges** rather than as strings, because what the caller wants is a
  prefix of the original.

- **`RichText` answers the same questions as `Text`** (J346): `align`, `max_lines`,
  `overflow`, and wrapping by default. They are questions about *text*, and the styles
  being mixed changes none of them. The cut is the one genuinely harder part —
  `frus_text::runs_cut_at` returns the byte offset into the concatenation of the runs at
  which the paragraph runs past its line limit, and the widget splits the runs there,
  keeping the styles; the ellipsis takes the style of the run it ends.

- **A striped band across every box whose children ran past it** (J345), in debug builds.
  Milestone 335 taught the layout to notice an overflow and the shell to report it in the
  console; this is the half a *screenshot* shows, and a screenshot is what a bug report is
  made of. Black and yellow diagonal stripes over a tenth of the box, which are the
  reference's colours and its fraction — the point of it is to be recognised on sight.

- **`Widget::main_axis_floor`** (J344): the width below which a widget will not be squeezed
  *along a row*. Flexbox has one `min-width` for both axes and the reference does not: along
  a row its inflexible children are never squeezed, and across a column the same children
  are handed a width. The walk applies the floor only where the parent runs horizontally.

- **`TextAlign`, `max_lines` and the four `TextOverflow` modes on `Text`** (J343). Where
  the lines sit inside their box, how many of them there are, and what becomes of the ones
  that do not fit — three of the reference's questions the widget could previously answer
  only with `wrap()` and `ellipsis()`. `Clip` intersects the primitive's clip with the box,
  and only where the text genuinely does not fit; `Fade` wraps the text in a masked group,
  which is the first time milestone 339's mask machinery is reached from a widget's paint;
  `Ellipsis` cuts the last kept line; `Visible` draws past the box, as every text did.

- **`frus_text::visual_lines`** (J343): the lines a text breaks into inside a box, at most
  so many of them, and whether there was more. A cut has to fall on a break the shaper
  chose or the words move, and a measurement cannot say where those are.

- **`Scene::masked` and `Scene::text_block`** (J343), and `TextBlock`, which carries the
  box width, whether it wraps, and the alignment inside it as one value — they are one
  decision.

- **`Row` and `Column`** (J342), with the reference's defaults rather than flexbox's: the
  run **fills the line it is given**, its children are **centred** across that line rather
  than stretched to it, and `start` is the start of the **reading direction**. `Flex` is
  unchanged and undeprecated — it is the flexbox primitive, and the framework's own widgets
  still use it.

- **`MainAxisSize`, `VerticalDirection`, `Justify::SpaceEvenly`, `FlexDirection::RowReverse`
  and `ColumnReverse`, `Style::align_self`** (J342) — the settings `Row` and `Column` need
  and `Flex` never had.

- **Baseline alignment** (J341): `Align::Baseline` on a row, plus the `Baseline` and
  `IgnoreBaseline` widgets — which closes the widget catalogue the count of milestone 336
  opened. A row measures each child's baseline, takes the deepest, and turns the difference
  into a top margin, so a figure and its unit sit on one line however different their
  sizes. taffy cannot do this: its measure function asks a leaf for a size and a leaf has
  no way to answer with an ascent.

- **`frus_text::baseline`** (J341), shaped rather than derived from the point size, and
  taken from the same layout run the renderer positions glyphs with. Memoised on the size
  and the resolved weight and style, which is everything it depends on.

- **`BackdropFilter` and `BackdropGroup`** (J340), which closes the filter subsystem. A
  backdrop filters **the frame so far**, so a frame containing one is built in a staging
  texture, cut into segments at each backdrop, and blitted to the target at the end; a
  frame without one takes the single pass it always took. `BackdropGroup` makes several
  backdrops share one filtered copy — sixty frosted rows, one blur — keyed by the group's
  own identity in the tree.

- **`FilterContext`** (J340): the filter hook now carries the widget's box *and* the
  enclosing backdrop group. Both are properties of where the widget turned out to be, and
  a widget cannot see its own ancestors.

- **Three filters** (J339): `ColorFiltered`, `ImageFiltered` and `ShaderMask` — the three
  that apply a pixel effect to their **own** subtree. They ride on the layer a subtree is
  already composited through, as independent slots on one `LayerFilter` applied in a fixed
  order (image, then colour, then mask). Two filter widgets nested one inside the other are
  **folded into a single layer**, because a layer nested in another is not re-composited;
  the fold refuses when both want the same slot, and those nest properly instead.

- **`BlendMode`** (J339): the Porter–Duff set plus the separable blends, eighteen in all.
  Needed by three separate things — a `ColorFilter::Mode`, a `ShaderMask`, and the backdrop
  to come — so one enum pays for itself three times.

- **A separable image-filter pre-pass** (J339): blur, dilate and erode, as two passes of
  twelve taps per side rather than one of `(2n+1)²`. The step scales with the radius, so a
  40-pixel blur costs exactly what a 4-pixel one does. The result is cached with the layer,
  and the filter is part of the cache key.

- **Shortcuts and actions** (J338): `Shortcuts`, `Actions`, `CallbackShortcuts`,
  `ActionListener`, `KeyboardListener` and `FocusableActionDetector`. Two steps rather than
  one, as the reference has it — a keystroke names an **intent**, and the innermost
  `Actions` that answers it supplies the message — so keys and handlers can be replaced
  independently. An intent nobody answers is inert, deliberately.

- **`KeyStroke` and `ShortcutKey`** (J338): a general key vocabulary, separate from `Key`,
  which is the one text editing needs. Letters match without case; Shift alone is not a
  command, which is what makes a bare-letter binding coexist with typing.

- **`Ui::keystroke()`** (J338), which resolves a stroke against the scopes containing the
  focused stop. A scope records the *range of focus stops* it contains — the walk is
  depth-first, so a subtree's stops are contiguous — which answers "is focus inside this
  subtree" without an ancestor test, and records innermost-first for free.

### Changed

- **A limited paragraph reaches the renderer as one prefix** (J347), not as a list of lines
  glued back together with newlines. Lines glued back together are a paragraph *per line*,
  and every rule that spans a paragraph stops working — most visibly justification, which
  leaves the last line ragged and can only do that if it knows which line is the last. A
  paragraph that is justified *and* limited to two lines could not be drawn a milestone ago.
  All ninety-one goldens agreed before and after: the renderer was always going to break the
  prefix in the same places.

- **A box with no room at all gets an ellipsis** (J347), where `truncate` used to hand back
  the whole string. The exception came from the app bar, whose title room has a floor of
  64 px and cannot produce a zero; what it actually did was let a genuinely collapsed box
  draw its label over whatever was beside it.

- **An overflow is measured on the unrounded layout** (J345). taffy rounds every node's
  edges to whole pixels independently, so a box 169.6 tall whose child sits at 21.6 and is
  148 tall becomes a box of 169 with a child at 22 — one pixel of overflow that exists
  nowhere but in the rounding. Four goldens wore a band for it the day the band was
  painted. It was making the console reports untrustworthy too: a survey that cries wolf at
  rounding is one nobody reads.

- **`Tabs` takes the width it is offered** (J345), as the reference's does. Sized by its
  content, the widest thing on the busiest tab decided how wide the whole control was — so
  the bar jumped from tab to tab, and a panel that did not fit hung out of whatever was
  centring it rather than being told to fit.

- **`Text` wraps by default** (J344), as in the reference. The flag was never the reason it
  did not: a text *declared its own width*, and a box that answers before it is asked cannot
  be told to wrap. A wrapping text is now measured from the space offered. Eighteen goldens
  moved, all of them by one or two pixels of vertical shift — a measured height is
  `lines × line height` where a declared one was rounded up — and all eighteen were read.
  `wrap()` is kept: saying it at the call site is not redundant when it is the whole reason
  the widget is there, and `no_wrap()` is the other half.

- **A text that says what to do on overflow is clamped to its parent** (J343). It is what
  makes the overflow modes fire at all: a text declares the width it wants, and without the
  clamp a narrower box does not take it away — the words draw past the edge, which is what
  the mode was set to prevent. It also lifts the automatic minimum size that makes a text
  refuse to shrink, which `ellipsis()` alone used to do. A text that has said nothing is
  untouched, and all 88 existing goldens agree.

- **An aligned text fills the width it is offered** (J343), through the hook milestone 342
  added. A box exactly as wide as its own words has nowhere to align them to.

- **A request to fill the parent travels up the layout walk** (J342). Filling is a question
  about the *parent* — grow along an axis it shares, stretch across one it does not — so a
  widget cannot answer it alone; and stopping at the parent leaves `Container > Column >
  Row` twenty pixels wide, each of the three as wide as the one inside it. The request now
  crosses each container that would take its size from the child, and stops at any box that
  was given a size of its own. At the root, where neither growing nor stretching means
  anything, it becomes a percentage of the room the layout is computed in — which hugs the
  content again when that room is unbounded.

- **The layout tree carries each leaf's baseline** (J341). The alternative was a second
  walk of the widget tree alongside the rectangles, which would have been a copy of
  `build_layout` waiting to drift out of step — a scrollable, a stack, a page view and a
  fitter are all *leaves* there, with their contents laid out elsewhere.

- **A layer nested in another layer is drawn** (J340). It never was: a group renders into a
  texture and composites it, and a layer found inside that group is not a primitive the
  group can paint, so it was skipped — a rounded card around a fading group, a clip around
  a transform, silently gone. A group now renders its nested layers first, depth-first,
  and composites them into its own pass. Two goldens moved and every pixel that changed
  got brighter: the milestone-339 coverage fix finally reaching the antialiased edges of
  those layers.

- **A clip around a filter is one layer, not two** (J340), which is what makes
  `ClipRect` around a `BackdropFilter` — the shape the reference tells callers to reach
  for — mean what it reads as. A backdrop refuses to share a layer with a filter *of the
  layer itself*, but shares with nothing at all, and a clip is nothing at all.

- **Layer compositing blends premultiplied** (J339). A layer texture is premultiplied — it
  was painted over a transparent target — and the composite pass was handing it to the
  straight-alpha blend, which multiplied the colour by the coverage a second time. Invisible
  on opaque content, a slight darkening on an antialiased edge, and unmissable on a mask
  fade, which ran to black instead of to the background. Half of white over black now reads
  188 rather than 137.

- **Colour filters and blends are evaluated on sRGB-encoded values** (J339), the space
  colours are authored in — not on the linear light the GPU holds, where the same greyscale
  matrix would make pure red more than twice as bright. The blur is the deliberate
  exception and stays linear, because a blur is an average of light.

- **The shell tracks Alt and Meta** (J338), and checks application shortcuts after its own
  keys (system back, F12) so that no binding can take those away, and before everything
  else. A stroke with no Ctrl, Alt or Meta goes to a focused field first.

- **The focus wrappers** (J337): `Focus`, `ExcludeFocus`, `ExcludeFocusTraversal`,
  `FocusTraversalOrder` and `FocusTraversalGroup`. Focus was a property of a widget and Tab
  followed the walk; what was missing is the ability for a *caller* to say something about
  focus without owning the widget that has it. `ExcludeFocus` and `ExcludeFocusTraversal`
  are deliberately two things: a panel behind a sheet is unreachable, a toolbar button is
  reachable and simply does not belong in a form's keyboard order.

- **`Ui::traversal_order()` and `Focusable`** (J337). A focus stop is no longer
  `(id, rect)` but carries whether Tab skips it, an explicit order and its traversal group.
  The sort is stable and runs *within* a group, so an explicit order is a local statement:
  a dialog that swaps its two fields leaves the page behind it in tree order.

- **Four subtree-scoped focus hooks on `Widget`** (J337) — `descendants_focusable`,
  `focus_skip_traversal`, `focus_order`, `focus_group` — pushed and popped around the walk
  rather than set inside it, because the walk has early returns in a dozen branches and a
  flag cleared at the end of the function would leak out of every one of them.

- **`ListTile`** (J336). The most reached-for row in the reference's catalogue and this
  framework's largest single gap: leading, title, subtitle, trailing, with the reference's
  Material 3 measurements (16 / 24 padding, a 16 px gap either side of the text, heights of
  56 / 72 / 88 by line count and 48 / 64 / 76 dense) and its text roles. The text column is
  an `Expanded`, so a long title is cut and the trailing chevron keeps its size; a
  three-line tile's subtitle wraps instead, which is what the extra room is for. Composed
  under the ambient theme rather than at construction, so a tile inside a `Themed` subtree
  comes out in that subtree's palette.

- **`Flexible`** (J336), as `Expanded::loose()` and the `flexible(child)` shorthand: at
  most its share, and less if it wants less. Flexbox shares a deficit in proportion to
  basis, so a fixed sibling still needs `no_shrink()` for the whole of it to land on the
  flexible child — a caveat the reference does not have, documented rather than hidden.

- **`Placeholder`** (J336), with the reference's blue grey, 2 px stroke and 400 px
  fallback, all overridable. It grows into the room on offer rather than insisting on its
  fallback.

- **`ConstrainedBox::new_boxed`** (J336), for a child that is already erased — what every
  widget slot holds.

- **A box that does not fit now says so** (J335). `frus_layout::Layout::overflows` and
  `Ui::overflows()` name every box whose children ran past it, with the edge and the amount;
  the shell reports each site on the console once. Three milestones (327, 333, 334) were one
  bug that took a device report to find, because nothing anywhere said a row did not fit.
  Content larger than the viewport that scrolls it never reaches the walk — scrollables,
  stacks, page views and fitters are laid out as leaves with their content computed
  separately — so no exception list was needed.

- **`no_screen_draws_outside_itself`** (J335), which surveys every demo route at a phone's
  width and a desktop's. Sixteen of eighteen were clean on the first run.

### Fixed

- **A segmented control ran past its parent** (J335), found by the survey above: four
  segments came to 584 px in a 363 px row on a phone and the last was drawn outside the
  card. The reference gives every segment the same width **capped at an equal share of the
  room**; ours had the first half and not the second. The control now stops at its parent's
  edge and the segments divide what there is, with the labels ellipsised. With room the
  share is the natural width, so nothing moved — no golden changed.

- **`Expanded`, the child that takes what is left** (J334). The reference's, and the fix for a
  device finding three milestones old: a long task label pushed the row's delete button off the
  card, out of the window and out of the hit registry, so the task could not be deleted at all.
  Three properties together — `flex_basis: 0` so the child stops telling the row how wide it
  wants to be, `flex_grow` so it takes the spare room, and `min_width: 0` to lift the automatic
  minimum that makes `flex: 1` alone a no-op on a long label. Any one of them alone does nothing.

- **`flex_shrink` and `flex_basis` on `frus_layout::Style`** (J334), with `shrink()` and
  `no_shrink()` on `Flex` and `Container`. The flex item model has three properties and this
  framework shipped with one. `flex_shrink` defaults to `1.0`, flexbox's and taffy's, so nothing
  moves until something asks.

- **`restyle`, a third hook on the transparent-wrapper macro** (J334). `forward_transparent!`
  hardcoded `style`, so a wrapper that changes the **box** had to be hand-written — the exact
  mistake the macro exists to prevent. Both `style` and `style_themed` now go through an inherent
  `restyle` each wrapper writes, and the test that checks wrappers state their hooks finds them by
  looking for the macro instead of from a hand-kept list of two.

- **A tone-gap guard on the outline roles** (J325). `an_outline_is_never_the_colour_of_a_disabled_one`
  checks both shipped schemes and three seeded ones: a live outline must sit a measurable
  number of tones away from `on_surface` at 12 %, which is what a disabled one resolves to.
  A palette cannot reintroduce this silently.

- **`light_outlines`, the first light-theme golden** (J325). Every picture in the repository
  was dark, which is how a palette bug affecting *both* schemes shipped — the light theme's
  outlines were the closer pair of the two.

- **`enabled` on `Rating`, `Stepper`, `Menu`, `Tabs` and `Pagination`** (J324). The last
  five. Every control in the framework that can be pressed can now be told not to be.
  `Stepper`'s value colour comes from the ambient theme through milestone 319's
  `ThemeBuilder` — its second consumer — rather than from a `Theme::default()` guessed at
  assembly time.

- **`blending`, a test that pins how a translucent token actually paints** (J328). A 12 %
  wash of `on_surface` is meant to read near tone 24 in the dark scheme and paints at
  tone 38: the target is `Rgba8UnormSrgb`, so the hardware blends in linear light, while
  the reference's opacity tokens assume the blend happens in sRGB. Nothing is
  misconfigured — but a whisper is painting at roughly what 33 % would give, which is the
  thread behind the disabled-state reports of milestones 324, 325 and 328. The test
  asserts the current behaviour without claiming it is right, so that changing the blend
  space is a deliberate and visible change.

- **`Text::ellipsis()`** (J333), the reference's `TextOverflow.ellipsis`: one line, cut to
  the box the layout gives it. Two things, and the second matters more — an ellipsising
  text also tells the layout it may be given *less* than it asked for (`min_width: 0`),
  where a plain one's automatic minimum size is its own content and it pushes its siblings
  out instead. The truncation itself moves out of `AppBar`, which had the only copy, into
  `text::truncate` where both callers can see it.

- **`painted_colours`, four tests asking whether a colour survives to the screen** (J331).
  Milestones 328, 329 and 330 were all one sRGB↔linear conversion in the wrong place, and
  nothing caught any of them: the widget builds the right colour, the scene primitive
  carries it, and the golden is regenerated from the same broken pipeline so it agrees with
  itself forever. These render a known colour through each surface that paints one — quad,
  glyph, path and image tint, which convert in four different places — and read the pixel
  back. A failure names the slip ("one sRGB→linear conversion too many") rather than
  leaving you to suspect the palette. Each was shown to fail: reverting milestone 330 fails
  only the glyph case; doubling `srgb_to_linear` in the three shaders fails the other three
  and leaves the glyph green.

- **`a_live_container_is_never_quieter_than_a_disabled_fill`** (J329). A control *filled*
  with a live container must be tellable from the disabled fill, on tone — measured as a
  distance from the surface — or on chroma, since the disabled fill is nearly grey. Written
  for milestone 328 and thrown away because it modelled a blend the GPU was not doing; it
  is back unchanged now that `over_surface` performs exactly that arithmetic.

### Fixed

- **An unselected checkbox's box and radio's ring were `outline`** (J332), the role for the
  edge of a container. They are not containers: they *are* the mark, and a mark takes an
  *on* colour — which `disabled.rs` has said in prose since milestone 322, while the
  enabled branch of both controls said otherwise. The reference resolves the side per
  state, and the roadmap's `on_surface` was only half of it: `on_surface_variant` at rest,
  the full `on_surface` under a finger, a pointer or focus. `RadioOption` is its own widget
  so it gets its own `Status`, and hovering one option lifts that ring alone.

- **Every glyph in the framework was painted at its linearised value** (J330), so all text
  was darker than the theme said — `on_surface` (230, 232, 236) reached the screen as
  (202, 206, 214). `text.rs` converted each glyph colour to linear before handing it to
  glyphon, reasoning that the sRGB target would re-encode it. That is right for the quads,
  whose own shader does the conversion; glyphon's shader already does it, so the colour was
  decoded twice and encoded once. Found on a device: a disabled label at (37, 39, 43) on a
  card at (36, 40, 48), invisible — `disabled_content` is (106, 109, 114), and the scene
  primitive carried exactly that. 110 goldens moved; 228 477 pixels changed and not one got
  darker, which is the exact invariant of removing a linearisation. Confirmed on hardware:
  both labels now land on their token to the byte.

- **The disabled tokens painted at roughly three times their opacity** (J329). Milestone 328
  measured it: the reference's 12 % and 38 % assume an sRGB blend, and an `Rgba8UnormSrgb`
  target makes the hardware blend in linear light. `disabled_container` and
  `disabled_content` now resolve the blend in sRGB and hand the GPU an opaque colour —
  which is what the module always said the rule was, *a disabled control flattens; it does
  not fade*. The error ran opposite ways in the two schemes and both are corrected: dark
  disabled controls were too loud (two disabled sliders were the brightest rails on the
  page, beating the live ones), light ones too faint to read. Ten goldens, read as pairs.
  Everything else translucent — scrims, ink, state layers — still blends in linear.

- **The blocking rustdoc check was red** (J329), and had been before this milestone.
  `disabled`'s module docs link to `crate::transparent`, which is `pub(crate)`, so
  `RUSTDOCFLAGS=-D warnings` refuses it. Found by editing that header and running the tool;
  the routine check is clippy and the tests, and neither `fmt` nor `cargo doc` is in it.

- **Dark `secondary_container` sat below the disabled fill it has to beat** (J329). Tone 22
  where the reference puts it at 30 and where the light scheme's already sat, which is why a
  slider's rail and a selected segment read as unavailable. Only checkable once the blend
  above was resolved.

- **A slider's inactive rail was on the wrong container role** (J328). The reference's M3
  slider uses `secondaryContainer` for it, not a surface container — a surface container
  sits by definition a few tones from the surface it lies on, which is where the disabled
  wash lands. Milestone 325 had moved the rail off `outline`, which was right, and onto
  `surface_container_high`, which was half a memory. Visible in light; it does not close
  the gap the roadmap reported, because that gap is the blend space (above).

- **A tap on a dismissible row did nothing** (J327). Found on a device: a task row's avatar
  opened nothing. The hit box was fine — a probe found the target exactly where it is drawn
  — and the gesture arena was the whole of it. A press on such a row prepares a swipe, which
  engages only past a threshold; the release told a tap from a drag against a list of the
  gestures that work that way, and the swipe was the one variant missing from it. So the
  press was recorded, the release returned early, and the widget under the finger was never
  told. It shows when the list is too short to scroll — a scrolling list claims the press
  first, and `Scroll` *was* in the list — and, reading the routing, on every pointer click
  whatever the list was doing. The list is now a named function, `gesture_was_a_tap`, with
  all five variants and a test that pins both directions.

- **An overlay was drawn over the screen that replaced its own** (J326). Found on a device:
  an app bar's overflow menu, left open while choosing an item that navigates, stayed on top
  of the incoming screen. `process_overlays` runs after both screens and paints above the
  whole window, and the transition is parallaxed — the outgoing screen travels only 30 % of
  the width — so the anchor never left and nothing corrected it. A screen on its way out now
  takes its floating layers with it.

- **An overlay whose anchor had left the window was dragged back into it** (J326). The
  auto-flip that keeps a menu inside the window assumed the window was showing the anchor.
  A portal in a horizontally scrolled row hits this with no navigator involved, and the
  window-wide dismissal barrier it left behind would swallow the next press anywhere.

- **The demo's overflow menu survived a navigation** (J326), so it reappeared on returning
  to the screen. `Msg::Push` dismissed the drawer and the popup menu and missed this one.
  Scoped to the messages that navigate — the toggles in that menu want it to stay open.


- **A live outline was the same colour as a disabled one** (J325), in both shipped palettes.
  Open since milestone 320, which wrote it up rather than weaken an assertion to something it
  would pass, and promoted by 324's device pass. `outline_variant` sat **2.2 tones** from the
  12 % disabled composite in dark and **0.5** in light; the reference separates them by about
  ten, and separates `outline` by nearly forty. `from_seed` already placed both roles at the
  reference's tones — only the hand-written schemes had drifted, downward — so the fix is
  those tones taken from each palette's own neutral-variant family. Milestone 320's absent
  assertion is now in `chip.rs`.

- **Separators were painted in the control-edge colour** (J325). Thirteen `theme.border`
  call sites — a chart's gridlines and axes, a navigation bar's hairline, a rail's dividers, a
  drawer's edge, a kanban column's frame, a collapsible's border — are separators, which the
  reference paints in `outline_variant`. Invisible at the old tone; they would have shouted at
  the new one.

- **A slider's rail is a track, not an edge** (J325). It was on `outline` and moved to a
  container tone, as the reference has it.

- **`cargo test --workspace --release` could never pass** (J325). `ReloadWatcher::new`
  refuses outside a debug build, and its test asserted the debug branch unconditionally,
  so the release suite failed in `frus-shell` before it ever reached the widgets or the
  goldens. Found while verifying this milestone, not by looking for it.

- **Outlined text fields had no visible outline** (J325). Not the defect this milestone set
  out to fix, and the largest visible change in the sixty-two goldens that moved: a field, an
  outlined button and a checkbox ring were all drawn at tone 32 on a tone-13 surface.

- **A disabled button flattened too far** (J324). `Button` gave **every** variant a 12 %
  container and dropped the outlined variant's outline. The reference is explicit that a
  text button's background is transparent in every state and that an outlined one keeps its
  outline at 12 % with no fill. A disabled button loses its **accent**, not its **shape** —
  which is what lets a group of buttons carrying a selection through their variants (a page
  strip) still show which one is current. `IconButton` had the same missing outline: a
  disabled stepper was drawn as two bare glyphs with no buttons around them.

- **A disabled rating lost its score** (J324), and a disabled tab bar kept its accent.
  Flattening every star to one grey says nothing about how many are lit, so a lit star takes
  the mark opacity and an unlit one the container's; the tab **indicator** was still painted
  in the accent under labels that had already flattened. Both found by reading the golden,
  neither by a test.

- **A bound you can see** (J324). `Stepper` clamped at its range ends, so "+" at the top
  stayed live and emitted the value already showing; `Pagination` built its end arrows
  without a message but left them painted as full buttons that Tab stopped on — its own
  comments said "disabled" and only the code did not. Both are disabled at their bounds now.

- **`Box<dyn Widget>` was two hooks short** (J324): `build_themed` and `repaint_boundary`
  were not forwarded, so a boxed `ThemeBuilder` asked to build did nothing and a boxed
  repaint boundary reported that it was not one. Latent — every walk in the framework takes
  `&dyn Widget` and dispatches virtually — and now guarded by the same source-reflection
  test that guards the transparent-wrapper macro.

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
