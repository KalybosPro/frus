//! [`Drawer`] : un **tiroir latéral** rétractable — le 3ᵉ palier de navigation
//! Material, en complément de `NavRail` (rail) et `BottomBar` (barre).
//!
//! Deux modes :
//! - **modal** (par défaut) : le corps reste visible en fond ; quand le tiroir
//!   est ouvert, un panneau plein-hauteur glisse depuis un bord (gauche ou
//!   droite) par-dessus, avec un voile qui le referme au clic. L'ouverture est
//!   animée automatiquement (voir jalon 46).
//! - **permanent** ([`Drawer::permanent`]) : le panneau est **accosté** dans le
//!   flux, toujours visible à côté du corps (sans voile). Typiquement activé au
//!   palier `Expanded`.
//!
//! ```ignore
//! Drawer::new(app.menu_open)
//!     .on_dismiss(Msg::CloseMenu)
//!     .permanent(class == SizeClass::Expanded)
//!     .panel(nav_list)   // contenu du tiroir
//!     .body(main_screen) // contenu de fond (toujours visible)
//! ```

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// Largeur d'un tiroir latéral, en px logiques.
pub const DRAWER_WIDTH: f32 = 280.0;

/// Panneau interne du tiroir : plein-hauteur, fond thématisé, liseré sur le bord
/// **intérieur** (droit pour un tiroir gauche, gauche pour un tiroir droit).
struct DrawerPanel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// Dessine le liseré sur le bord gauche (tiroir accosté à droite).
    border_left: bool,
}

impl<Msg: Clone> Widget<Msg> for DrawerPanel<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(DRAWER_WIDTH),
            // La hauteur se déploie à toute la fenêtre (placement latéral) ou à
            // la hauteur de la rangée (mode permanent).
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
        // Surface opaque du tiroir + fin liseré sur le bord intérieur.
        scene.fill_rect(bounds, theme.surface.fade(o));
        let x = if self.border_left { bounds.x } else { bounds.x + bounds.width - 1.0 };
        scene.fill_rect(Rect::new(x, bounds.y, 1.0, bounds.height), theme.border.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Tiroir latéral rétractable : corps de fond + panneau (modal ou accosté).
pub struct Drawer<Msg> {
    open: bool,
    right: bool,
    permanent: bool,
    on_dismiss: Option<Msg>,
    /// Contenu du tiroir, fourni par l'appelant (avant enrobage `DrawerPanel`).
    panel_content: Option<Box<dyn Widget<Msg>>>,
    /// Panneau modal (mode non permanent), enrobé, prêt pour l'overlay.
    modal_panel: Option<Box<dyn Widget<Msg>>>,
    /// Enfants dans le flux : `[corps]` (modal) ou `[panneau, corps]` (permanent).
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Drawer<Msg> {
    /// Crée un tiroir : `open` indique s'il est déployé (ignoré en mode permanent).
    pub fn new(open: bool) -> Self {
        Self {
            open,
            right: false,
            permanent: false,
            on_dismiss: None,
            panel_content: None,
            modal_panel: None,
            children: Vec::new(),
        }
    }

    /// Message émis au clic sur le voile (hors du panneau) — pour refermer.
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Accoste le tiroir au bord **droit** (par défaut : gauche).
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }

    /// Rend le tiroir **permanent** (accosté dans le flux, sans voile) quand
    /// `permanent` est vrai — typiquement au palier `Expanded`.
    pub fn permanent(mut self, permanent: bool) -> Self {
        self.permanent = permanent;
        self
    }

    /// Définit le **contenu du tiroir** (la navigation, en général).
    pub fn panel(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.panel_content = Some(Box::new(content));
        self
    }

    /// Définit le **corps de fond** (toujours visible) et finalise le tiroir.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        let panel = self.panel_content.take().map(|content| {
            Box::new(DrawerPanel { children: vec![content], border_left: self.right })
                as Box<dyn Widget<Msg>>
        });

        if self.permanent {
            // Accosté dans le flux : rangée `[panneau, corps]` (ou l'inverse à droite).
            let body_pane: Box<dyn Widget<Msg>> = Box::new(Flex::column().flex(1.0).child(body));
            self.children = match panel {
                Some(panel) if self.right => vec![body_pane, panel],
                Some(panel) => vec![panel, body_pane],
                None => vec![body_pane],
            };
        } else {
            // Modal : seul le corps est dans le flux ; le panneau part en overlay.
            self.modal_panel = panel;
            self.children = vec![Box::new(body)];
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Drawer<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            // Permanent : rangée (panneau + corps côte à côte). Modal : le corps
            // remplit seul (le panneau flotte en overlay).
            flex_direction: if self.permanent {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
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
        // Uniquement en mode modal : c'est la **progression** animée
        // (`anim_target`) qui décide de l'affichage et du glissement.
        let placement = if self.right { Placement::Right } else { Placement::Left };
        self.modal_panel.as_ref().map(|p| (p.as_ref(), placement))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn anim_target(&self) -> Option<f32> {
        // Pas d'animation en mode permanent (toujours affiché). Sinon, cible
        // d'ouverture `0↔1` interpolée par le runtime (glissement + fondu).
        if self.permanent {
            None
        } else {
            Some(if self.open { 1.0 } else { 0.0 })
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
    fn anim_target_reflects_open_state() {
        let closed = Drawer::new(false)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&closed), Some(0.0));
        assert!(Widget::<Msg>::overlay(&closed).is_some());

        let open = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&open), Some(1.0));
        assert_eq!(Widget::<Msg>::overlay_dismiss(&open), Some(Msg::Close));
    }

    #[test]
    fn right_drawer_uses_right_placement() {
        let d = Drawer::new(true)
            .right()
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        assert!(matches!(
            Widget::<Msg>::overlay(&d),
            Some((_, Placement::Right))
        ));
    }

    #[test]
    fn closed_drawer_draws_no_scrim() {
        let drawer = Drawer::new(false)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &drawer,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0),
        );
        assert!(!scrim, "un tiroir fermé ne peint pas de voile");
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
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0),
        );
        assert!(scrim, "le voile doit couvrir la fenêtre");
        let panel = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && rect.height >= 399.0)
        });
        assert!(panel, "le panneau doit se déployer sur toute la hauteur");
    }

    #[test]
    fn mid_animation_slides_panel_and_fades_scrim() {
        let drawer = Drawer::new(true)
            .on_dismiss(Msg::Close)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        let mut rt = Runtime::default();
        rt.values.insert(crate::WidgetId::ROOT, 0.5);
        let ui = build_ui(&drawer, Size::new(500.0, 400.0), &rt, &Theme::default());
        let panel_edge = ui.scene().primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 =>
            {
                Some(rect.x + rect.width)
            }
            _ => None,
        });
        let edge = panel_edge.expect("le panneau du tiroir doit être présent");
        assert!(
            (edge - DRAWER_WIDTH * 0.5).abs() < 2.0,
            "panneau à moitié rentré : bord droit ≈ largeur/2, obtenu {edge}"
        );
    }

    #[test]
    fn permanent_drawer_docks_panel_beside_body_without_scrim() {
        let drawer = Drawer::new(false) // `open` ignoré en permanent
            .permanent(true)
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        // Aucune animation, aucun overlay : le panneau est dans le flux.
        assert_eq!(Widget::<Msg>::anim_target(&drawer), None);
        assert!(Widget::<Msg>::overlay(&drawer).is_none());
        // Rangée [panneau, corps].
        assert_eq!(Widget::<Msg>::style(&drawer).flex_direction, FlexDirection::Row);
        assert_eq!(Widget::<Msg>::children(&drawer).len(), 2);

        let ui = build_ui(
            &drawer,
            Size::new(900.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Pas de voile plein écran (le panneau ne couvre que sa largeur).
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 900.0),
        );
        assert!(!scrim, "un tiroir permanent ne peint pas de voile");
        // Le panneau accosté est présent, plein-hauteur, à gauche (x ≈ 0).
        let docked = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && rect.x < 1.0 && rect.height >= 399.0)
        });
        assert!(docked, "le panneau doit être accosté à gauche, plein-hauteur");
    }

    #[test]
    fn permanent_right_docks_panel_on_the_right() {
        let drawer = Drawer::new(false)
            .permanent(true)
            .right()
            .panel(Text::new("menu"))
            .body(Container::<Msg>::new());
        // Ordre [corps, panneau] pour un accostage à droite.
        assert_eq!(Widget::<Msg>::children(&drawer).len(), 2);
        let ui = build_ui(
            &drawer,
            Size::new(900.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Panneau plein-hauteur dont le bord droit touche la fenêtre (x+w ≈ 900).
        let on_right = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - DRAWER_WIDTH).abs() < 1.0 && (rect.x + rect.width - 900.0).abs() < 1.0)
        });
        assert!(on_right, "le panneau doit être accosté à droite");
    }
}
