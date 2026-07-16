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

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.inner.children()
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        self.inner.paint(bounds, status, theme, scene);
    }

    fn on_click(&self) -> Option<Msg> {
        self.inner.on_click()
    }

    fn key(&self) -> Option<u64> {
        Some(self.key)
    }

    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        self.inner.on_edit(edit, key)
    }

    fn cursor_at(&self, local_x: f32, width: f32, scroll_cursor: usize) -> Option<usize> {
        self.inner.cursor_at(local_x, width, scroll_cursor)
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        self.inner.selected_text(edit)
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

    fn anim_target(&self) -> Option<f32> {
        self.inner.anim_target()
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
