//! [`Portal`] : affiche un contenu **au-dessus** du reste (hors flux de layout,
//! non découpé par les parents). Base des menus flottants, tooltips et modales.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Où placer l'overlay par rapport à son ancre / à la fenêtre.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Juste sous l'ancre (menu déroulant).
    Below,
    /// Centré dans la fenêtre, avec un voile (modale).
    Center,
    /// Au-dessus de l'ancre, **uniquement si l'ancre est survolée** (tooltip).
    Tooltip,
    /// Panneau plein-hauteur accolé au bord **gauche** de la fenêtre, avec un
    /// voile (tiroir latéral). Voir [`crate::Drawer`].
    Left,
    /// Panneau plein-hauteur accolé au bord **droit** de la fenêtre, avec un
    /// voile (tiroir latéral). Voir [`crate::Drawer`].
    Right,
}

/// Un portail : une **ancre** (dans le flux) et un **overlay** flottant optionnel.
pub struct Portal<Msg> {
    /// `[ancre]` ou `[ancre, overlay]`.
    children: Vec<Box<dyn Widget<Msg>>>,
    placement: Placement,
    /// Message émis au clic sur le voile (modale) pour fermer.
    on_dismiss: Option<Msg>,
}

impl<Msg> Portal<Msg> {
    /// Crée un portail autour d'une ancre (rendue normalement).
    pub fn new(anchor: impl Widget<Msg> + 'static) -> Self {
        Self {
            children: vec![Box::new(anchor)],
            placement: Placement::Below,
            on_dismiss: None,
        }
    }

    /// Ajoute le contenu flottant et son placement.
    pub fn overlay(mut self, content: impl Widget<Msg> + 'static, placement: Placement) -> Self {
        self.children.truncate(1);
        self.children.push(Box::new(content));
        self.placement = placement;
        self
    }

    /// Message émis au clic **hors** du contenu (sur le voile) — pour fermer.
    pub fn dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Portal<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|content| (content.as_ref(), self.placement))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Runtime, Size, Text};

    #[test]
    fn overlay_present_only_when_set() {
        let bare: Portal<()> = Portal::new(Container::<()>::new());
        assert!(Widget::<()>::overlay(&bare).is_none());

        let with: Portal<()> =
            Portal::new(Container::<()>::new()).overlay(Text::new("tip"), Placement::Center);
        assert!(Widget::<()>::overlay(&with).is_some());
    }

    #[test]
    fn center_overlay_draws_scrim_and_content() {
        // Un overlay Center dessine un voile plein-écran + le contenu par-dessus.
        let portal: Portal<()> =
            Portal::new(Container::<()>::new().width(20.0).height(20.0)).overlay(
                Container::<()>::new().width(100.0).height(60.0).color(frus_core::Color::WHITE),
                Placement::Center,
            );
        let ui = crate::build_ui(
            &portal,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &crate::Theme::default(),
        );
        // Au moins : le voile (plein écran) + le contenu de l'overlay.
        let full_screen = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 400.0));
        assert!(full_screen, "le voile plein écran doit être présent");
    }
}
