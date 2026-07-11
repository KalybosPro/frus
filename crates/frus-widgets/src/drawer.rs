//! [`Drawer`] : un **tiroir latéral** rétractable — le 3ᵉ palier de navigation
//! Material, en complément de `NavRail` (rail) et `BottomBar` (barre). Le corps
//! reste visible en fond ; quand le tiroir est ouvert, un panneau plein-hauteur
//! glisse depuis la gauche par-dessus, avec un voile qui le referme au clic.
//!
//! ```ignore
//! Drawer::new(app.menu_open)
//!     .on_dismiss(Msg::CloseMenu)
//!     .panel(nav_list)   // contenu du tiroir
//!     .body(main_screen) // contenu de fond (toujours visible)
//! ```

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// Largeur d'un tiroir latéral, en px logiques.
pub const DRAWER_WIDTH: f32 = 280.0;

/// Panneau interne du tiroir : plein-hauteur, fond thématisé, bordure droite.
struct DrawerPanel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for DrawerPanel<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(DRAWER_WIDTH),
            // La hauteur se déploie à toute la fenêtre (placement `Left`).
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Surface opaque du tiroir + fin liseré sur le bord droit.
        scene.fill_rect(bounds, theme.surface.fade(o));
        let x = bounds.x + bounds.width - 1.0;
        scene.fill_rect(Rect::new(x, bounds.y, 1.0, bounds.height), theme.border.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Tiroir latéral rétractable : corps de fond + panneau flottant à gauche.
pub struct Drawer<Msg> {
    open: bool,
    on_dismiss: Option<Msg>,
    panel: Option<Box<dyn Widget<Msg>>>,
    /// `[corps]` — le fond, toujours rendu dans le flux.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Drawer<Msg> {
    /// Crée un tiroir : `open` indique s'il est déployé.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            on_dismiss: None,
            panel: None,
            children: Vec::new(),
        }
    }

    /// Message émis au clic sur le voile (hors du panneau) — pour refermer.
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Définit le **contenu du tiroir** (la navigation, en général).
    pub fn panel(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.panel = Some(Box::new(DrawerPanel {
            children: vec![Box::new(content)],
        }));
        self
    }

    /// Définit le **corps de fond** (toujours visible) et finalise le tiroir.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        self.children = vec![Box::new(body)];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Drawer<Msg> {
    fn style(&self) -> Style {
        // Remplit son parent pour que le corps occupe toute la place.
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
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
        if self.open {
            self.panel.as_ref().map(|p| (p.as_ref(), Placement::Left))
        } else {
            None
        }
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        if self.open {
            self.on_dismiss.clone()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size, Text};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
    }

    #[test]
    fn overlay_present_only_when_open() {
        let closed = Drawer::new(false)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert!(Widget::<Msg>::overlay(&closed).is_none());
        assert!(Widget::<Msg>::overlay_dismiss(&closed).is_none());

        let open = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert!(matches!(
            Widget::<Msg>::overlay(&open),
            Some((_, Placement::Left))
        ));
        assert_eq!(Widget::<Msg>::overlay_dismiss(&open), Some(Msg::Close));
    }

    #[test]
    fn open_drawer_draws_scrim_and_full_height_panel() {
        let drawer = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &drawer,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Voile plein écran (largeur ≥ fenêtre) présent.
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0),
        );
        assert!(scrim, "le voile doit couvrir la fenêtre");
        // Panneau plein-hauteur (hauteur ≈ fenêtre) à la largeur du tiroir.
        let panel = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && rect.height >= 399.0)
        });
        assert!(panel, "le panneau doit se déployer sur toute la hauteur");
    }

    #[test]
    fn closed_drawer_clicking_does_not_dismiss() {
        // Fermé : aucun overlay, donc pas de cible de fermeture plein écran.
        let drawer = Drawer::new(false)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new().width(50.0).height(50.0));
        let ui = build_ui(
            &drawer,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(ui.hit(frus_core::Point::new(250.0, 200.0)).is_none());
    }
}
