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

/// Une destination de navigation (glyphe + libellé), peinte selon son état.
struct NavItem<Msg> {
    icon: String,
    label: String,
    selected: bool,
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
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Construit les éléments de navigation depuis les destinations déclarées.
fn build_items<Msg: Clone + 'static>(
    items: &[(String, String)],
    selected: usize,
    on_select: &dyn Fn(usize) -> Msg,
    rail: bool,
) -> Vec<Box<dyn Widget<Msg>>> {
    items
        .iter()
        .enumerate()
        .map(|(i, (icon, label))| {
            Box::new(NavItem {
                icon: icon.clone(),
                label: label.clone(),
                selected: i == selected,
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
    items: Vec<(String, String)>,
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
        self.items.push((icon.into(), label.into()));
        self.children = build_items(&self.items, self.selected, &*self.on_select, true);
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
    items: Vec<(String, String)>,
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
        self.items.push((icon.into(), label.into()));
        self.children = build_items(&self.items, self.selected, &*self.on_select, false);
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
    fn bottom_bar_items_are_flexible() {
        let bar = BottomBar::new(0, Msg::Go).item("H", "Home").item("S", "Search");
        let children = Widget::<Msg>::children(&bar);
        assert_eq!(children.len(), 2);
        // Élément de barre : partage la largeur (flex_grow > 0), pas de largeur fixe.
        assert_eq!(Widget::<Msg>::style(&*children[0]).flex_grow, 1.0);
        assert_eq!(children[1].on_click(), Some(Msg::Go(1)));
    }
}
