//! [`Responsive`] : choisit un sous-arbre selon la [`SizeClass`] courante.
//!
//! `responsive(width).compact(a).medium(b).expanded(c)` sélectionne la variante
//! correspondant au palier de largeur, avec **repli gracieux** : si le palier
//! exact n'est pas fourni, le plus proche est utilisé (en préférant plus petit).
//! Le widget résultant **délègue tout** à la variante choisie (comme [`Keyed`]).
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

/// Sélectionne un sous-arbre selon la classe de taille (délègue au choisi).
pub struct Responsive<Msg> {
    class_rank: u8,
    inner: Option<Box<dyn Widget<Msg>>>,
    inner_rank: u8,
}

/// Distance d'un palier à la classe cible : minimiser `(écart, est_au-dessus)`
/// choisit le plus proche, en préférant un palier plus petit à égalité d'écart.
fn closeness(rank: u8, class: u8) -> (u8, bool) {
    (rank.abs_diff(class), rank > class)
}

impl<Msg> Responsive<Msg> {
    /// Construit un sélecteur pour la classe `class`.
    pub fn new(class: SizeClass) -> Self {
        Self {
            class_rank: class.rank(),
            inner: None,
            inner_rank: 0,
        }
    }

    /// Considère `widget` pour le palier de rang `rank`, en gardant le meilleur
    /// candidat vu jusqu'ici (indépendant de l'ordre des appels).
    fn consider(mut self, rank: u8, widget: impl Widget<Msg> + 'static) -> Self {
        let better = self.inner.is_none()
            || closeness(rank, self.class_rank) < closeness(self.inner_rank, self.class_rank);
        if better {
            self.inner = Some(Box::new(widget));
            self.inner_rank = rank;
        }
        self
    }

    /// Variante pour le palier `Compact` (< 600 px).
    pub fn compact(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.consider(SizeClass::Compact.rank(), widget)
    }

    /// Variante pour le palier `Medium` (600–840 px).
    pub fn medium(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.consider(SizeClass::Medium.rank(), widget)
    }

    /// Variante pour le palier `Expanded` (≥ 840 px).
    pub fn expanded(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.consider(SizeClass::Expanded.rank(), widget)
    }
}

/// Sélecteur responsive pour une largeur donnée (px logiques).
///
/// `responsive(w).compact(a).medium(b).expanded(c)` — voir [`Responsive`].
pub fn responsive<Msg>(width: f32) -> Responsive<Msg> {
    Responsive::new(SizeClass::from_width(width))
}

impl<Msg> Widget<Msg> for Responsive<Msg> {
    fn style(&self) -> Style {
        self.inner.as_ref().map(|w| w.style()).unwrap_or_default()
    }

    fn debug_name(&self) -> &'static str {
        // Wrapper transparent : l'inspecteur montre le widget réalisé.
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
        self.inner.as_ref().and_then(|w| w.text_metrics(width, cursor))
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
        self.inner.as_ref().and_then(|w| w.caret_vertical(width, cursor, down, page, goal_x))
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

    fn virtual_list(&self) -> Option<crate::list::VirtualList<'_, Msg>> {
        self.inner.as_ref().and_then(|w| w.virtual_list())
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

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.on_drag_delta(dx))
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

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.inner.as_ref().and_then(|w| w.overlay())
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.inner.as_ref().and_then(|w| w.overlay_dismiss())
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

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        self.inner.as_ref().and_then(|w| w.measure())
    }

    fn measure_key(&self) -> Option<u64> {
        self.inner.as_ref().and_then(|w| w.measure_key())
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

    /// Largeur de style de la variante choisie (chaque palier a une largeur
    /// distincte pour l'identifier).
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
        // Seul `expanded` fourni → utilisé pour toutes les largeurs.
        let only_expanded = |w: f32| responsive::<()>(w).expanded(Container::new().width(300.0));
        assert_eq!(chosen_width(&only_expanded(300.0)), Dimension::Length(300.0));

        // compact + expanded, largeur medium (rang 1) : compact (rang 0, écart 1,
        // en-dessous) préféré à expanded (rang 2, écart 1, au-dessus).
        let two = responsive::<()>(700.0)
            .compact(Container::new().width(100.0))
            .expanded(Container::new().width(300.0));
        assert_eq!(chosen_width(&two), Dimension::Length(100.0));
    }
}
