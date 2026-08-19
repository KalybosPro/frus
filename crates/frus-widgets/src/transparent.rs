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
//!
//! ## The third hook: `restyle`
//!
//! [`crate::Expanded`] is transparent in every respect but one — it alters the flex item
//! its child *is*. So the **box** is the third thing a wrapper may legitimately claim,
//! and the macro asks for it the same way: an inherent method every wrapper writes, so
//! that the one which does not change the box has to say so.
//!
//! ```ignore
//! impl<Msg> Expanded<Msg> {
//!     fn restyle(&self, base: Style) -> Style {
//!         Style { flex_grow: self.flex, ..base }
//!     }
//! }
//! ```
//!
//! It is applied to `style` **and** `style_themed`, so a themed child keeps its themed
//! sizing and the two cannot drift apart.

/// Implements `Widget<Msg>` for a `{ inner: Box<dyn Widget<Msg>>, … }` wrapper by
/// delegating every hook to `inner`, except [`Widget::key`] and
/// [`Widget::theme_override`](crate::Widget::theme_override) — the two a wrapper may
/// legitimately answer for itself, which the caller therefore states, claimed or
/// forwarded.
macro_rules! forward_transparent {
    ($ty:ident { $($extra:item)* }) => {
        impl<Msg> $crate::widget::Widget<Msg> for $ty<Msg> {
            $($extra)*

            // Both go through `restyle`, the wrapper's own inherent method, so a themed
            // child keeps its themed sizing and the two cannot drift apart.
            fn style(&self) -> frus_layout::Style {
                self.restyle(self.inner.style())
            }

            fn style_themed(&self, theme: &$crate::theme::Theme) -> frus_layout::Style {
                self.restyle(self.inner.style_themed(theme))
            }

            fn build_themed(&self, theme: &$crate::theme::Theme) {
                // A deferred subtree inside a wrapper still has to be composed: without
                // this, a `ThemeBuilder` behind a `Keyed` never builds and the wrapper
                // reports a node with no children.
                self.inner.build_themed(theme)
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

            fn shortcut_bindings(
                &self,
            ) -> &[($crate::shortcuts::KeyStroke, $crate::shortcuts::Intent)] {
                self.inner.shortcut_bindings()
            }

            fn shortcut_callbacks(&self) -> &[($crate::shortcuts::KeyStroke, Msg)] {
                self.inner.shortcut_callbacks()
            }

            fn action_bindings(&self) -> &[($crate::shortcuts::Intent, Msg)] {
                self.inner.action_bindings()
            }

            fn action_listeners(&self) -> &[($crate::shortcuts::Intent, Msg)] {
                self.inner.action_listeners()
            }

            fn on_keystroke(
                &self,
            ) -> Option<std::rc::Rc<dyn Fn($crate::shortcuts::KeyStroke) -> Option<Msg>>> {
                self.inner.on_keystroke()
            }

            fn descendants_focusable(&self) -> bool {
                self.inner.descendants_focusable()
            }

            fn focus_skip_traversal(&self) -> bool {
                self.inner.focus_skip_traversal()
            }

            fn focus_order(&self) -> Option<f32> {
                self.inner.focus_order()
            }

            fn focus_group(&self) -> bool {
                self.inner.focus_group()
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

            fn layer_filter(
                &self,
                cx: $crate::widget::FilterContext,
            ) -> Option<frus_core::LayerFilter> {
                self.inner.layer_filter(cx)
            }

            fn backdrop_group(&self) -> bool {
                self.inner.backdrop_group()
            }

            fn text_baseline(&self, theme: &$crate::theme::Theme) -> Option<f32> {
                self.inner.text_baseline(theme)
            }

            fn ignores_baseline(&self) -> bool {
                self.inner.ignores_baseline()
            }

            fn baseline_target(&self) -> Option<f32> {
                self.inner.baseline_target()
            }

            fn main_axis_fill(&self) -> Option<frus_layout::FlexDirection> {
                self.inner.main_axis_fill()
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

    /// The **same instrument, second target**: `Box<dyn Widget>` also claims to be the
    /// widget it holds, through a hand-written blanket impl with no macro behind it.
    ///
    /// Milestone 324 found it two hooks short — `build_themed` and `repaint_boundary` —
    /// so a boxed `ThemeBuilder` asked to build would quietly do nothing and a boxed
    /// repaint boundary would report that it was not one. Neither was reachable, because
    /// every walk in the framework takes `&dyn Widget` and dispatches virtually; both were
    /// waiting for the first caller to hold a `Box` instead. That is exactly the shape of
    /// bug this module exists to prevent, and the blanket impl had no guard at all.
    #[test]
    fn the_boxed_widget_forwards_every_hook_the_trait_declares() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/widget.rs"))
            .expect("widget.rs");
        let trait_body = src
            .split_once("pub trait Widget")
            .expect("the trait")
            .1
            .split_once(
                "
}
",
            )
            .expect("its end")
            .0;
        let blanket = src
            .split_once("impl<Msg> Widget<Msg> for Box<dyn Widget<Msg>>")
            .expect("the blanket impl")
            .1;
        let names = |src: &str| -> Vec<String> {
            src.lines()
                .filter_map(|l| l.trim().strip_prefix("fn "))
                .filter_map(|l| l.split(['(', '<']).next())
                .map(str::to_owned)
                .collect()
        };
        let forwarded = names(blanket);
        let missing: Vec<String> = names(trait_body)
            .into_iter()
            .filter(|n| !forwarded.contains(n))
            .collect();
        assert!(
            missing.is_empty(),
            "a boxed widget answers these for itself: {missing:?}"
        );
    }

    /// And every wrapper states the two the macro left out — forwarding one of them by
    /// forgetting it is the same silence this module exists to prevent.
    #[test]
    fn every_wrapper_states_the_hooks_the_macro_leaves_out() {
        // Found, not listed: a wrapper added to the crate is covered the day it is
        // written, which a hand-kept array is not.
        let mut checked = 0;
        for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
            .expect("the source directory")
        {
            let path = entry.expect("a directory entry").path();
            let file = path
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .to_string();
            if file == "transparent.rs" {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("the wrapper's source");
            if !src.contains("forward_transparent!(") {
                continue;
            }
            checked += 1;
            for hook in ["fn key(", "fn theme_override(", "fn restyle("] {
                assert!(src.contains(hook), "{file} says nothing about `{hook}`");
            }
        }
        assert!(
            checked >= 3,
            "only {checked} wrappers found — did the search break?"
        );
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
