//! The **transparent wrapper**: one macro, so that a wrapper cannot forget a hook.
//!
//! A transparent wrapper *is* its child. It exists to add something the tree needs — a
//! stable identity ([`crate::Keyed`]), a theme for a subtree ([`crate::Themed`]) — and
//! must otherwise be indistinguishable from the widget it holds: the same style, the same
//! children, the same paint, and above all the same answers to the **structural**
//! questions the walk asks before it looks at a widget at all.
//!
//! Those answers are where this goes wrong. A wrapper that forwards `style` and
//! `children` and stops there looks correct in every test, until someone wraps a
//! [`crate::Stack`] in it and the layers lay out in flow, or wraps a scrollable and the
//! scrolling stops. That bug has been found on a device here once already, which is why
//! there is a macro rather than a convention: a wrapper written by hand forwards what its
//! author remembered.
//!
//! ```ignore
//! forward_transparent!(Keyed {
//!     fn key(&self) -> Option<u64> {
//!         Some(self.key)                            // what this wrapper adds
//!     }
//!     fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
//!         self.inner.theme_override(inherited)      // and what it passes through
//!     }
//! });
//! ```
//!
//! The type must have an `inner: Box<dyn Widget<Msg>>` field. The two hooks above are the
//! ones a transparent wrapper can have a reason to claim, so the macro leaves **both** to
//! the caller: whichever one this wrapper is not for is then visibly forwarded rather than
//! quietly defaulted.

/// Implements `Widget<Msg>` for a `{ inner: Box<dyn Widget<Msg>>, … }` wrapper by
/// delegating every hook to `inner`, except [`Widget::key`] and
/// [`Widget::theme_override`](crate::Widget::theme_override) — the two a wrapper may
/// legitimately answer for itself, which the caller therefore states, claimed or
/// forwarded.
macro_rules! forward_transparent {
    ($ty:ident { $($extra:item)* }) => {
        impl<Msg> $crate::widget::Widget<Msg> for $ty<Msg> {
            $($extra)*

            fn style(&self) -> frus_layout::Style {
                self.inner.style()
            }

            fn style_themed(&self, theme: &$crate::theme::Theme) -> frus_layout::Style {
                self.inner.style_themed(theme)
            }

            fn debug_name(&self) -> &'static str {
                // A transparent wrapper: the inspector shows the wrapped widget.
                self.inner.debug_name()
            }

            fn semantics(&self) -> Option<frus_core::Semantics> {
                self.inner.semantics()
            }

            fn children(&self) -> &[Box<dyn $crate::widget::Widget<Msg>>] {
                self.inner.children()
            }

            fn paint(
                &self,
                bounds: frus_core::Rect,
                status: $crate::interaction::Status,
                theme: &$crate::theme::Theme,
                scene: &mut frus_core::Scene,
            ) {
                self.inner.paint(bounds, status, theme, scene);
            }

            fn ink(&self, theme: &$crate::theme::Theme) -> Option<$crate::ink::InkStyle> {
                self.inner.ink(theme)
            }

            fn on_click(&self) -> Option<Msg> {
                self.inner.on_click()
            }

            fn positional_click(
                &self,
                local_x: f32,
                local_y: f32,
                width: f32,
                height: f32,
            ) -> Option<Msg> {
                self.inner.positional_click(local_x, local_y, width, height)
            }

            fn cursor_icon(
                &self,
                local_x: f32,
                local_y: f32,
                width: f32,
                height: f32,
            ) -> Option<$crate::interaction::Cursor> {
                self.inner.cursor_icon(local_x, local_y, width, height)
            }

            fn on_edit(
                &self,
                edit: &mut $crate::runtime::Edit,
                key: &$crate::interaction::Key,
            ) -> Option<Msg> {
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
                self.inner
                    .caret_vertical(width, cursor, down, page, goal_x)
            }

            fn selected_text(&self, edit: &$crate::runtime::Edit) -> Option<String> {
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

            // The **structural** questions the walk and the layout ask before they look
            // at a widget's children. A transparent wrapper that answered these for
            // itself would change how its content is laid out — a keyed stack would have
            // its layers put in flow instead of on top of one another — which is exactly
            // what a wrapper that claims to be transparent must not do.
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

            fn reorder_axis(&self) -> $crate::widget::ReorderAxis {
                self.inner.reorder_axis()
            }

            fn reorder_draggable(&self) -> bool {
                self.inner.reorder_draggable()
            }

            fn announce(&self) -> Option<String> {
                self.inner.announce()
            }

            fn scroll_content(&self) -> Option<&dyn $crate::widget::Widget<Msg>> {
                self.inner.scroll_content()
            }

            fn virtual_list(&self) -> Option<$crate::list::VirtualList<'_, Msg>> {
                self.inner.virtual_list()
            }

            fn page_view(&self) -> Option<$crate::pageview::PagedView<'_, Msg>> {
                self.inner.page_view()
            }

            fn intrinsic(&self) -> Option<($crate::constraints::IntrinsicAxis, Option<f32>)> {
                self.inner.intrinsic()
            }

            fn overflow_box(&self) -> Option<$crate::constraints::Overflow> {
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

            fn layout_builder(
                &self,
            ) -> Option<&dyn Fn(frus_core::Size) -> Box<dyn $crate::widget::Widget<Msg>>> {
                self.inner.layout_builder()
            }

            fn scroll_axis(&self) -> $crate::scroll::Axis {
                self.inner.scroll_axis()
            }

            fn scroll_physics(&self) -> Option<$crate::physics::ScrollPhysics> {
                self.inner.scroll_physics()
            }

            fn overlay(
                &self,
            ) -> Option<(&dyn $crate::widget::Widget<Msg>, $crate::portal::Placement)> {
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

            fn barrier(&self) -> Option<$crate::barrier::Barrier> {
                self.inner.barrier()
            }

            fn dismissible(&self) -> Option<$crate::dismiss::DismissSpec> {
                self.inner.dismissible()
            }

            fn on_dismissed(
                &self,
                direction: $crate::dismiss::DismissDirection,
            ) -> Option<Msg> {
                self.inner.on_dismissed(direction)
            }

            fn refresh(&self) -> Option<$crate::refresh::RefreshSpec> {
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

            fn on_key(
                &self,
                key: &$crate::interaction::Key,
            ) -> $crate::interaction::KeyResponse<Msg> {
                self.inner.on_key(key)
            }
        }
    };
}

pub(crate) use forward_transparent;

#[cfg(test)]
mod tests {
    use crate::widget::Widget;

    /// Every hook the trait has, forwarded — checked against the trait itself rather
    /// than against a list someone kept up to date by hand. A hook added to `Widget`
    /// and not to the macro is a wrapper that silently answers for itself.
    #[test]
    fn the_macro_forwards_every_hook_the_trait_declares() {
        let widget = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/widget.rs"))
            .expect("widget.rs");
        let macro_src =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/transparent.rs"))
                .expect("transparent.rs");
        // The trait's own declarations stop at its closing brace (the blanket impl for
        // `Box<dyn Widget>` follows and repeats every name).
        let trait_body = widget
            .split_once("pub trait Widget")
            .expect("the trait")
            .1
            .split_once("\n}\n")
            .expect("its end")
            .0;
        let names = |src: &str| -> Vec<String> {
            src.lines()
                .filter_map(|l| l.trim().strip_prefix("fn "))
                .filter_map(|l| l.split(['(', '<']).next())
                .map(str::to_owned)
                .collect()
        };
        // The two the macro deliberately leaves to its callers; each wrapper states both.
        let claimable = ["key", "theme_override"];
        let missing: Vec<String> = names(trait_body)
            .into_iter()
            .filter(|n| !claimable.contains(&n.as_str()))
            .filter(|n| !macro_src.contains(&format!("fn {n}(")))
            .collect();
        assert!(
            missing.is_empty(),
            "a transparent wrapper would answer these for itself: {missing:?}"
        );
    }

    /// And every wrapper states the two the macro left out — forwarding one of them by
    /// forgetting it is the same silence this module exists to prevent.
    #[test]
    fn every_wrapper_states_the_hooks_the_macro_leaves_out() {
        for file in ["keyed.rs", "themed.rs"] {
            let src = std::fs::read_to_string(format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR")))
                .expect("the wrapper's source");
            for hook in ["fn key(", "fn theme_override("] {
                assert!(src.contains(hook), "{file} says nothing about `{hook}`");
            }
        }
    }

    #[test]
    fn a_wrapped_widget_keeps_its_structure() {
        let stack = crate::Stack::<()>::new()
            .width(100.0)
            .height(50.0)
            .layer(crate::Container::new());
        let wrapped = crate::Keyed::new(1u64, stack);
        assert!(
            Widget::<()>::stack(&wrapped),
            "a wrapped stack is still a stack"
        );
    }
}
