//! [`Dropdown`] : une liste déroulante **contrôlée** dont les options flottent
//! au-dessus du reste (via le mécanisme d'overlay), sous l'en-tête.
//!
//! Largeur réglable ([`width`](Dropdown::width)), option **sélectionnée** surlignée et
//! cochée ([`selected`](Dropdown::selected)), et navigation **clavier** : l'en-tête et
//! les options prennent le focus (Entrée ouvre / choisit, les flèches parcourent).

use frus_core::{Path, Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::icons::IconName;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const DEFAULT_WIDTH: f32 = 240.0;
const ROW_H: f32 = 40.0;
const PAD_X: f32 = 12.0;
const SIZE: f32 = 18.0;

/// Une ligne (en-tête ou option).
struct Row<Msg> {
    label: String,
    width: f32,
    is_header: bool,
    /// Option actuellement sélectionnée (surlignée + cochée). Ignoré pour l'en-tête.
    selected: bool,
    on_click: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Option sélectionnée : fond teinté primary ; survol par-dessus (couche d'état).
        let base = if self.selected {
            theme.surface.lerp(theme.primary, 0.14)
        } else {
            theme.surface
        };
        let bg = theme.state_layer(base, theme.on_surface, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));

        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(Point::new(bounds.x + PAD_X, ty), self.label.clone(), SIZE, theme.on_surface.fade(o));

        if self.is_header {
            // Chevron « ▾ » vectoriel (triangle pointant vers le bas), à droite.
            let cx = bounds.x + self.width - PAD_X - 4.0;
            let cy = bounds.y + ROW_H * 0.5;
            let (w, h) = (5.0, 3.0);
            let tri = Path::new()
                .move_to(Point::new(cx - w, cy - h))
                .line_to(Point::new(cx + w, cy - h))
                .line_to(Point::new(cx, cy + h))
                .close();
            scene.fill_path(&tri, theme.muted.fade(o));
        } else if self.selected {
            // Coche de l'option sélectionnée, à droite.
            let size = 18.0;
            let scale = size / 24.0;
            let x = bounds.x + self.width - PAD_X - size;
            let y = bounds.y + (ROW_H - size) * 0.5;
            let path = IconName::Check.path().scaled(scale).translated(x, y);
            scene.fill_path(&path, theme.primary.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }

    fn focusable(&self) -> bool {
        self.on_click.is_some()
    }
}

/// Une liste déroulante à sélection unique (menu flottant).
pub struct Dropdown<Msg> {
    header_label: String,
    on_toggle: Msg,
    width: f32,
    selected: Option<usize>,
    open: bool,
    labels: Vec<String>,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Dropdown<Msg> {
    /// Crée une liste : libellé courant + message de bascule (ouvrir/fermer).
    pub fn new(selected_label: impl Into<String>, on_toggle: Msg) -> Self {
        let mut dropdown = Self {
            header_label: selected_label.into(),
            on_toggle,
            width: DEFAULT_WIDTH,
            selected: None,
            open: false,
            labels: Vec::new(),
            on_select: None,
            children: Vec::new(),
        };
        dropdown.rebuild();
        dropdown
    }

    /// Largeur de l'en-tête et du menu, en pixels logiques (défaut 240).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Index de l'option **sélectionnée** (surlignée + cochée dans le menu).
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self.rebuild();
        self
    }

    /// Définit les options ; si `open`, elles flottent sous l'en-tête. `on_select` mappe
    /// l'index choisi vers un message.
    pub fn options(mut self, open: bool, labels: &[&str], on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.open = open;
        self.labels = labels.iter().map(|s| s.to_string()).collect();
        self.on_select = Some(Box::new(on_select));
        self.rebuild();
        self
    }

    /// Régénère l'en-tête (et le menu si ouvert) depuis l'état courant.
    fn rebuild(&mut self) {
        let header = Row {
            label: self.header_label.clone(),
            width: self.width,
            is_header: true,
            selected: false,
            on_click: Some(self.on_toggle.clone()),
        };
        self.children = vec![Box::new(header)];

        if self.open && !self.labels.is_empty() {
            let mut menu = Flex::column().gap(4.0);
            for (index, label) in self.labels.iter().enumerate() {
                let on_click = self.on_select.as_ref().map(|f| f(index));
                menu = menu.child(Row {
                    label: label.clone(),
                    width: self.width,
                    is_header: false,
                    selected: self.selected == Some(index),
                    on_click,
                });
            }
            self.children.push(Box::new(menu));
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Dropdown<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children.get(1).map(|menu| (menu.as_ref(), Placement::Below))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle,
        Select(usize),
    }

    #[test]
    fn closed_has_no_overlay_open_floats_options() {
        let closed = Dropdown::new("Pick one", Msg::Toggle).options(false, &["A", "B"], Msg::Select);
        assert!(Widget::<Msg>::overlay(&closed).is_none(), "fermée : pas d'overlay");

        let open = Dropdown::new("Pick one", Msg::Toggle).options(true, &["A", "B"], Msg::Select);
        assert!(Widget::<Msg>::overlay(&open).is_some(), "ouverte : menu flottant");
        let menu = &Widget::<Msg>::children(&open)[1];
        assert_eq!(menu.children().len(), 2);
        assert_eq!(menu.children()[1].on_click(), Some(Msg::Select(1)));
    }

    #[test]
    fn header_and_options_are_keyboard_focusable() {
        let open = Dropdown::new("Pick", Msg::Toggle).options(true, &["A", "B"], Msg::Select);
        // En-tête focusable (ouvre au clavier) + 2 options.
        assert!(Widget::<Msg>::children(&open)[0].focusable());
        let menu = &Widget::<Msg>::children(&open)[1];
        assert!(menu.children()[0].focusable() && menu.children()[1].focusable());
    }

    #[test]
    fn selected_option_is_highlighted_and_checked() {
        let open = Dropdown::new("Pick", Msg::Toggle)
            .selected(1)
            .options(true, &["A", "B"], Msg::Select)
            .width(200.0);
        // Le menu est un overlay : on le rend seul pour lire ses primitives.
        let (menu, _) = Widget::<Msg>::overlay(&open).unwrap();
        let ui = build_ui(menu, Size::new(220.0, 120.0), &Runtime::default(), &Theme::default());
        let theme = Theme::default();
        // Coche de l'option sélectionnée (chemin rempli).
        let has_check = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Path { .. }));
        assert!(has_check, "l'option sélectionnée est cochée");
        // Fond teinté primary de l'option sélectionnée.
        let sel = theme.surface.lerp(theme.primary, 0.14);
        let has_tint = ui.scene().primitives().iter().any(|p| matches!(
            p,
            Primitive::Rect { color, .. } if color.fade(1.0) == sel.fade(1.0)
        ));
        assert!(has_tint, "l'option sélectionnée est surlignée");
    }
}
