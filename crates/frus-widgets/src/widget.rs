//! The [`Widget`] trait, generic over the message type emitted on interaction.

use frus_core::{Rect, Scene, SemanticsProperties, Size};
use frus_layout::Style;

use crate::interaction::{Cursor, Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
use crate::theme::Theme;

/// A slot built on demand: the closure a `Table` header, a `Table` row or a `Kanban`
/// column takes for a cell that holds a **widget** rather than a string. It is called
/// again on every rebuild, so it hands back a fresh widget each time.
pub type CellFn<Msg> = Box<dyn Fn() -> Box<dyn Widget<Msg>>>;

/// Axis along which a **reorderable** widget moves while dragging: `Table` columns slide
/// **horizontally** (the default), `Kanban` cards **vertically**. The shell adapts the drag
/// preview (ghost direction, insertion indicator) to this axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReorderAxis {
    /// Horizontal reordering (columns) — the default.
    #[default]
    Horizontal,
    /// Vertical reordering (stacked cards).
    Vertical,
}

/// The **sizing** half of a style: the fields that decide how big a box is, and none
/// of the ones that decide what happens inside it.
///
/// For a wrapper that nests its child ([`crate::Draggable`], [`crate::Hero`]) and wants
/// the box its child would have had. Copying the *whole* style would inset that child
/// twice — once by the wrapper's padding, then again by the child's, which is the same
/// number.
pub(crate) fn sizing_of(style: Style) -> Style {
    Style {
        width: style.width,
        height: style.height,
        min_width: style.min_width,
        min_height: style.min_height,
        max_width: style.max_width,
        max_height: style.max_height,
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis,
        aspect_ratio: style.aspect_ratio,
        ..Default::default()
    }
}

/// A widget: a composable interface element.
///
/// `Msg` is the application message type emitted on interaction (a message-passing
/// model, in the Elm/iced style).
/// What the paint walk knows and a widget does not, at the moment its layer filter is
/// asked for.
///
/// Both fields are there for the same reason: they are properties of *where the widget
/// turned out to be*, not of what it was built with. A mask is written in fractions of
/// a box, and the box is only a place on screen once layout has run; a shared backdrop
/// belongs to the nearest enclosing group, and a widget cannot see its own ancestors.
#[derive(Clone, Copy, Debug)]
pub struct FilterContext {
    /// The widget's own box, on screen.
    pub box_rect: frus_core::Rect,
    /// The identity of the nearest enclosing [`crate::BackdropGroup`], if any — the
    /// key a backdrop asking to be shared should use.
    pub backdrop_group: Option<u64>,
}

/// The axes a widget asks to **fill**: on each, it takes the room its parent leaves it
/// rather than shrink-wrapping its children.
///
/// # Why this is asked and not declared
///
/// `width: 100%` looks like the same thing and is not. A percentage resolves against the
/// parent's **resolved** width, which a parent that shrink-wraps has not got yet — it is
/// waiting on this very child to find out how wide it should be. Both are "full width" in
/// English, and only one can be computed in time:
///
/// | | what it needs | known |
/// |---|---|---|
/// | `width: 100%` | the parent's own width | on the way back **up** |
/// | a fill request | the room being offered | on the way **down** |
///
/// Fifteen widgets in this crate said it the first way and every one of them collapsed
/// under a plain column (milestone 404). Asking is the only phrasing the layout can answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FillAxes {
    /// Take the width the parent offers.
    pub horizontal: bool,
    /// Take the height the parent offers.
    pub vertical: bool,
}

impl FillAxes {
    /// Shrink-wrap on both axes — what a widget with no opinion answers.
    pub const NONE: Self = Self {
        horizontal: false,
        vertical: false,
    };
    /// Take the width offered; hug the content vertically.
    pub const WIDTH: Self = Self {
        horizontal: true,
        vertical: false,
    };
    /// Take the height offered; hug the content horizontally.
    pub const HEIGHT: Self = Self {
        horizontal: false,
        vertical: true,
    };
    /// Take everything offered — a full-screen shell.
    pub const BOTH: Self = Self {
        horizontal: true,
        vertical: true,
    };

    /// The axes asked for, as one flex direction, or `None` where that cannot be said —
    /// neither axis, or both. For the walk, which resolves each axis separately anyway.
    pub fn single(self) -> Option<frus_layout::FlexDirection> {
        match (self.horizontal, self.vertical) {
            (true, false) => Some(frus_layout::FlexDirection::Row),
            (false, true) => Some(frus_layout::FlexDirection::Column),
            _ => None,
        }
    }

    /// Whether one axis is asked for.
    pub fn wants(self, horizontal: bool) -> bool {
        if horizontal {
            self.horizontal
        } else {
            self.vertical
        }
    }

    /// The axis a [`frus_layout::FlexDirection`] names, and nothing else.
    pub fn along(direction: frus_layout::FlexDirection) -> Self {
        if direction.is_horizontal() {
            Self::WIDTH
        } else {
            Self::HEIGHT
        }
    }
}

pub trait Widget<Msg> {
    /// Layout style (handed to `frus-layout`).
    fn style(&self) -> Style;

    /// The widget's children (possibly empty).
    fn children(&self) -> &[Box<dyn Widget<Msg>>];

    /// Paints the widget's own decoration, according to its status (hover / focus /
    /// cursor / selection) and the current theme.
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene);

    /// Message to emit on click (`None` = not clickable).
    fn on_click(&self) -> Option<Msg>;

    /// **Stable** identity key (independent of the position among siblings).
    /// `None` = positional identity. See [`crate::Keyed`].
    fn key(&self) -> Option<u64> {
        None
    }

    /// **Short** name of the widget — for the inspector and diagnostic dumps.
    /// Defaults to the name of the concrete type, without module path or
    /// generics (each implementation gets its own monomorphised copy of this
    /// default body). Transparent wrappers (`Box`, [`crate::Keyed`]…) delegate
    /// to their content.
    fn debug_name(&self) -> &'static str {
        short_type_name::<Self>()
    }

    /// The widget's **accessibility annotation** (role, label, value, state),
    /// exposed to assistive technologies through the AccessKit tree. `None` =
    /// no semantics of its own (a layout container — its children, though,
    /// may have some). See [`frus_core::SemanticsProperties`].
    fn semantics(&self) -> Option<SemanticsProperties> {
        None
    }

    /// What this widget says about its **child's** subtree, when it knows something the
    /// child cannot know about itself — the [`crate::Semantics`] wrapper, and nothing else
    /// so far.
    ///
    /// [`Widget::semantics`] is a widget answering for itself and covers the ordinary
    /// case. This covers the one it cannot: a caller handed an already-built widget, with
    /// no way in. Read in one place in the walk, the way a
    /// [`ModalBarrier`](crate::barrier::ModalBarrier) is — the subtree is walked exactly as
    /// usual and what it produced is reconciled afterwards, so a widget deep inside
    /// annotates itself without knowing anything is speaking for it.
    fn describes(&self) -> Option<crate::semantics::Description> {
        None
    }

    /// Applies a key to the focused widget: mutates the edit state
    /// (cursor/selection) and returns a message if the **value** changes.
    fn on_edit(&self, _edit: &mut Edit, _key: &Key) -> Option<Msg> {
        None
    }

    /// **Positional** click: a message emitted according to where the click landed (local
    /// coordinates from the widget's top-left corner, within the `width × height` box), taking
    /// **priority** over [`on_click`](Self::on_click). For clickable sub-regions — e.g. the
    /// **suffix** icon of a [`crate::TextField`] (clear / reveal) or a chart **point**
    /// (milestone 221). `None` (the default) = no sub-region; the shell falls back to `on_click`.
    fn positional_click(
        &self,
        _local_x: f32,
        _local_y: f32,
        _width: f32,
        _height: f32,
    ) -> Option<Msg> {
        None
    }

    /// Shape of the **system cursor** wanted at the **local** position `(local_x, local_y)` inside
    /// the widget's `width × height` box (milestone 205): `Some(Cursor::Pointer)` over a clickable
    /// sub-region (a suffix icon…), `None` (the default) = no opinion, the shell keeps the default
    /// cursor. Does not affect clicking; this is purely the pointer's appearance on hover.
    fn cursor_icon(
        &self,
        _local_x: f32,
        _local_y: f32,
        _width: f32,
        _height: f32,
    ) -> Option<Cursor> {
        None
    }

    /// Cursor index matching a local position (px from the widget's top-left
    /// corner) — for placing the caret on click. `local_y` picks the **line** in a
    /// multi-line field (ignored on a single line).
    ///
    /// `width` = the field's width, `scroll_cursor` = the cursor from which to recompute
    /// the current **scroll** (the same one the render used), so that a click lands right
    /// even when the text is scrolled. `None` = not a text field.
    fn cursor_at(
        &self,
        _local_x: f32,
        _local_y: f32,
        _width: f32,
        _scroll_cursor: usize,
    ) -> Option<usize> {
        None
    }

    /// SingleChildScrollView metrics of a **multi-line** field, for a widget width and a cursor:
    /// `(content height, visible box height, caret top, caret height)`, in content
    /// space (px). The shell uses them to register the scrollable region and to keep
    /// the caret in view. `None` otherwise.
    fn text_metrics(&self, _width: f32, _cursor: usize) -> Option<(f32, f32, f32, f32)> {
        None
    }

    /// Frame (input box) of a **multi-line** field within its `rect` — the area where
    /// the text scrolls, below the label. Used to place the scrollbar and the
    /// scrollable region exactly on the box. `None` otherwise.
    fn text_viewport(&self, _rect: crate::Rect) -> Option<crate::Rect> {
        None
    }

    /// Moves the caret vertically in a multi-line field and returns
    /// `(new index, visual column used)`.
    ///
    /// - `down`: down (`true`) or up. `page`: jump by a **page** (the visible
    ///   height) rather than by a **line**.
    /// - `goal_x`: remembered target visual column (px) to preserve while crossing
    ///   shorter lines; `None` = start from the current column. The returned value
    ///   is the column to remember again for the next jump.
    /// - **Line**: `None` if already on the first/last line (or not multi-line) —
    ///   the shell then moves the focus instead.
    /// - **Page**: clamped to the field (it never leaves it); `None` only when not
    ///   multi-line.
    fn caret_vertical(
        &self,
        _width: f32,
        _cursor: usize,
        _down: bool,
        _page: bool,
        _goal_x: Option<f32>,
    ) -> Option<(usize, f32)> {
        None
    }

    /// The currently selected text (for copy/cut).
    fn selected_text(&self, _edit: &Edit) -> Option<String> {
        None
    }

    /// The field's **whole** value, if this widget is a text field — to give the
    /// IME its input context (suggestions). `None` otherwise.
    fn text_value(&self) -> Option<&str> {
        None
    }

    /// Range `(start, end)` of the word around the given index (for double-click).
    fn word_at(&self, _index: usize) -> Option<(usize, usize)> {
        None
    }

    /// Keystrokes this subtree binds to **intents** — see [`crate::Shortcuts`].
    fn shortcut_bindings(&self) -> &[(crate::shortcuts::KeyStroke, crate::shortcuts::Intent)] {
        &[]
    }

    /// Keystrokes this subtree binds **straight to messages** — see
    /// [`crate::CallbackShortcuts`].
    fn shortcut_callbacks(&self) -> &[(crate::shortcuts::KeyStroke, Msg)] {
        &[]
    }

    /// Intents this subtree **answers** — see [`crate::Actions`].
    fn action_bindings(&self) -> &[(crate::shortcuts::Intent, Msg)] {
        &[]
    }

    /// Intents this subtree **watches** without answering — see
    /// [`crate::ActionListener`].
    fn action_listeners(&self) -> &[(crate::shortcuts::Intent, Msg)] {
        &[]
    }

    /// Every keystroke reaching this subtree, handed to a closure — see
    /// [`crate::KeyboardListener`]. `None` (the default) is not a listener.
    #[allow(clippy::type_complexity)]
    fn on_keystroke(
        &self,
    ) -> Option<std::rc::Rc<dyn Fn(crate::shortcuts::KeyStroke) -> Option<Msg>>> {
        None
    }

    /// If `true`, the widget can take keyboard focus (click or Tab).
    fn focusable(&self) -> bool {
        false
    }

    /// If `false`, **nothing inside this subtree** can take focus — the reference's
    /// `ExcludeFocus`. A dimmed panel behind a sheet is still drawn and still measured;
    /// it simply stops being somewhere Tab can land.
    fn descendants_focusable(&self) -> bool {
        true
    }

    /// If `true`, this subtree's focus stops are **skipped by Tab** while remaining
    /// focusable by a click — the reference's `ExcludeFocusTraversal`. The two are
    /// separate questions: a toolbar button may be reachable with the pointer and still
    /// not belong in the keyboard's order.
    fn focus_skip_traversal(&self) -> bool {
        false
    }

    /// A **traversal order** for this subtree's focus stops, smallest first — the
    /// reference's `NumericFocusOrder`. `None` leaves them in tree order, which is where
    /// everything sits until someone says otherwise.
    fn focus_order(&self) -> Option<f32> {
        None
    }

    /// If `true`, this subtree is a **traversal group**: an order set inside it is
    /// resolved among its own members and nowhere else, so a reordered dialog does not
    /// reshuffle the page behind it.
    fn focus_group(&self) -> bool {
        false
    }

    /// If `true`, the widget draws its focus indicator **itself** (the driver then
    /// does not stroke the generic ring). E.g. `TextField`.
    fn draws_own_focus(&self) -> bool {
        false
    }

    /// If `true`, the widget responds to pointer dragging (sliders, handles).
    fn draggable(&self) -> bool {
        false
    }

    /// Message produced while dragging, `fraction` being the relative horizontal
    /// position (`0.0..=1.0`) within the widget's bounds.
    fn on_drag(&self, _fraction: f32) -> Option<Msg> {
        None
    }

    /// Message produced when a value drag **begins** — on the press, before the
    /// first [`on_drag`](Self::on_drag).
    ///
    /// With [`on_drag_end`](Self::on_drag_end) it brackets the stream: `on_drag` fires
    /// on every pixel of the movement, and an application that saves to disk, seeks a
    /// video or asks the network on each of those does it sixty times a second. The
    /// bracket is what lets it do the cheap thing while the finger is down and the
    /// expensive one when it lifts — and without an end there is no moment at which
    /// to do it at all.
    ///
    /// Not to be confused with [`on_dropped`](Self::on_dropped), which belongs to
    /// drag-and-**drop**: that one moves a thing, this one changes a number.
    fn on_drag_start(&self, _fraction: f32) -> Option<Msg> {
        None
    }

    /// Message produced when a value drag **ends** — on the release, with the
    /// fraction the pointer finished on.
    ///
    /// See [`on_drag_start`](Self::on_drag_start).
    fn on_drag_end(&self, _fraction: f32) -> Option<Msg> {
        None
    }

    /// Message produced by a horizontal drag expressed as a **delta**: `dx` is the
    /// movement (logical px) since the last event. For handles that **accumulate**
    /// (column resizing), unlike [`on_drag`](Self::on_drag) (an absolute fraction,
    /// e.g. a slider). The shell tries it **before** `on_drag`; a widget implements
    /// only one of the two.
    fn on_drag_delta(&self, _dx: f32) -> Option<Msg> {
        None
    }

    /// If this widget is a **shared element**, its tag. Two heroes with the same tag on
    /// the two sides of a route transition are one thing in two places.
    /// See [`crate::Hero`].
    fn hero_tag(&self) -> Option<u64> {
        None
    }

    /// If this widget can be **picked up** and dropped elsewhere, what it carries.
    /// See [`crate::Draggable`].
    fn drag_payload(&self) -> Option<u64> {
        None
    }

    /// Does this item lift on a **long press** rather than on the first movement?
    /// The answer inside a scrollable, where a plain drag belongs to the scroll.
    fn drag_needs_long_press(&self) -> bool {
        false
    }

    /// How solid the item left behind looks while a copy of it is being dragged.
    fn drag_ghost_opacity(&self) -> f32 {
        1.0
    }

    /// Message sent when a drag of this item ends, `true` if a target took it.
    fn on_dropped(&self, _accepted: bool) -> Option<Msg> {
        None
    }

    /// Is this widget a **drop target**? See [`crate::DragTarget`].
    fn drop_zone(&self) -> bool {
        false
    }

    /// Would this target take `payload`? Only asked of a drop target.
    fn accepts_drag(&self, _payload: u64) -> bool {
        true
    }

    /// Message sent when an accepted item is dropped on this target.
    fn on_drop(&self, _payload: u64) -> Option<Msg> {
        None
    }

    /// If this widget is a **reorderable header**, its column index. The shell uses
    /// it to identify the **source** column (on press) and the **target** column
    /// (under the pointer on release) of a reordering drag.
    fn reorder_index(&self) -> Option<usize> {
        None
    }

    /// Message emitted when this header (the source) is **dropped** on column `to`.
    /// The widget knows its own index and the `on_reorder(from, to)` callback.
    fn on_reorder(&self, _to: usize) -> Option<Msg> {
        None
    }

    /// Movement **axis** of this reorderable (default [`ReorderAxis::Horizontal`]): the shell
    /// orients the drag preview (ghost + insertion indicator) accordingly. `Kanban` cards
    /// return [`ReorderAxis::Vertical`].
    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Horizontal
    }

    /// Can this reorderable be **picked up** as a drag source? `true` by default for every
    /// reorderable. A **target-only** slot — the drop zone at the end of a Kanban column —
    /// returns `false`: a card can be **dropped** there, not **lifted** from it (otherwise the
    /// drag would raise an empty ghost that moves nothing).
    fn reorder_draggable(&self) -> bool {
        true
    }

    /// Text to **announce** to the screen reader when this widget is **activated** (mouse
    /// click or Enter/Space) — a live region reads it out loud. Describes the effect the
    /// activation **produced** ("Sorted by Name ascending", "All rows selected"), for the
    /// blind user who cannot perceive the visual change. `None` = nothing to announce
    /// (the default).
    fn announce(&self) -> Option<String> {
        None
    }

    /// If `true`, the widget is a **stack**: its children are superimposed layers
    /// (the same box), rendered in order (the last one on top).
    fn stack(&self) -> bool {
        false
    }

    /// If `true`, the widget animates **continuously** (driven by time, not by a
    /// target): the framework keeps redrawing. E.g. `CircularProgressIndicator`.
    fn continuous(&self) -> bool {
        false
    }

    /// If `true`, this widget is a **repaint boundary**: its subtree is cached
    /// (primitives + interaction maps) and **reused as is** as long as its geometry
    /// and the interaction state of its descendants do not change — a widget
    /// animating elsewhere no longer forces it to repaint.
    /// See [`crate::Container::repaint_boundary`] and `paintcache.rs`.
    fn repaint_boundary(&self) -> bool {
        false
    }

    /// If the widget is a scrollable container, returns its content.
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }

    /// If the widget is a **virtualised list**, returns its description (item count,
    /// height, factory). Only the visible items are built.
    fn virtual_list(
        &self,
        _viewport: frus_core::Size,
    ) -> Option<crate::list::VirtualList<'_, Msg>> {
        None
    }

    /// If the widget is a **paged view**, returns its description (page count,
    /// axis, factory). Like a virtualised list, only the visible pages are built.
    fn page_view(&self) -> Option<crate::pageview::PagedView<'_, Msg>> {
        None
    }

    /// Message sent by a paged view when the page on screen changes.
    fn on_page_changed(&self, _page: usize) -> Option<Msg> {
        None
    }

    /// If the widget takes one axis from its content's **preferred** size, returns
    /// that axis and the step its measurement is rounded up to. See
    /// [`crate::Intrinsic`].
    fn intrinsic(&self) -> Option<(crate::constraints::IntrinsicAxis, Option<f32>)> {
        None
    }

    /// If the widget lays its child out to constraints of **its own**, which the
    /// child may exceed, returns them. See [`crate::OverflowBox`].
    fn overflow_box(&self) -> Option<crate::constraints::Overflow> {
        None
    }

    /// If the widget builds its content **from its actual box**, returns the
    /// `size → widget` factory. The content is built on the fly: no retained state
    /// and no overlay (like a virtualised list item).
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        None
    }

    /// SingleChildScrollView axis (or axes), for a scrollable container.
    fn scroll_axis(&self) -> crate::scroll::Axis {
        crate::scroll::Axis::Vertical
    }

    /// The edge and fling behaviour of a scrollable container, when this widget
    /// wants one in particular. `None` — the usual answer — leaves the choice to
    /// the application, which defaults to what the platform does.
    fn scroll_physics(&self) -> Option<crate::physics::ScrollPhysics> {
        None
    }

    /// What the **software keyboard** should be, when this widget is the focused
    /// editable one.
    ///
    /// Only ever asked of the field that has focus — the platform layer finds it the
    /// same way it finds the caret — so a widget that is not editable never has to
    /// answer, and the default is the ordinary text keyboard with a *Done* key.
    ///
    /// See [`crate::ime`] for what the two halves mean and why the mapping to the
    /// platform's numbers is tested rather than trusted.
    fn ime(&self) -> crate::ime::Ime {
        crate::ime::Ime::default()
    }

    /// If the widget is a portal, returns its floating content and its placement.
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        None
    }

    /// Message emitted when the scrim of a modal is clicked (dismissal), if any.
    fn overlay_dismiss(&self) -> Option<Msg> {
        None
    }

    /// The colour of the scrim behind this widget's **modal** overlay, when it wants one
    /// in particular. `None` — the usual answer — leaves it to the scheme's `scrim`
    /// role at the framework's own opacity.
    ///
    /// The alpha is the caller's: a scrim is the one colour whose *transparency* is the
    /// whole point, so a fully opaque value here means an opaque scrim, and
    /// [`Color::TRANSPARENT`](frus_core::Color::TRANSPARENT) means none at all — an
    /// overlay that darkens nothing, which the reference reaches by the same route.
    fn overlay_scrim(&self, _theme: &crate::theme::Theme) -> Option<frus_core::Color> {
        None
    }

    /// If `true`, this widget's **anchored** overlay (a menu…) **traps focus**: Tab and the
    /// arrow keys cycle within its focusables while it is open (the keyboard pattern of menus).
    /// Modal (scrimmed) overlays already trap by default; a tooltip or an autocomplete list
    /// (focus stays on the field) do **not**. Default: `false`.
    fn overlay_traps_focus(&self) -> bool {
        false
    }

    /// Target of the widget's own animated value (e.g. `1.0` for a switch that is on,
    /// `0.0` for off). The runtime drives the retained value towards this target and
    /// hands it back through `Status::value`. `None` = no animated value.
    fn anim_target(&self) -> Option<f32> {
        None
    }

    /// Duration (seconds) of the animated value's transition (`anim_target`).
    /// Default: the framework's standard duration.
    fn anim_duration(&self) -> f32 {
        crate::runtime::ANIM_DURATION
    }

    /// Easing curve of the animated value's transition (`anim_target`).
    /// Default: linear.
    fn anim_curve(&self) -> frus_core::Curve {
        frus_core::Curve::Linear
    }

    /// If the widget is an **opacity group**, returns the (target) opacity `[0,1]`
    /// applied to its whole subtree **as one**: the paint walk wraps its primitives
    /// in a composited layer at that opacity — no double-blending where they
    /// overlap. `None` = not a group. Combined with `anim_target`, the opacity
    /// **animates** (a group fade).
    fn opacity_group(&self) -> Option<f32> {
        None
    }

    /// **Target** background colour of an animated background
    /// (`Container::animated_color`): the runtime tweens it (through
    /// `anim_duration`/`anim_curve`) and hands the interpolated value back in
    /// `Status::anim_color`. `None` = no animated colour.
    fn anim_color(&self) -> Option<frus_core::Color> {
        None
    }

    /// **Target** size of an animated size (`Container::animated_size`): the runtime
    /// tweens it (through `anim_duration`/`anim_curve`) and the interpolated size is
    /// injected **at layout** (see `effective_style`). `None` = fixed size.
    fn anim_size(&self) -> Option<Size> {
        None
    }

    /// **Target** corner radius of an animated radius (`Container::animated_radius`):
    /// the runtime tweens it and hands the interpolated value back in
    /// `Status::anim_radius`. `None` = fixed radius.
    fn anim_radius(&self) -> Option<frus_core::BorderRadius> {
        None
    }

    /// **Target** padding of an animated padding (`Container::animated_padding`):
    /// the runtime tweens it and the interpolated padding is injected **at layout**
    /// (see `effective_style`). `None` = fixed padding.
    fn anim_padding(&self) -> Option<frus_core::Insets> {
        None
    }

    /// **Alignment** of the single child within the box: the walk offsets the child
    /// through the free space according to the alignment's fractions, resolved
    /// against the reading direction (physical or directional — see
    /// [`frus_core::AlignmentGeometry`]). Being continuous, a `Tween<Alignment>`
    /// read in `view()` slides the child. `None` = default flex placement.
    fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
        None
    }

    /// **Paint offset** `(dx, dy)` applied to the widget's whole subtree: the render
    /// and the hit-test are offset **without touching layout** (siblings do not move,
    /// the child may overflow its box). Continuous → a `Tween` read in `view()` slides
    /// the subtree. `None` = no offset. See [`crate::Transform`].
    fn transform_translate(&self) -> Option<(f32, f32)> {
        None
    }

    /// **Paint scaling** `(sx, sy, pivot)` of the whole subtree: the render and the
    /// hit-test are scaled — per axis — around the `pivot` (a
    /// [`frus_core::Alignment`] within the box), **without touching layout**. Stays
    /// axis-aligned (a scaled rect is still a rect). `None` = no scaling.
    /// See [`crate::Transform`].
    fn transform_scale(&self) -> Option<(f32, f32, frus_core::Alignment)> {
        None
    }

    /// **Paint rotation** `(angle, pivot)` of the whole subtree: the subtree is
    /// painted rotated by `angle` radians (clockwise) around the `pivot` (a
    /// [`frus_core::Alignment`] within the box), **without touching layout**. The
    /// render goes through a rotated composited layer; the hit-test counter-rotates
    /// the point. `None` = no rotation. See [`crate::Transform`].
    fn transform_rotate(&self) -> Option<(f32, frus_core::Alignment)> {
        None
    }

    /// **Shape clip** of the whole subtree: the paint walk wraps its primitives in a
    /// composited layer whose [`frus_core::ClipShape`] (rounded corners, ellipse)
    /// modulates the alpha — whatever falls outside the shape is erased, edges
    /// antialiased. `None` = no shape clip. See [`crate::ClipRRect`],
    /// [`crate::ClipOval`].
    fn clip_shape(&self) -> Option<frus_core::ClipShape> {
        None
    }

    /// The widget's style **once the theme has had its say** — the layout half of the
    /// `caller ?? theme ?? framework` chain.
    ///
    /// Defaults to [`Widget::style`], which is what almost every widget wants: only the
    /// ones whose *size or spacing* a theme can set need to override this. It exists
    /// because `style` has no theme and cannot be given one without touching every
    /// implementation; a theme that could only reach `paint` would be able to recolour a
    /// divider but not make one thin, which is the setting an application actually asks
    /// for (milestone 309).
    ///
    /// A **transparent wrapper must forward this** alongside `style`, or its child's
    /// themed size is silently dropped for the unthemed one.
    fn style_themed(&self, _theme: &Theme) -> Style {
        self.style()
    }

    /// The theme this widget imposes on **itself and its subtree**, given the one it
    /// inherits. `None` — the default — means "whatever came down from above", which is
    /// what every widget but [`crate::Themed`] answers.
    ///
    /// This is the third and last thing a theme needed to be usable: milestone 309 gave
    /// it per-widget defaults, and this makes those defaults *scoped*. A dark panel on a
    /// light page is one theme swapped for a subtree, not a colour written out by hand at
    /// every call site inside it.
    ///
    /// It is honoured by **layout as well as paint** — the walk and the layout pass swap
    /// the ambient theme on the way down and restore it on the way out — so a subtree
    /// theme can change a divider's height, not only its colour. A deferred overlay
    /// (a dialog, a drawer, a tooltip) is painted long after the walk has left the node
    /// that declared it, so it **carries the theme it was declared under** rather than
    /// the root's.
    ///
    /// Boxed because a `Theme` is a page and a half of tokens and this is asked of
    /// **every node, every frame**: an `Option<Box<_>>` costs a word on the stack of a
    /// recursion as deep as the tree, and the allocation happens only where a subtree
    /// actually claims a theme.
    fn theme_override(&self, _inherited: &Theme) -> Option<Box<Theme>> {
        None
    }

    /// The **surface** this widget imposes on itself and its subtree, given the one it
    /// inherits. `None` — the default — means "whatever came down from above", which is
    /// what every widget but [`MediaScope`](crate::MediaScope) answers.
    ///
    /// The counterpart of [`Self::theme_override`], and it exists for the same reason one
    /// milestone later: a surface could until now only be narrowed **where a widget is
    /// constructed** (`SafeArea::build`), and the widget that knows what to narrow is often
    /// not the one doing the constructing. A shell hands its app bar's slot the description
    /// that slot should believe in; the bar then decides for itself whether to consume the
    /// status bar, which is how the reference splits that job across two widgets.
    ///
    /// Applied by the layout walk on the way down and held for the subtree, so anything
    /// reading [`MediaQuery::of`](crate::MediaQuery::of) below this node sees it — including
    /// a subtree deferred until [`Self::build_themed`], which runs after the swap.
    /// [`crate::build_deferred`], the relayout fingerprint and the paint walk make the same
    /// swap; the four staying in step is what keeps the cache and the picture honest.
    fn media_override(&self, _inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        None
    }

    /// The **shell** this widget imposes on its subtree: what a
    /// [`Scaffold`](crate::Scaffold) knows about itself and its slots do not.
    ///
    /// The third of the inherited things, after the theme and the surface, and it exists
    /// for the same reason as the second: a slot is handed to a shell **already built**, so
    /// the widget that knows there is a drawer is never the one that has to draw the button
    /// for it. The reference reads it from the context (`Scaffold.of`), which is how a bar
    /// with no `leading` grows a menu button on a screen that has a menu and stays empty on
    /// one that does not (`app_bar.dart:1010`).
    ///
    /// `None` — the default — means "whatever came down from above", which is what every
    /// widget but [`ScaffoldScope`](crate::ScaffoldScope) answers. Applied by the same four
    /// walks as [`Self::media_override`], and for the same reason: an
    /// [`AppBar`](crate::AppBar) is composed in [`Self::build_themed`], which runs after the
    /// swap, and the relayout fingerprint has to see what the composition saw.
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        None
    }

    /// Builds this widget's subtree **from the ambient theme**, for the widgets that
    /// defer that decision — see [`ThemeBuilder`](crate::ThemeBuilder). Does nothing by
    /// default, which is what all but a handful of widgets want.
    ///
    /// [`Self::style_themed`] and [`Self::paint`] let a theme decide how a widget
    /// **looks**. This is for the questions a theme answers *earlier* than that: whether
    /// an application bar centres its title decides which children exist and in what
    /// order, and by the time anything is being painted the composition has already been
    /// made. A widget assembled by a builder — `AppBar::new(…).build()` — never sees a
    /// theme at all.
    ///
    /// Called by the layout pass on the way down, **before** [`Self::children`] is read,
    /// under the theme of the subtree the node sits in. It takes `&self` like every other
    /// hook, so an implementor needs interior mutability; that is safe here because a
    /// widget tree is rebuilt from `view` rather than mutated, so "once per instance" and
    /// "once per frame" are the same thing.
    fn build_themed(&self, _theme: &Theme) {}

    /// If the widget takes **ink** — the splash a tap leaves on a material surface —
    /// the shape and colour to splash in. `None` = no ink, which is the default: a
    /// widget has to ask.
    ///
    /// The walk paints the ripples the runtime holds for this widget directly over
    /// this widget's own `paint` and **under its children**, which is where a material
    /// surface puts them. A widget that draws its own content (a [`crate::Button`]
    /// paints its label itself) therefore gets the ink *over* that content — at the
    /// splash's alpha, a tint rather than a veil. See [`crate::InkWell`].
    fn ink(&self, _theme: &Theme) -> Option<crate::ink::InkStyle> {
        None
    }

    /// A decoration painted **over this widget's children** rather than behind them:
    /// the reference's `foregroundDecoration`.
    ///
    /// It is the one place in the walk where a widget paints after its own subtree,
    /// and it exists because there is no other way to say it. A border over a
    /// photograph, a wash across a tile that is disabled, a sheen over a card — every
    /// one of them is a decoration whose whole point is that the content does not
    /// cover it, and behind the content is exactly where [`Widget::paint`] puts one.
    ///
    /// The box is this widget's own, the same one `paint` is given. Inside any layer
    /// this widget asks for: an opacity group fades it with everything else, a
    /// transform carries it, and a shape clip holds it to the shape — which for a
    /// decoration wearing that same radius changes nothing.
    ///
    /// `None` is the default, and costs the walk one `Option` check per node.
    fn foreground(&self, _theme: &Theme) -> Option<frus_core::BoxDecoration> {
        None
    }

    /// The **pixel effects** this widget applies to its whole subtree: a blur, a
    /// colour transform, a mask, or a filter over what is painted underneath. The
    /// paint walk drains the subtree into a composited layer and hands the filter to
    /// the renderer, exactly as it does for an opacity group or a shape clip.
    ///
    /// The [`FilterContext`] carries what only the walk knows — where the box ended
    /// up, and which backdrop group encloses it. A widget that needs neither ignores
    /// it.
    ///
    /// `None` — the default — means no layer and no cost. See [`crate::ColorFiltered`],
    /// [`crate::ImageFiltered`], [`crate::ShaderMask`], [`crate::BackdropFilter`].
    fn layer_filter(&self, _cx: FilterContext) -> Option<frus_core::LayerFilter> {
        None
    }

    /// `true` when this widget is a **backdrop group**: the backdrops below it that
    /// ask to be shared are filtered once between them. See [`crate::BackdropGroup`].
    fn backdrop_group(&self) -> bool {
        false
    }

    /// The widget's own **text baseline**: the distance from the top of its box down
    /// to the line its letters sit on. `None` — the default — means this widget has no
    /// text of its own, and whatever is inside it answers instead.
    ///
    /// It is what makes a price and its currency, or a heading and the note beside it,
    /// sit on one line rather than merely in one row. See [`crate::Baseline`] and
    /// `Align::Baseline`.
    fn text_baseline(&self, _theme: &Theme) -> Option<f32> {
        None
    }

    /// `true` when this subtree should be **left out** of a parent's baseline
    /// alignment: it has a baseline, and the parent is to pretend it does not. An icon
    /// beside a label, where the label is what the row should line up on. See
    /// [`crate::IgnoreBaseline`].
    fn ignores_baseline(&self) -> bool {
        false
    }

    /// The **axes this widget asks to fill**: on each, it takes the room the parent
    /// leaves it rather than shrink-wrapping its children. [`FillAxes::NONE`] means
    /// shrink-wrap on both.
    ///
    /// `MainAxisSize::Max` on a [`crate::Row`] or a [`crate::Column`] is the reason it
    /// exists, and there the axis is the widget's own main one; a [`crate::TabBar`] asks
    /// for the horizontal one, which is its cross axis; a full-screen shell asks for
    /// [`FillAxes::BOTH`].
    ///
    /// It is a question about the *parent*, which is why it is asked rather than written
    /// into the style: filling means growing when the parent runs the same way and
    /// stretching when it runs across, and a widget cannot know what it was put inside.
    /// The layout walk resolves it, where both are in view. **Declaring `width: 100%`
    /// instead does not work** and cannot be made to — see [`FillAxes`].
    ///
    /// The theme is here for the same reason it is on [`Widget::measure`]: a text asks to
    /// fill because it was told to align inside its box, and a subtree can hand that
    /// alignment down. A hook blind to the theme would leave the one setting that arrives
    /// by inheritance silently doing nothing.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes::NONE
    }

    /// The width below which this widget must not be squeezed **when its parent runs
    /// horizontally** — a line of text that would rather run past the end of a row than
    /// be folded into a column of single words. `None` means squeeze freely.
    ///
    /// It is asked rather than written into the style because the same number means two
    /// different things on the two axes. Down a column a box is *handed* a width and a
    /// floor would refuse it; along a row a box is *asked* how wide it wants to be, and
    /// this is the answer it will not go below. The reference draws the same line: a flex
    /// leaves its inflexible children an unbounded main axis and never squeezes them,
    /// while across it they take the width they are given.
    ///
    /// It is handed the theme for the same reason [`Widget::style_themed`] is: the floor
    /// is a *measurement*, and a text whose size comes from an inherited style measures
    /// differently. A hook that decided a size without seeing the theme would answer for
    /// a font nobody is drawing.
    fn main_axis_floor(&self, _theme: &Theme) -> Option<f32> {
        None
    }

    /// If this container gives all of its children the **shape of a tile**, the
    /// `width / height` ratio to impose on each of them. `None` = each child keeps
    /// its own shape.
    ///
    /// A grid's tiles are the same shape in the reference, and it is the grid that
    /// says so — the tile does not know how wide its column came out. Imposed during
    /// the walk, like the fill request and the shrink grant, because it needs the
    /// children's layout nodes rather than their styles.
    fn tile_shape(&self) -> Option<f32> {
        None
    }

    /// If this is a layer **pinned** against its [`crate::Stack`]'s edges
    /// ([`crate::Positioned`]), what it pins. `None` = an ordinary layer, sized and
    /// placed by the stack's own fit and alignment.
    ///
    /// Read by the stack rather than written into a style: an offset from an edge is not
    /// something the layout engine's box model has a field for, and the number it
    /// resolves to depends on how big the stack came out.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        None
    }

    /// Whether this stack sizes its unpinned layers **loosely** — asking each what it
    /// would like to be — rather than handing each of them the whole box.
    /// See [`crate::StackFit`]. Meaningless unless [`Widget::stack`] is true.
    fn stack_loose(&self) -> bool {
        false
    }

    /// The box this widget asks the scroll region around it to **keep in view**, in its
    /// own coordinates, with a key naming which box it is.
    ///
    /// A tab bar wider than its window is what this is for: the eighth tab is off the
    /// side, and nothing about selecting it would otherwise bring it back. The reference
    /// scrolls its bar to the selected tab, and so does this.
    ///
    /// The **key** is what makes the region act on a change rather than on every frame.
    /// The box moves as the region scrolls, so a region that chased it would pin the
    /// content in place and no finger could move it; the key changes only when the widget
    /// means a different box.
    ///
    /// `centre` asks for the box in the middle of the window rather than merely inside
    /// it — what a tab bar wants, since a selected tab flush against the edge looks like
    /// the end of the row. Clamped either way, so the first tab stays at the start.
    fn keep_visible(&self, _size: Size, _theme: &Theme) -> Option<crate::ui::KeepVisible> {
        None
    }

    /// Whether this scrollable runs **from the far end** ([`crate::SingleChildScrollView::reverse`]):
    /// the content is anchored to the end of the viewport and offsets are counted from
    /// there. Meaningless unless [`Widget::scroll_content`] is `Some`.
    fn scroll_reverse(&self) -> bool {
        false
    }

    /// The insets a scrollable area puts **around its content**, inside the viewport.
    ///
    /// The padding scrolls with the content rather than shrinking the window onto it,
    /// which is the reference's `SliverPadding` and the only reading that is any use:
    /// room at the end of a list is reachable by scrolling to it, room taken out of the
    /// viewport is not. Read by both the scroll branch and the virtualised list.
    fn scroll_padding(&self) -> frus_core::Insets {
        frus_core::Insets::ZERO
    }

    /// If the widget positions its child **by the child's baseline** ([`crate::Baseline`]),
    /// the distance from the top of this box at which that baseline should land.
    /// `None` = not a baseline box.
    fn baseline_target(&self) -> Option<f32> {
        None
    }

    /// If the widget clips its child to an **arbitrary path** (`ClipPath`), returns the
    /// path in **local coordinates** (origin at the box's top-left corner). The walk
    /// offsets it to the screen and wraps it in a layer whose mask (the path) erases
    /// the outside. Takes priority over [`Widget::clip_shape`]. `None` = no path clip.
    /// See [`crate::ClipPath`].
    fn clip_path(&self) -> Option<&frus_core::Path> {
        None
    }

    /// If the widget is a **dismissible item** ([`crate::Dismissible`]), its
    /// configuration for this frame. The shell gives a mostly-sideways drag on it to
    /// the dismissal and a mostly-up-and-down one to the enclosing scrollable. `None` =
    /// not dismissible.
    fn dismissible(&self) -> Option<crate::dismiss::DismissSpec> {
        None
    }

    /// The message a dismissed item dispatches, once it has flown out **and** its gap
    /// has closed. `None` = the swipe is shown but removes nothing.
    fn on_dismissed(&self, _direction: crate::dismiss::DismissDirection) -> Option<Msg> {
        None
    }

    /// If the widget is a **refresh area** ([`crate::RefreshIndicator`]), its configuration for
    /// this frame. Any scrollable inside it routes the movement its physics refuses at
    /// the top edge into the pull, instead of into the overscroll glow. `None` = not a
    /// refresh area.
    fn refresh(&self) -> Option<crate::refresh::RefreshSpec> {
        None
    }

    /// The message a refresh area dispatches when an armed pull is released. `None` =
    /// the pull is shown but asks for nothing, which is what an area still waiting for
    /// its `on_refresh` looks like.
    fn on_refresh(&self) -> Option<Msg> {
        None
    }

    /// What the widget **withholds** from the frame on behalf of its whole subtree:
    /// input targets, primitives, accessibility nodes. The walk visits the subtree
    /// normally and then discards whatever it added to the selected registries, so a
    /// target registered deep inside is caught just as surely as one at the top.
    /// `None` = nothing withheld (the default). See [`crate::ModalBarrier`],
    /// [`crate::IgnorePointer`], [`crate::AbsorbPointer`], [`crate::Visibility`],
    /// [`crate::ExcludeSemantics`].
    fn barrier(&self) -> Option<crate::barrier::ModalBarrier> {
        None
    }

    /// If the widget is an **interactive viewport** (`InteractiveViewer`), returns its
    /// scale bounds `(min, max)`. The paint walk renders its child in a transformed
    /// layer (retained scale + translation, clipped to the viewport); the shell routes
    /// drag → pan and wheel/pinch → zoom. `None` = not an interactive viewport.
    /// See [`crate::InteractiveViewer`].
    fn interactive(&self) -> Option<(f32, f32)> {
        None
    }

    /// If the widget **fits** its child to its box (`FittedBox`), returns the
    /// [`frus_core::BoxFit`]: the walk measures the child at its natural size, scales
    /// it according to the fit and centres it (composited layer, hit-test through
    /// `M⁻¹`). `None` = not a fitter. See [`crate::FittedBox`].
    fn fitted(&self) -> Option<frus_core::BoxFit> {
        None
    }

    /// If the widget **rotates** its child by quarter turns (`RotatedBox`), returns
    /// the number of quarters. Unlike `Transform`, this **affects layout**: the box
    /// swaps its dimensions for an odd number. `None` = not a `RotatedBox`.
    /// See [`crate::RotatedBox`].
    fn rotated_quarter_turns(&self) -> Option<i32> {
        None
    }

    /// If the widget is a screen navigator, returns `(progress, push?)`. Its children
    /// (`[screen]` or `[outgoing, incoming]`) are rendered full-window with a sliding
    /// transition.
    fn navigator(&self) -> Option<(f32, bool)> {
        None
    }

    /// Whether a [`navigator`](Self::navigator) **clips its pages to its own box**.
    /// `true`, which is the reference's default (`Clip.hardEdge`) and the only sane one:
    /// a screen sliding in comes from outside the box and has to stop at its edge.
    ///
    /// Consulted only inside the navigator branch of the walk, so it costs nothing for
    /// every other widget.
    fn navigator_clips(&self) -> bool {
        true
    }

    /// Message emitted by a **long press** (a press held ~500 ms without movement).
    /// The long press *pre-empts* the click: the release that follows does not emit
    /// `on_click`.
    fn on_long_press(&self) -> Option<Msg> {
        None
    }

    /// Key received while **bubbling leaf→root**: the focused widget gets it first,
    /// then each ancestor as long as the response is `Ignored`. (E.g. an `OverlayPortal`
    /// consumes `Escape` to close itself.)
    fn on_key(&self, _key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        crate::interaction::KeyResponse::Ignored
    }

    /// **Measure under constraints**: for a widget whose size depends on the space
    /// offered (a paragraph that wraps…), returns the closure wired into taffy.
    /// `None` = size fixed by `style()`. Contract: must be `Some` **if and only if**
    /// [`Widget::measure_key`] is.
    ///
    /// The theme comes in because the measurement has to agree with the paint. A text
    /// that takes its size from an inherited style must be *measured* at that size too;
    /// measured at one size and drawn at another, every box on the screen is the wrong
    /// height at once, and nothing in the picture says which of the two numbers is wrong.
    fn measure(&self, _theme: &Theme) -> Option<frus_layout::MeasureFn<'_>> {
        None
    }

    /// Fingerprint of the **content** [`Widget::measure`] depends on (text, style…),
    /// mixed into the relayout fingerprint: without it, two different contents with
    /// the same style would be conflated by the cache and would keep a stale
    /// geometry. Contract: `Some` if and only if `measure()` is.
    ///
    /// Whatever the theme contributes to the measurement belongs in here as well — a
    /// cache key that ignores half of its inputs is a stale layout waiting for the
    /// subtree's style to change.
    fn measure_key(&self, _theme: &Theme) -> Option<u64> {
        None
    }
}

/// **Short** type name: without module path or generic parameters
/// (`frus_widgets::text::Text` → `Text`, `Container<Msg>` → `Container`).
pub(crate) fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    let no_generics = full.split('<').next().unwrap_or(full);
    no_generics.rsplit("::").next().unwrap_or(no_generics)
}

/// Lets an **already boxed** widget be composed where an `impl Widget` is expected
/// (e.g. `Flex::child`). Delegates everything to the contained widget.
impl<Msg> Widget<Msg> for Box<dyn Widget<Msg>> {
    fn style(&self) -> Style {
        (**self).style()
    }
    fn style_themed(&self, theme: &Theme) -> Style {
        (**self).style_themed(theme)
    }
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        (**self).theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        (**self).media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        (**self).scaffold_override()
    }
    fn build_themed(&self, theme: &Theme) {
        (**self).build_themed(theme)
    }
    fn repaint_boundary(&self) -> bool {
        (**self).repaint_boundary()
    }
    fn debug_name(&self) -> &'static str {
        (**self).debug_name()
    }
    fn semantics(&self) -> Option<SemanticsProperties> {
        (**self).semantics()
    }
    fn describes(&self) -> Option<crate::semantics::Description> {
        (**self).describes()
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        (**self).children()
    }
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        (**self).paint(bounds, status, theme, scene)
    }
    fn on_click(&self) -> Option<Msg> {
        (**self).on_click()
    }
    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        (**self).positional_click(local_x, local_y, width, height)
    }
    fn cursor_icon(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Cursor> {
        (**self).cursor_icon(local_x, local_y, width, height)
    }
    fn key(&self) -> Option<u64> {
        (**self).key()
    }
    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        (**self).on_edit(edit, key)
    }
    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        scroll_cursor: usize,
    ) -> Option<usize> {
        (**self).cursor_at(local_x, local_y, width, scroll_cursor)
    }
    fn text_metrics(&self, width: f32, cursor: usize) -> Option<(f32, f32, f32, f32)> {
        (**self).text_metrics(width, cursor)
    }
    fn text_viewport(&self, rect: crate::Rect) -> Option<crate::Rect> {
        (**self).text_viewport(rect)
    }
    fn caret_vertical(
        &self,
        width: f32,
        cursor: usize,
        down: bool,
        page: bool,
        goal_x: Option<f32>,
    ) -> Option<(usize, f32)> {
        (**self).caret_vertical(width, cursor, down, page, goal_x)
    }
    fn selected_text(&self, edit: &Edit) -> Option<String> {
        (**self).selected_text(edit)
    }
    fn text_value(&self) -> Option<&str> {
        (**self).text_value()
    }
    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        (**self).word_at(index)
    }
    fn focusable(&self) -> bool {
        (**self).focusable()
    }

    fn shortcut_bindings(&self) -> &[(crate::shortcuts::KeyStroke, crate::shortcuts::Intent)] {
        (**self).shortcut_bindings()
    }

    fn shortcut_callbacks(&self) -> &[(crate::shortcuts::KeyStroke, Msg)] {
        (**self).shortcut_callbacks()
    }

    fn action_bindings(&self) -> &[(crate::shortcuts::Intent, Msg)] {
        (**self).action_bindings()
    }

    fn action_listeners(&self) -> &[(crate::shortcuts::Intent, Msg)] {
        (**self).action_listeners()
    }

    fn on_keystroke(
        &self,
    ) -> Option<std::rc::Rc<dyn Fn(crate::shortcuts::KeyStroke) -> Option<Msg>>> {
        (**self).on_keystroke()
    }

    fn descendants_focusable(&self) -> bool {
        (**self).descendants_focusable()
    }

    fn focus_skip_traversal(&self) -> bool {
        (**self).focus_skip_traversal()
    }

    fn focus_order(&self) -> Option<f32> {
        (**self).focus_order()
    }

    fn focus_group(&self) -> bool {
        (**self).focus_group()
    }
    fn draws_own_focus(&self) -> bool {
        (**self).draws_own_focus()
    }
    fn draggable(&self) -> bool {
        (**self).draggable()
    }
    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        (**self).on_drag(fraction)
    }
    fn on_drag_start(&self, fraction: f32) -> Option<Msg> {
        (**self).on_drag_start(fraction)
    }
    fn on_drag_end(&self, fraction: f32) -> Option<Msg> {
        (**self).on_drag_end(fraction)
    }
    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        (**self).on_drag_delta(dx)
    }
    fn reorder_index(&self) -> Option<usize> {
        (**self).reorder_index()
    }
    fn on_reorder(&self, to: usize) -> Option<Msg> {
        (**self).on_reorder(to)
    }
    fn reorder_draggable(&self) -> bool {
        (**self).reorder_draggable()
    }
    fn reorder_axis(&self) -> ReorderAxis {
        (**self).reorder_axis()
    }
    fn announce(&self) -> Option<String> {
        (**self).announce()
    }
    fn stack(&self) -> bool {
        (**self).stack()
    }
    fn continuous(&self) -> bool {
        (**self).continuous()
    }
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        (**self).scroll_content()
    }
    fn virtual_list(&self, viewport: frus_core::Size) -> Option<crate::list::VirtualList<'_, Msg>> {
        (**self).virtual_list(viewport)
    }
    fn page_view(&self) -> Option<crate::pageview::PagedView<'_, Msg>> {
        (**self).page_view()
    }
    fn intrinsic(&self) -> Option<(crate::constraints::IntrinsicAxis, Option<f32>)> {
        (**self).intrinsic()
    }
    fn overflow_box(&self) -> Option<crate::constraints::Overflow> {
        (**self).overflow_box()
    }
    fn hero_tag(&self) -> Option<u64> {
        (**self).hero_tag()
    }
    fn drag_payload(&self) -> Option<u64> {
        (**self).drag_payload()
    }
    fn drag_needs_long_press(&self) -> bool {
        (**self).drag_needs_long_press()
    }
    fn drag_ghost_opacity(&self) -> f32 {
        (**self).drag_ghost_opacity()
    }
    fn on_dropped(&self, accepted: bool) -> Option<Msg> {
        (**self).on_dropped(accepted)
    }
    fn drop_zone(&self) -> bool {
        (**self).drop_zone()
    }
    fn accepts_drag(&self, payload: u64) -> bool {
        (**self).accepts_drag(payload)
    }
    fn on_drop(&self, payload: u64) -> Option<Msg> {
        (**self).on_drop(payload)
    }
    fn on_page_changed(&self, page: usize) -> Option<Msg> {
        (**self).on_page_changed(page)
    }
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        (**self).layout_builder()
    }
    fn scroll_axis(&self) -> Axis {
        (**self).scroll_axis()
    }
    fn scroll_physics(&self) -> Option<crate::physics::ScrollPhysics> {
        (**self).scroll_physics()
    }
    fn ime(&self) -> crate::ime::Ime {
        (**self).ime()
    }
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        (**self).overlay()
    }
    fn overlay_dismiss(&self) -> Option<Msg> {
        (**self).overlay_dismiss()
    }
    fn overlay_scrim(&self, theme: &crate::theme::Theme) -> Option<frus_core::Color> {
        (**self).overlay_scrim(theme)
    }
    fn overlay_traps_focus(&self) -> bool {
        (**self).overlay_traps_focus()
    }
    fn anim_target(&self) -> Option<f32> {
        (**self).anim_target()
    }
    fn anim_duration(&self) -> f32 {
        (**self).anim_duration()
    }
    fn anim_curve(&self) -> frus_core::Curve {
        (**self).anim_curve()
    }
    fn opacity_group(&self) -> Option<f32> {
        (**self).opacity_group()
    }
    fn anim_color(&self) -> Option<frus_core::Color> {
        (**self).anim_color()
    }
    fn anim_size(&self) -> Option<Size> {
        (**self).anim_size()
    }
    fn anim_radius(&self) -> Option<frus_core::BorderRadius> {
        (**self).anim_radius()
    }
    fn anim_padding(&self) -> Option<frus_core::Insets> {
        (**self).anim_padding()
    }
    fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
        (**self).alignment_geometry()
    }
    fn transform_translate(&self) -> Option<(f32, f32)> {
        (**self).transform_translate()
    }
    fn transform_scale(&self) -> Option<(f32, f32, frus_core::Alignment)> {
        (**self).transform_scale()
    }
    fn transform_rotate(&self) -> Option<(f32, frus_core::Alignment)> {
        (**self).transform_rotate()
    }
    fn clip_shape(&self) -> Option<frus_core::ClipShape> {
        (**self).clip_shape()
    }
    fn clip_path(&self) -> Option<&frus_core::Path> {
        (**self).clip_path()
    }
    fn layer_filter(&self, cx: FilterContext) -> Option<frus_core::LayerFilter> {
        (**self).layer_filter(cx)
    }
    fn text_baseline(&self, theme: &Theme) -> Option<f32> {
        (**self).text_baseline(theme)
    }
    fn ignores_baseline(&self) -> bool {
        (**self).ignores_baseline()
    }
    fn baseline_target(&self) -> Option<f32> {
        (**self).baseline_target()
    }
    fn fill_axes(&self, theme: &Theme) -> FillAxes {
        (**self).fill_axes(theme)
    }
    fn main_axis_floor(&self, theme: &Theme) -> Option<f32> {
        (**self).main_axis_floor(theme)
    }
    fn tile_shape(&self) -> Option<f32> {
        (**self).tile_shape()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        (**self).positioned()
    }
    fn stack_loose(&self) -> bool {
        (**self).stack_loose()
    }
    fn scroll_reverse(&self) -> bool {
        (**self).scroll_reverse()
    }

    fn keep_visible(&self, size: Size, theme: &Theme) -> Option<crate::ui::KeepVisible> {
        (**self).keep_visible(size, theme)
    }
    fn scroll_padding(&self) -> frus_core::Insets {
        (**self).scroll_padding()
    }
    fn backdrop_group(&self) -> bool {
        (**self).backdrop_group()
    }
    fn ink(&self, theme: &Theme) -> Option<crate::ink::InkStyle> {
        (**self).ink(theme)
    }
    fn foreground(&self, theme: &Theme) -> Option<frus_core::BoxDecoration> {
        (**self).foreground(theme)
    }
    fn barrier(&self) -> Option<crate::barrier::ModalBarrier> {
        (**self).barrier()
    }
    fn dismissible(&self) -> Option<crate::dismiss::DismissSpec> {
        (**self).dismissible()
    }
    fn on_dismissed(&self, direction: crate::dismiss::DismissDirection) -> Option<Msg> {
        (**self).on_dismissed(direction)
    }
    fn refresh(&self) -> Option<crate::refresh::RefreshSpec> {
        (**self).refresh()
    }
    fn on_refresh(&self) -> Option<Msg> {
        (**self).on_refresh()
    }
    fn interactive(&self) -> Option<(f32, f32)> {
        (**self).interactive()
    }
    fn fitted(&self) -> Option<frus_core::BoxFit> {
        (**self).fitted()
    }
    fn rotated_quarter_turns(&self) -> Option<i32> {
        (**self).rotated_quarter_turns()
    }
    fn navigator(&self) -> Option<(f32, bool)> {
        (**self).navigator()
    }
    fn navigator_clips(&self) -> bool {
        (**self).navigator_clips()
    }
    fn measure(&self, theme: &Theme) -> Option<frus_layout::MeasureFn<'_>> {
        (**self).measure(theme)
    }
    fn measure_key(&self, theme: &Theme) -> Option<u64> {
        (**self).measure_key(theme)
    }
    fn on_long_press(&self) -> Option<Msg> {
        (**self).on_long_press()
    }
    fn on_key(&self, key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        (**self).on_key(key)
    }
}
