//! [`Responsive`]: picks a subtree from the current [`SizeClass`].
//!
//! `responsive(width).compact(a).medium(b).expanded(c)` selects the variant matching
//! the width breakpoint, with a **graceful fallback**: when the exact breakpoint is not
//! supplied, the nearest is used, preferring the smaller one. The resulting widget
//! **delegates everything** to the chosen variant, as [`Keyed`] does.
//!
//! [`Keyed`]: crate::Keyed

use frus_core::{Rect, Scene, SizeClass};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
use crate::theme::Theme;
use crate::widget::Widget;

/// Selects a subtree from the size class, delegating to whichever is chosen.
pub struct Responsive<Msg> {
    class_rank: u8,
    inner: Option<Box<dyn Widget<Msg>>>,
    inner_rank: u8,
}

/// A breakpoint's distance from the target class: minimising `(gap, is_above)` picks
/// the nearest, preferring a smaller breakpoint when the gaps are equal.
fn closeness(rank: u8, class: u8) -> (u8, bool) {
    (rank.abs_diff(class), rank > class)
}

impl<Msg> Responsive<Msg> {
    /// Builds a selector for the class `class`.
    pub fn new(class: SizeClass) -> Self {
        Self {
            class_rank: class.rank(),
            inner: None,
            inner_rank: 0,
        }
    }

    /// Considers `widget` for the breakpoint of rank `rank`, keeping the best
    /// candidate seen so far, independently of the order of the calls.
    fn consider(mut self, rank: u8, widget: impl Widget<Msg> + 'static) -> Self {
        let better = self.inner.is_none()
            || closeness(rank, self.class_rank) < closeness(self.inner_rank, self.class_rank);
        if better {
            self.inner = Some(Box::new(widget));
            self.inner_rank = rank;
        }
        self
    }

    /// Variant for the `Compact` breakpoint (< 600 px).
    pub fn compact(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.consider(SizeClass::Compact.rank(), widget)
    }

    /// Variant for the `Medium` breakpoint (600–840 px).
    pub fn medium(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.consider(SizeClass::Medium.rank(), widget)
    }

    /// Variant for the `Expanded` breakpoint (≥ 840 px).
    pub fn expanded(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.consider(SizeClass::Expanded.rank(), widget)
    }
}

/// A responsive selector for a given width, in logical px.
///
/// `responsive(w).compact(a).medium(b).expanded(c)` — see [`Responsive`].
pub fn responsive<Msg>(width: f32) -> Responsive<Msg> {
    Responsive::new(SizeClass::from_width(width))
}

impl<Msg> Widget<Msg> for Responsive<Msg> {
    fn style(&self) -> Style {
        self.inner.as_ref().map(|w| w.style()).unwrap_or_default()
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.inner
            .as_ref()
            .map(|w| w.style_themed(theme))
            .unwrap_or_default()
    }

    fn debug_name(&self) -> &'static str {
        // A transparent wrapper: the inspector shows the realised widget.
        self.inner
            .as_ref()
            .map(|w| w.debug_name())
            .unwrap_or("Responsive")
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        self.inner.as_ref().and_then(|w| w.semantics())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.inner.as_ref().map(|w| w.children()).unwrap_or(&[])
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        if let Some(w) = &self.inner {
            w.paint(bounds, status, theme, scene);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_click())
    }

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        self.inner
            .as_ref()
            .and_then(|w| w.positional_click(local_x, local_y, width, height))
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        self.inner
            .as_ref()
            .and_then(|w| w.cursor_icon(local_x, local_y, width, height))
    }

    fn key(&self) -> Option<u64> {
        self.inner.as_ref().and_then(|w| w.key())
    }

    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_edit(edit, key))
    }

    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        scroll_cursor: usize,
    ) -> Option<usize> {
        self.inner
            .as_ref()
            .and_then(|w| w.cursor_at(local_x, local_y, width, scroll_cursor))
    }

    fn text_metrics(&self, width: f32, cursor: usize) -> Option<(f32, f32, f32, f32)> {
        self.inner
            .as_ref()
            .and_then(|w| w.text_metrics(width, cursor))
    }

    fn text_viewport(&self, rect: frus_core::Rect) -> Option<frus_core::Rect> {
        self.inner.as_ref().and_then(|w| w.text_viewport(rect))
    }

    fn caret_vertical(
        &self,
        width: f32,
        cursor: usize,
        down: bool,
        page: bool,
        goal_x: Option<f32>,
    ) -> Option<(usize, f32)> {
        self.inner
            .as_ref()
            .and_then(|w| w.caret_vertical(width, cursor, down, page, goal_x))
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        self.inner.as_ref().and_then(|w| w.selected_text(edit))
    }

    fn text_value(&self) -> Option<&str> {
        self.inner.as_ref().and_then(|w| w.text_value())
    }

    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        self.inner.as_ref().and_then(|w| w.word_at(index))
    }

    fn focusable(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.focusable())
    }

    fn draws_own_focus(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.draws_own_focus())
    }

    fn stack(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.stack())
    }

    fn continuous(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.continuous())
    }

    fn virtual_list(&self, viewport: frus_core::Size) -> Option<crate::list::VirtualList<'_, Msg>> {
        self.inner.as_ref().and_then(|w| w.virtual_list(viewport))
    }

    fn page_view(&self) -> Option<crate::pageview::PagedView<'_, Msg>> {
        self.inner.as_ref().and_then(|w| w.page_view())
    }

    fn intrinsic(&self) -> Option<(crate::constraints::IntrinsicAxis, Option<f32>)> {
        self.inner.as_ref().and_then(|w| w.intrinsic())
    }

    fn overflow_box(&self) -> Option<crate::constraints::Overflow> {
        self.inner.as_ref().and_then(|w| w.overflow_box())
    }

    fn hero_tag(&self) -> Option<u64> {
        self.inner.as_ref().and_then(|w| w.hero_tag())
    }

    fn drag_payload(&self) -> Option<u64> {
        self.inner.as_ref().and_then(|w| w.drag_payload())
    }

    fn drag_needs_long_press(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|w| w.drag_needs_long_press())
    }

    fn drag_ghost_opacity(&self) -> f32 {
        self.inner.as_ref().map_or(1.0, |w| w.drag_ghost_opacity())
    }

    fn on_dropped(&self, accepted: bool) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_dropped(accepted))
    }

    fn drop_zone(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.drop_zone())
    }

    fn accepts_drag(&self, payload: u64) -> bool {
        self.inner.as_ref().is_none_or(|w| w.accepts_drag(payload))
    }

    fn on_drop(&self, payload: u64) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_drop(payload))
    }

    fn on_page_changed(&self, page: usize) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_page_changed(page))
    }

    fn layout_builder(&self) -> Option<&dyn Fn(frus_core::Size) -> Box<dyn Widget<Msg>>> {
        self.inner.as_ref().and_then(|w| w.layout_builder())
    }

    fn draggable(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.draggable())
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_drag(fraction))
    }

    fn on_drag_start(&self, fraction: f32) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_drag_start(fraction))
    }

    fn on_drag_end(&self, fraction: f32) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_drag_end(fraction))
    }

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_drag_delta(dx))
    }

    fn reorder_index(&self) -> Option<usize> {
        self.inner.as_ref().and_then(|w| w.reorder_index())
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_reorder(to))
    }

    fn announce(&self) -> Option<String> {
        self.inner.as_ref().and_then(|w| w.announce())
    }

    fn overlay_traps_focus(&self) -> bool {
        self.inner.as_ref().is_some_and(|w| w.overlay_traps_focus())
    }

    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        self.inner.as_ref().and_then(|w| w.scroll_content())
    }

    fn scroll_axis(&self) -> Axis {
        self.inner
            .as_ref()
            .map(|w| w.scroll_axis())
            .unwrap_or(Axis::Vertical)
    }

    fn scroll_physics(&self) -> Option<crate::physics::ScrollPhysics> {
        self.inner.as_ref().and_then(|w| w.scroll_physics())
    }

    fn keep_visible(
        &self,
        size: frus_core::Size,
        theme: &crate::theme::Theme,
    ) -> Option<crate::ui::KeepVisible> {
        self.inner
            .as_ref()
            .and_then(|w| w.keep_visible(size, theme))
    }

    fn ime(&self) -> crate::ime::Ime {
        self.inner.as_ref().map(|w| w.ime()).unwrap_or_default()
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.inner.as_ref().and_then(|w| w.overlay())
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.overlay_dismiss())
    }

    fn overlay_scrim(&self, theme: &Theme) -> Option<frus_core::Color> {
        self.inner.as_ref().and_then(|w| w.overlay_scrim(theme))
    }

    fn anim_target(&self) -> Option<f32> {
        self.inner.as_ref().and_then(|w| w.anim_target())
    }

    fn anim_duration(&self) -> f32 {
        self.inner
            .as_ref()
            .map(|w| w.anim_duration())
            .unwrap_or(crate::runtime::ANIM_DURATION)
    }

    fn anim_curve(&self) -> frus_core::Curve {
        self.inner
            .as_ref()
            .map(|w| w.anim_curve())
            .unwrap_or(frus_core::Curve::Linear)
    }

    fn opacity_group(&self) -> Option<f32> {
        self.inner.as_ref().and_then(|w| w.opacity_group())
    }

    fn anim_color(&self) -> Option<frus_core::Color> {
        self.inner.as_ref().and_then(|w| w.anim_color())
    }

    fn anim_size(&self) -> Option<frus_core::Size> {
        self.inner.as_ref().and_then(|w| w.anim_size())
    }

    fn anim_radius(&self) -> Option<frus_core::BorderRadius> {
        self.inner.as_ref().and_then(|w| w.anim_radius())
    }

    fn anim_padding(&self) -> Option<frus_core::Insets> {
        self.inner.as_ref().and_then(|w| w.anim_padding())
    }

    fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
        self.inner.as_ref().and_then(|w| w.alignment_geometry())
    }

    fn transform_translate(&self) -> Option<(f32, f32)> {
        self.inner.as_ref().and_then(|w| w.transform_translate())
    }

    fn transform_scale(&self) -> Option<(f32, f32, frus_core::Alignment)> {
        self.inner.as_ref().and_then(|w| w.transform_scale())
    }

    fn transform_rotate(&self) -> Option<(f32, frus_core::Alignment)> {
        self.inner.as_ref().and_then(|w| w.transform_rotate())
    }

    fn clip_shape(&self) -> Option<frus_core::ClipShape> {
        self.inner.as_ref().and_then(|w| w.clip_shape())
    }

    fn clip_path(&self) -> Option<&frus_core::Path> {
        self.inner.as_ref().and_then(|w| w.clip_path())
    }

    fn barrier(&self) -> Option<crate::barrier::ModalBarrier> {
        self.inner.as_ref().and_then(|w| w.barrier())
    }

    fn dismissible(&self) -> Option<crate::dismiss::DismissSpec> {
        self.inner.as_ref().and_then(|w| w.dismissible())
    }

    fn on_dismissed(&self, direction: crate::dismiss::DismissDirection) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_dismissed(direction))
    }

    fn refresh(&self) -> Option<crate::refresh::RefreshSpec> {
        self.inner.as_ref().and_then(|w| w.refresh())
    }

    fn on_refresh(&self) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_refresh())
    }

    fn interactive(&self) -> Option<(f32, f32)> {
        self.inner.as_ref().and_then(|w| w.interactive())
    }

    fn fitted(&self) -> Option<frus_core::BoxFit> {
        self.inner.as_ref().and_then(|w| w.fitted())
    }

    fn rotated_quarter_turns(&self) -> Option<i32> {
        self.inner.as_ref().and_then(|w| w.rotated_quarter_turns())
    }

    fn navigator(&self) -> Option<(f32, bool)> {
        self.inner.as_ref().and_then(|w| w.navigator())
    }

    fn measure(&self, theme: &Theme) -> Option<frus_layout::MeasureFn<'_>> {
        self.inner.as_ref().and_then(|w| w.measure(theme))
    }

    fn measure_key(&self, theme: &Theme) -> Option<u64> {
        self.inner.as_ref().and_then(|w| w.measure_key(theme))
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_long_press())
    }

    fn on_key(&self, key: &crate::Key) -> crate::KeyResponse<Msg> {
        self.inner
            .as_ref()
            .map(|w| w.on_key(key))
            .unwrap_or(crate::KeyResponse::Ignored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_layout::Dimension;

    /// Style width of the chosen variant (each breakpoint gets a distinct
    /// width so it can be identified).
    fn chosen_width(r: &Responsive<()>) -> Dimension {
        Widget::<()>::style(r).width
    }

    fn tagged(w: f32) -> Responsive<()> {
        responsive(w)
            .compact(Container::new().width(100.0))
            .medium(Container::new().width(200.0))
            .expanded(Container::new().width(300.0))
    }

    #[test]
    fn picks_variant_for_width() {
        assert_eq!(chosen_width(&tagged(400.0)), Dimension::Length(100.0));
        assert_eq!(chosen_width(&tagged(700.0)), Dimension::Length(200.0));
        assert_eq!(chosen_width(&tagged(1200.0)), Dimension::Length(300.0));
    }

    #[test]
    fn falls_back_to_nearest_when_missing() {
        // Only `expanded` is supplied, so it is used at every width.
        let only_expanded = |w: f32| responsive::<()>(w).expanded(Container::new().width(300.0));
        assert_eq!(
            chosen_width(&only_expanded(300.0)),
            Dimension::Length(300.0)
        );

        // compact plus expanded at a medium width (rank 1): compact (rank 0, gap 1,
        // below) wins over expanded (rank 2, gap 1, above).
        let two = responsive::<()>(700.0)
            .compact(Container::new().width(100.0))
            .expanded(Container::new().width(300.0));
        assert_eq!(chosen_width(&two), Dimension::Length(100.0));
    }
}
