//! [`Keyed`]: a **transparent** wrapper that gives a widget a **stable identity**,
//! through a key, independent of its position among its siblings.
//!
//! Without a key, identity is positional: removing an item from the middle of a list
//! shifts the identity of everything after it, and their retained state — hover, focus,
//! caret, animations, the leaving fade — jumps. Wrapping each item in a `Keyed`, keyed
//! on its stable domain id, fixes that.

use std::hash::{Hash, Hasher};

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
use crate::theme::Theme;
use crate::widget::Widget;

/// Wraps a widget in a stable identity key, delegating everything else.
pub struct Keyed<Msg> {
    key: u64,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> Keyed<Msg> {
    /// Wraps `inner` with the key `key`, which may be any hashable type.
    pub fn new(key: impl Hash, inner: impl Widget<Msg> + 'static) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        Self {
            key: hasher.finish(),
            inner: Box::new(inner),
        }
    }
}

impl<Msg> Widget<Msg> for Keyed<Msg> {
    fn style(&self) -> Style {
        self.inner.style()
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.inner.style_themed(theme)
    }

    fn debug_name(&self) -> &'static str {
        // A transparent wrapper: the inspector shows the wrapped widget.
        self.inner.debug_name()
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        self.inner.semantics()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.inner.children()
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        self.inner.paint(bounds, status, theme, scene);
    }

    fn on_click(&self) -> Option<Msg> {
        self.inner.on_click()
    }

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        self.inner.positional_click(local_x, local_y, width, height)
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        self.inner.cursor_icon(local_x, local_y, width, height)
    }

    fn key(&self) -> Option<u64> {
        Some(self.key)
    }

    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        self.inner.on_edit(edit, key)
    }

    fn cursor_at(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        scroll_cursor: usize,
    ) -> Option<usize> {
        self.inner.cursor_at(local_x, local_y, width, scroll_cursor)
    }

    fn text_metrics(&self, width: f32, cursor: usize) -> Option<(f32, f32, f32, f32)> {
        self.inner.text_metrics(width, cursor)
    }

    fn text_viewport(&self, rect: frus_core::Rect) -> Option<frus_core::Rect> {
        self.inner.text_viewport(rect)
    }

    fn caret_vertical(
        &self,
        width: f32,
        cursor: usize,
        down: bool,
        page: bool,
        goal_x: Option<f32>,
    ) -> Option<(usize, f32)> {
        self.inner.caret_vertical(width, cursor, down, page, goal_x)
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        self.inner.selected_text(edit)
    }

    fn text_value(&self) -> Option<&str> {
        self.inner.text_value()
    }

    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        self.inner.word_at(index)
    }

    fn focusable(&self) -> bool {
        self.inner.focusable()
    }

    // The **structural** questions the walk and the layout ask before they look at a
    // widget's children. A transparent wrapper that answered these for itself would
    // change how its content is laid out — a keyed stack would have its layers put in
    // flow instead of on top of one another — which is exactly what a wrapper that
    // claims to be transparent must not do.
    fn stack(&self) -> bool {
        self.inner.stack()
    }

    fn continuous(&self) -> bool {
        self.inner.continuous()
    }

    fn draws_own_focus(&self) -> bool {
        self.inner.draws_own_focus()
    }

    fn repaint_boundary(&self) -> bool {
        self.inner.repaint_boundary()
    }

    fn draggable(&self) -> bool {
        self.inner.draggable()
    }

    fn on_drag(&self, fraction: f32) -> Option<Msg> {
        self.inner.on_drag(fraction)
    }

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        self.inner.on_drag_delta(dx)
    }

    fn reorder_index(&self) -> Option<usize> {
        self.inner.reorder_index()
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        self.inner.on_reorder(to)
    }

    fn announce(&self) -> Option<String> {
        self.inner.announce()
    }

    fn scroll_content(&self) -> Option<&dyn Widget<Msg>> {
        self.inner.scroll_content()
    }

    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        self.inner.virtual_list()
    }

    fn page_view(&self) -> Option<crate::pageview::PagedView<'_, Msg>> {
        self.inner.page_view()
    }

    fn intrinsic(&self) -> Option<(crate::constraints::IntrinsicAxis, Option<f32>)> {
        self.inner.intrinsic()
    }

    fn overflow_box(&self) -> Option<crate::constraints::Overflow> {
        self.inner.overflow_box()
    }

    fn hero_tag(&self) -> Option<u64> {
        self.inner.hero_tag()
    }

    fn drag_payload(&self) -> Option<u64> {
        self.inner.drag_payload()
    }

    fn drag_needs_long_press(&self) -> bool {
        self.inner.drag_needs_long_press()
    }

    fn drag_ghost_opacity(&self) -> f32 {
        self.inner.drag_ghost_opacity()
    }

    fn on_dropped(&self, accepted: bool) -> Option<Msg> {
        self.inner.on_dropped(accepted)
    }

    fn drop_zone(&self) -> bool {
        self.inner.drop_zone()
    }

    fn accepts_drag(&self, payload: u64) -> bool {
        self.inner.accepts_drag(payload)
    }

    fn on_drop(&self, payload: u64) -> Option<Msg> {
        self.inner.on_drop(payload)
    }

    fn on_page_changed(&self, page: usize) -> Option<Msg> {
        self.inner.on_page_changed(page)
    }

    fn layout_builder(&self) -> Option<&dyn Fn(frus_core::Size) -> Box<dyn Widget<Msg>>> {
        self.inner.layout_builder()
    }

    fn scroll_axis(&self) -> Axis {
        self.inner.scroll_axis()
    }

    fn scroll_physics(&self) -> Option<crate::physics::ScrollPhysics> {
        self.inner.scroll_physics()
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.inner.overlay()
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.inner.overlay_dismiss()
    }

    fn overlay_traps_focus(&self) -> bool {
        self.inner.overlay_traps_focus()
    }

    fn anim_target(&self) -> Option<f32> {
        self.inner.anim_target()
    }

    fn anim_duration(&self) -> f32 {
        self.inner.anim_duration()
    }

    fn anim_curve(&self) -> frus_core::Curve {
        self.inner.anim_curve()
    }

    fn opacity_group(&self) -> Option<f32> {
        self.inner.opacity_group()
    }

    fn anim_color(&self) -> Option<frus_core::Color> {
        self.inner.anim_color()
    }

    fn anim_size(&self) -> Option<frus_core::Size> {
        self.inner.anim_size()
    }

    fn anim_radius(&self) -> Option<frus_core::BorderRadius> {
        self.inner.anim_radius()
    }

    fn anim_padding(&self) -> Option<frus_core::Insets> {
        self.inner.anim_padding()
    }

    fn alignment_geometry(&self) -> Option<frus_core::AlignmentGeometry> {
        self.inner.alignment_geometry()
    }

    fn transform_translate(&self) -> Option<(f32, f32)> {
        self.inner.transform_translate()
    }

    fn transform_scale(&self) -> Option<(f32, f32, frus_core::Alignment)> {
        self.inner.transform_scale()
    }

    fn transform_rotate(&self) -> Option<(f32, frus_core::Alignment)> {
        self.inner.transform_rotate()
    }

    fn clip_shape(&self) -> Option<frus_core::ClipShape> {
        self.inner.clip_shape()
    }

    fn clip_path(&self) -> Option<&frus_core::Path> {
        self.inner.clip_path()
    }

    fn barrier(&self) -> Option<crate::barrier::Barrier> {
        self.inner.barrier()
    }

    fn dismissible(&self) -> Option<crate::dismiss::DismissSpec> {
        self.inner.dismissible()
    }

    fn on_dismissed(&self, direction: crate::dismiss::DismissDirection) -> Option<Msg> {
        self.inner.on_dismissed(direction)
    }

    fn refresh(&self) -> Option<crate::refresh::RefreshSpec> {
        self.inner.refresh()
    }

    fn on_refresh(&self) -> Option<Msg> {
        self.inner.on_refresh()
    }

    fn interactive(&self) -> Option<(f32, f32)> {
        self.inner.interactive()
    }

    fn fitted(&self) -> Option<frus_core::BoxFit> {
        self.inner.fitted()
    }

    fn rotated_quarter_turns(&self) -> Option<i32> {
        self.inner.rotated_quarter_turns()
    }

    fn navigator(&self) -> Option<(f32, bool)> {
        self.inner.navigator()
    }

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        self.inner.measure()
    }

    fn measure_key(&self) -> Option<u64> {
        self.inner.measure_key()
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.inner.on_long_press()
    }

    fn on_key(&self, key: &crate::Key) -> crate::KeyResponse<Msg> {
        self.inner.on_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Text};

    /// The structural questions decide **how the content is laid out**, so a
    /// transparent wrapper must pass them through. Answering them for itself made a
    /// keyed stack lay its layers out in flow instead of on top of one another — found
    /// on a device, wrapping a swipeable row in `keyed(...)`.
    #[test]
    fn structural_questions_pass_through() {
        let stack = crate::Stack::<()>::new()
            .width(100.0)
            .height(50.0)
            .layer(Container::new())
            .layer(Container::new());
        assert!(Widget::<()>::stack(&stack));
        assert!(
            Widget::<()>::stack(&Keyed::new(1u64, stack)),
            "a keyed stack is still a stack"
        );

        let spinner = crate::Spinner::new();
        assert!(Widget::<()>::continuous(&spinner));
        assert!(Widget::<()>::continuous(&Keyed::new(2u64, spinner)));
    }

    #[test]
    fn reports_key_and_delegates() {
        let inner = Container::<()>::new().width(40.0).child(Text::new("x"));
        let keyed = Keyed::new(99u64, inner);
        // It returns a key.
        assert!(Widget::<()>::key(&keyed).is_some());
        // It delegates the children; the Container has one.
        assert_eq!(Widget::<()>::children(&keyed).len(), 1);
    }

    #[test]
    fn same_key_same_hash() {
        let a: Keyed<()> = Keyed::new("todo-3", Container::new());
        let b: Keyed<()> = Keyed::new("todo-3", Container::new());
        assert_eq!(Widget::<()>::key(&a), Widget::<()>::key(&b));
    }
}
