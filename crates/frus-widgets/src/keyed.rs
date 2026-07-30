//! [`Keyed`] : un wrapper **transparent** qui donne à un widget une **identité
//! stable** (par clé), indépendante de sa position parmi ses frères.
//!
//! Sans clé, l'identité est positionnelle : supprimer un élément au milieu d'une
//! liste décale l'identité des suivants, et leur état retenu (survol, focus,
//! curseur, animations, fondu de sortie) « saute ». Envelopper chaque élément
//! dans un `Keyed` (clé = son id métier stable) résout ça.

use std::hash::{Hash, Hasher};

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Key, Status};
use crate::portal::Placement;
use crate::runtime::Edit;
use crate::scroll::Axis;
use crate::theme::Theme;
use crate::widget::Widget;

/// Enveloppe un widget d'une clé d'identité stable (délègue tout le reste).
pub struct Keyed<Msg> {
    key: u64,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> Keyed<Msg> {
    /// Enveloppe `inner` avec la clé `key` (n'importe quel type hachable).
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

    fn debug_name(&self) -> &'static str {
        // Wrapper transparent : l'inspecteur montre le widget enveloppé.
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

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32) -> Option<Msg> {
        self.inner.positional_click(local_x, local_y, width)
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

    fn layout_builder(&self) -> Option<&dyn Fn(frus_core::Size) -> Box<dyn Widget<Msg>>> {
        self.inner.layout_builder()
    }

    fn scroll_axis(&self) -> Axis {
        self.inner.scroll_axis()
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

    #[test]
    fn reports_key_and_delegates() {
        let inner = Container::<()>::new().width(40.0).child(Text::new("x"));
        let keyed = Keyed::new(99u64, inner);
        // Renvoie une clé.
        assert!(Widget::<()>::key(&keyed).is_some());
        // Délègue les enfants (le Container a un enfant).
        assert_eq!(Widget::<()>::children(&keyed).len(), 1);
    }

    #[test]
    fn same_key_same_hash() {
        let a: Keyed<()> = Keyed::new("todo-3", Container::new());
        let b: Keyed<()> = Keyed::new("todo-3", Container::new());
        assert_eq!(Widget::<()>::key(&a), Widget::<()>::key(&b));
    }
}
