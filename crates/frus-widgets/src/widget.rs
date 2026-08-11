//! The [`Widget`] trait, generic over the message type emitted on interaction.

use frus_core::{Rect, Scene, Semantics, Size};
use frus_layout::Style;

use crate::interaction::{Cursor, Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
use crate::theme::Theme;

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

/// A widget: a composable interface element.
///
/// `Msg` is the application message type emitted on interaction (a message-passing
/// model, in the Elm/iced style).
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
    /// may have some). See [`frus_core::Semantics`].
    fn semantics(&self) -> Option<Semantics> {
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
    /// **suffix** icon of a [`crate::TextInput`] (clear / reveal) or a chart **point**
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

    /// Scroll metrics of a **multi-line** field, for a widget width and a cursor:
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

    /// If `true`, the widget can take keyboard focus (click or Tab).
    fn focusable(&self) -> bool {
        false
    }

    /// If `true`, the widget draws its focus indicator **itself** (the driver then
    /// does not stroke the generic ring). E.g. `TextInput`.
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

    /// Message produced by a horizontal drag expressed as a **delta**: `dx` is the
    /// movement (logical px) since the last event. For handles that **accumulate**
    /// (column resizing), unlike [`on_drag`](Self::on_drag) (an absolute fraction,
    /// e.g. a slider). The shell tries it **before** `on_drag`; a widget implements
    /// only one of the two.
    fn on_drag_delta(&self, _dx: f32) -> Option<Msg> {
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
    /// target): the framework keeps redrawing. E.g. `Spinner`.
    fn continuous(&self) -> bool {
        false
    }

    /// If `true`, this widget is a **repaint boundary**: its subtree is cached
    /// (primitives + interaction maps) and **reused as is** as long as its geometry
    /// and the interaction state of its descendants do not change — a widget
    /// animating elsewhere no longer forces it to repaint.
    /// See [`crate::RepaintBoundary`] and `paintcache.rs`.
    fn repaint_boundary(&self) -> bool {
        false
    }

    /// If the widget is a scrollable container, returns its content.
    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        None
    }

    /// If the widget is a **virtualised list**, returns its description (item count,
    /// height, factory). Only the visible items are built.
    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        None
    }

    /// If the widget builds its content **from its actual box**, returns the
    /// `size → widget` factory. The content is built on the fly: no retained state
    /// and no overlay (like a virtualised list item).
    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        None
    }

    /// Scroll axis (or axes), for a scrollable container.
    fn scroll_axis(&self) -> crate::scroll::Axis {
        crate::scroll::Axis::Vertical
    }

    /// The edge and fling behaviour of a scrollable container, when this widget
    /// wants one in particular. `None` — the usual answer — leaves the choice to
    /// the application, which defaults to what the platform does.
    fn scroll_physics(&self) -> Option<crate::physics::ScrollPhysics> {
        None
    }

    /// If the widget is a portal, returns its floating content and its placement.
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        None
    }

    /// Message emitted when the scrim of a modal is clicked (dismissal), if any.
    fn overlay_dismiss(&self) -> Option<Msg> {
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

    /// If the widget is a **refresh area** ([`crate::Refresh`]), its configuration for
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
    /// `None` = nothing withheld (the default). See [`crate::Barrier`],
    /// [`crate::IgnorePointer`], [`crate::AbsorbPointer`], [`crate::Visibility`],
    /// [`crate::ExcludeSemantics`].
    fn barrier(&self) -> Option<crate::barrier::Barrier> {
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

    /// Message emitted by a **long press** (a press held ~500 ms without movement).
    /// The long press *pre-empts* the click: the release that follows does not emit
    /// `on_click`.
    fn on_long_press(&self) -> Option<Msg> {
        None
    }

    /// Key received while **bubbling leaf→root**: the focused widget gets it first,
    /// then each ancestor as long as the response is `Ignored`. (E.g. a `Portal`
    /// consumes `Escape` to close itself.)
    fn on_key(&self, _key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        crate::interaction::KeyResponse::Ignored
    }

    /// **Measure under constraints**: for a widget whose size depends on the space
    /// offered (a paragraph that wraps…), returns the closure wired into taffy.
    /// `None` = size fixed by `style()`. Contract: must be `Some` **if and only if**
    /// [`Widget::measure_key`] is.
    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        None
    }

    /// Fingerprint of the **content** [`Widget::measure`] depends on (text, style…),
    /// mixed into the relayout fingerprint: without it, two different contents with
    /// the same style would be conflated by the cache and would keep a stale
    /// geometry. Contract: `Some` if and only if `measure()` is.
    fn measure_key(&self) -> Option<u64> {
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
    fn debug_name(&self) -> &'static str {
        (**self).debug_name()
    }
    fn semantics(&self) -> Option<Semantics> {
        (**self).semantics()
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
    fn draws_own_focus(&self) -> bool {
        (**self).draws_own_focus()
    }
    fn draggable(&self) -> bool {
        (**self).draggable()
    }
    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        (**self).on_drag(fraction)
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
    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        (**self).virtual_list()
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
    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        (**self).overlay()
    }
    fn overlay_dismiss(&self) -> Option<Msg> {
        (**self).overlay_dismiss()
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
    fn barrier(&self) -> Option<crate::barrier::Barrier> {
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
    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        (**self).measure()
    }
    fn measure_key(&self) -> Option<u64> {
        (**self).measure_key()
    }
    fn on_long_press(&self) -> Option<Msg> {
        (**self).on_long_press()
    }
    fn on_key(&self, key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        (**self).on_key(key)
    }
}
