//! [`NavRail`] et [`BottomBar`] : les deux présentations d'une **navigation
//! principale** à sélection unique. Même API (`new(selected, on_select).item(
//! icon, label)`) ; [`crate::NavScaffold`] choisit l'une ou l'autre selon la
//! taille. L'« icône » est un glyphe texte (le framework n'a pas de police
//! d'icônes) : emoji ou caractère Unicode.

use frus_core::{Color, Insets, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Largeur d'un rail vertical, en px logiques.
pub(crate) const RAIL_WIDTH: f32 = 76.0;
/// Hauteur d'une barre de navigation basse, en px logiques.
pub(crate) const BAR_HEIGHT: f32 = 60.0;
const ITEM_HEIGHT: f32 = 58.0;
const ICON_SIZE: f32 = 22.0;
const LABEL_SIZE: f32 = 12.0;
const BADGE_SIZE: f32 = 10.0;
/// Rouge de notification (constant : une pastille d'alerte se lit rouge quel
/// que soit le thème).
const BADGE_COLOR: Color = Color::rgb(0.90, 0.24, 0.24);

/// Une destination de navigation (glyphe + libellé), peinte selon son état.
struct NavItem<Msg> {
    icon: String,
    label: String,
    selected: bool,
    /// Compteur de notifications (pastille sur l'icône). `0`/`None` = rien.
    badge: Option<u32>,
    /// `true` = élément de rail (largeur fixe) ; `false` = élément de barre (flex).
    rail: bool,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for NavItem<Msg> {
    fn style(&self) -> Style {
        if self.rail {
            Style {
                width: Dimension::Length(RAIL_WIDTH),
                height: Dimension::Length(ITEM_HEIGHT),
                ..Default::default()
            }
        } else {
            // Dans une barre, les éléments se partagent la largeur également.
            Style {
                flex_grow: 1.0,
                height: Dimension::Length(BAR_HEIGHT),
                ..Default::default()
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let icon_m = frus_text::measure(&self.icon, ICON_SIZE);
        let label_m = frus_text::measure(&self.label, LABEL_SIZE);
        let gap = 2.0;
        let total_h = icon_m.height + gap + label_m.height;
        let top = bounds.y + ((bounds.height - total_h) * 0.5).max(0.0);

        // Pastille de fond : pleine si sélectionné, discrète au survol.
        let pill_w = icon_m.width + 28.0;
        let pill_h = icon_m.height + 8.0;
        let pill = Rect::new(
            bounds.x + (bounds.width - pill_w) * 0.5,
            top - 4.0,
            pill_w,
            pill_h,
        );
        if self.selected {
            scene.draw_rect(pill, theme.primary.fade(0.16 * o), pill_h * 0.5, 0.0, Color::TRANSPARENT);
        } else if status.hover_progress > 0.0 {
            let a = 0.12 * status.hover_progress * o;
            scene.draw_rect(pill, theme.muted.fade(a), pill_h * 0.5, 0.0, Color::TRANSPARENT);
        }

        let color = if self.selected { theme.primary } else { theme.muted };
        scene.text(
            Point::new(bounds.x + (bounds.width - icon_m.width) * 0.5, top),
            self.icon.clone(),
            ICON_SIZE,
            color.fade(o),
        );
        scene.text(
            Point::new(
                bounds.x + (bounds.width - label_m.width) * 0.5,
                top + icon_m.height + gap,
            ),
            self.label.clone(),
            LABEL_SIZE,
            color.fade(o),
        );

        // Pastille de notification, ancrée au coin haut-droit du glyphe d'icône.
        if let Some(count) = self.badge.filter(|&n| n > 0) {
            let text = if count > 99 { "99+".to_string() } else { count.to_string() };
            let m = frus_text::measure(&text, BADGE_SIZE);
            let bw = (m.width + 8.0).max(m.height + 4.0);
            let bh = m.height + 4.0;
            let icon_right = bounds.x + (bounds.width + icon_m.width) * 0.5;
            let bx = (icon_right - bw * 0.4).min(bounds.x + bounds.width - bw);
            let by = top - bh * 0.35;
            let rect = Rect::new(bx, by, bw, bh);
            scene.draw_rect(rect, BADGE_COLOR.fade(o), bh * 0.5, 0.0, Color::TRANSPARENT);
            scene.text(
                Point::new(bx + (bw - m.width) * 0.5, by + 2.0),
                text,
                BADGE_SIZE,
                Color::WHITE.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Une destination déclarée : glyphe, libellé, compteur de badge éventuel.
type Destination = (String, String, Option<u32>);

/// Construit les éléments de navigation depuis les destinations déclarées.
fn build_items<Msg: Clone + 'static>(
    items: &[Destination],
    selected: usize,
    on_select: &dyn Fn(usize) -> Msg,
    rail: bool,
) -> Vec<Box<dyn Widget<Msg>>> {
    items
        .iter()
        .enumerate()
        .map(|(i, (icon, label, badge))| {
            Box::new(NavItem {
                icon: icon.clone(),
                label: label.clone(),
                selected: i == selected,
                badge: *badge,
                rail,
                message: on_select(i),
            }) as Box<dyn Widget<Msg>>
        })
        .collect()
}

/// Rail de navigation **vertical** (tablette / bureau).
pub struct NavRail<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    items: Vec<Destination>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavRail<Msg> {
    /// Crée un rail : `selected` = index actif, `on_select(i)` au clic.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            items: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Ajoute une destination (glyphe + libellé).
    pub fn item(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.items.push((icon.into(), label.into(), None));
        self.children = build_items(&self.items, self.selected, &*self.on_select, true);
        self
    }

    /// Ajoute un compteur de notifications à la **dernière** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.items.last_mut() {
            last.2 = Some(count);
            self.children = build_items(&self.items, self.selected, &*self.on_select, true);
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavRail<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(RAIL_WIDTH),
            flex_direction: FlexDirection::Column,
            align: Align::Center,
            padding: Insets::new(8.0, 0.0, 8.0, 0.0),
            gap: 4.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Séparateur vertical sur le bord droit.
        let x = bounds.x + bounds.width - 1.0;
        scene.fill_rect(
            Rect::new(x, bounds.y, 1.0, bounds.height),
            theme.border.fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Barre de navigation **horizontale** en bas (téléphone).
pub struct BottomBar<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    items: Vec<Destination>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> BottomBar<Msg> {
    /// Crée une barre : `selected` = index actif, `on_select(i)` au clic.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            items: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Ajoute une destination (glyphe + libellé).
    pub fn item(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.items.push((icon.into(), label.into(), None));
        self.children = build_items(&self.items, self.selected, &*self.on_select, false);
        self
    }

    /// Ajoute un compteur de notifications à la **dernière** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.items.last_mut() {
            last.2 = Some(count);
            self.children = build_items(&self.items, self.selected, &*self.on_select, false);
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for BottomBar<Msg> {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(BAR_HEIGHT),
            flex_direction: FlexDirection::Row,
            justify: Justify::SpaceAround,
            align: Align::Stretch,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Séparateur horizontal sur le bord haut.
        scene.fill_rect(
            Rect::new(bounds.x, bounds.y, bounds.width, 1.0),
            theme.border.fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    #[test]
    fn rail_items_emit_index_and_track_selection() {
        let rail = NavRail::new(1, Msg::Go)
            .item("H", "Home")
            .item("S", "Search")
            .item("P", "Profile");
        let children = Widget::<Msg>::children(&rail);
        assert_eq!(children.len(), 3);
        assert_eq!(children[2].on_click(), Some(Msg::Go(2)));
    }

    #[test]
    fn badge_decorates_last_item_and_paints_counter() {
        let rail = NavRail::new(0, Msg::Go)
            .item("H", "Home")
            .item("M", "Mail")
            .badge(5);
        let children = Widget::<Msg>::children(&rail);
        // Le badge peint une pastille + le texte du compteur sur l'élément visé.
        let mut scene = Scene::new();
        children[1].paint(
            Rect::new(0.0, 0.0, RAIL_WIDTH, ITEM_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "5")));
        // L'élément sans badge ne peint pas ce compteur.
        let mut bare = Scene::new();
        children[0].paint(
            Rect::new(0.0, 0.0, RAIL_WIDTH, ITEM_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut bare,
        );
        assert!(!bare
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "5")));
    }

    #[test]
    fn badge_over_99_is_capped() {
        let bar = BottomBar::new(0, Msg::Go).item("M", "Mail").badge(150);
        let children = Widget::<Msg>::children(&bar);
        let mut scene = Scene::new();
        children[0].paint(
            Rect::new(0.0, 0.0, 80.0, BAR_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "99+")));
    }

    #[test]
    fn bottom_bar_items_are_flexible() {
        let bar = BottomBar::new(0, Msg::Go).item("H", "Home").item("S", "Search");
        let children = Widget::<Msg>::children(&bar);
        assert_eq!(children.len(), 2);
        // Élément de barre : partage la largeur (flex_grow > 0), pas de largeur fixe.
        assert_eq!(Widget::<Msg>::style(&*children[0]).flex_grow, 1.0);
        assert_eq!(children[1].on_click(), Some(Msg::Go(1)));
    }
}
