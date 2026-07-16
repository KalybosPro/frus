//! [`BottomSheet`] : une **feuille modale** qui glisse depuis le bas de la
//! fenêtre — pour un lot d'actions contextuelles ou un formulaire court, sans
//! quitter l'écran courant.
//!
//! Le corps reste visible en fond ; quand la feuille est ouverte, un panneau
//! pleine-largeur monte depuis le bord bas par-dessus, avec un voile qui la
//! referme au clic. Le glissement est animé automatiquement (courbe en ressort,
//! comme le tiroir — jalons 46/48), sans câblage côté application.
//!
//! ```ignore
//! BottomSheet::new(app.sheet_open)
//!     .on_dismiss(Msg::CloseSheet)
//!     .sheet(actions_column)  // contenu de la feuille
//!     .body(main_screen)      // contenu de fond (toujours visible)
//! ```

use frus_core::{Color, Insets, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// Poignée (« grabber ») en haut de la feuille : largeur / hauteur en px logiques.
const GRABBER_WIDTH: f32 = 36.0;
const GRABBER_HEIGHT: f32 = 4.0;

/// Panneau interne de la feuille : pleine-largeur, hauteur naturelle, fond
/// thématisé, liseré + poignée en haut.
struct SheetPanel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for SheetPanel<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            // Hauteur naturelle : le contenu fixe la hauteur, la feuille s'y ajuste.
            height: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            // Marge haute pour laisser respirer la poignée au-dessus du contenu.
            padding: Insets::new(20.0, 0.0, 0.0, 0.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Surface opaque aux coins **hauts** arrondis (le bord bas est collé à la
        // fenêtre) + fin liseré haut, en retrait des arrondis.
        let radius = theme.radius + 6.0;
        scene.draw_rect(
            bounds,
            theme.surface.fade(o),
            frus_core::BorderRadius::top(radius),
            0.0,
            Color::TRANSPARENT,
        );
        scene.fill_rect(
            Rect::new(bounds.x + radius, bounds.y, (bounds.width - 2.0 * radius).max(0.0), 1.0),
            theme.border.fade(o),
        );
        // Poignée arrondie centrée près du haut.
        let gx = bounds.x + (bounds.width - GRABBER_WIDTH) * 0.5;
        let gy = bounds.y + 8.0;
        scene.draw_rect(
            Rect::new(gx, gy, GRABBER_WIDTH, GRABBER_HEIGHT),
            theme.muted.fade(0.5 * o),
            GRABBER_HEIGHT * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Feuille modale glissant depuis le bas : corps de fond + panneau escamotable.
pub struct BottomSheet<Msg> {
    open: bool,
    on_dismiss: Option<Msg>,
    /// Contenu de la feuille, fourni par l'appelant (avant enrobage `SheetPanel`).
    sheet_content: Option<Box<dyn Widget<Msg>>>,
    /// Panneau modal enrobé, prêt pour l'overlay.
    modal_panel: Option<Box<dyn Widget<Msg>>>,
    /// Enfants dans le flux : `[corps]` (le panneau flotte en overlay).
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> BottomSheet<Msg> {
    /// Crée une feuille : `open` indique si elle est déployée.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            on_dismiss: None,
            sheet_content: None,
            modal_panel: None,
            children: Vec::new(),
        }
    }

    /// Message émis au clic sur le voile (hors de la feuille) — pour refermer.
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Définit le **contenu de la feuille** (actions, formulaire court…).
    pub fn sheet(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.sheet_content = Some(Box::new(content));
        self
    }

    /// Définit le **corps de fond** (toujours visible) et finalise la feuille.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        self.modal_panel = self
            .sheet_content
            .take()
            .map(|content| Box::new(SheetPanel { children: vec![content] }) as Box<dyn Widget<Msg>>);
        self.children = vec![Box::new(body)];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for BottomSheet<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
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
        // C'est la **progression** animée (`anim_target`) qui décide de
        // l'affichage et du glissement vers le haut.
        self.modal_panel.as_ref().map(|p| (p.as_ref(), Placement::Bottom))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn anim_target(&self) -> Option<f32> {
        // Cible d'ouverture `0↔1` interpolée par le runtime (glissement + fondu).
        Some(if self.open { 1.0 } else { 0.0 })
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
        let closed = BottomSheet::new(false)
            .on_dismiss(Msg::Close)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&closed), Some(0.0));
        assert!(Widget::<Msg>::overlay(&closed).is_some());

        let open = BottomSheet::new(true)
            .on_dismiss(Msg::Close)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        assert_eq!(Widget::<Msg>::anim_target(&open), Some(1.0));
        assert_eq!(Widget::<Msg>::overlay_dismiss(&open), Some(Msg::Close));
    }

    #[test]
    fn sheet_uses_bottom_placement() {
        let s = BottomSheet::new(true)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        assert!(matches!(
            Widget::<Msg>::overlay(&s),
            Some((_, Placement::Bottom))
        ));
    }

    #[test]
    fn closed_sheet_draws_no_scrim() {
        let sheet = BottomSheet::new(false)
            .on_dismiss(Msg::Close)
            .sheet(Text::new("actions"))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &sheet,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0 && rect.height >= 400.0),
        );
        assert!(!scrim, "une feuille fermée ne peint pas de voile");
    }

    #[test]
    fn open_sheet_draws_scrim_and_full_width_panel() {
        // Un contenu de hauteur fixe pour que la feuille ait une hauteur mesurable.
        let sheet = BottomSheet::new(true)
            .on_dismiss(Msg::Close)
            .sheet(Container::<Msg>::new().height(120.0))
            .body(Container::<Msg>::new());
        let ui = build_ui(
            &sheet,
            Size::new(500.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let scrim = ui.scene().primitives().iter().any(
            |p| matches!(p, frus_core::Primitive::Rect { rect, .. } if rect.width >= 500.0 && rect.height >= 399.0),
        );
        assert!(scrim, "le voile doit couvrir la fenêtre");
        // Panneau pleine-largeur (h ≈ 140, pas le voile), collé au bord bas (y+h ≈ 400).
        let docked = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, .. }
                if (rect.width - 500.0).abs() < 1.0 && rect.height < 300.0
                    && (rect.y + rect.height - 400.0).abs() < 1.0)
        });
        assert!(docked, "le panneau doit être pleine-largeur, accosté au bas");
    }

    #[test]
    fn mid_animation_slides_sheet_up() {
        let sheet = BottomSheet::new(true)
            .on_dismiss(Msg::Close)
            .sheet(Container::<Msg>::new().height(120.0))
            .body(Container::<Msg>::new());
        let mut rt = Runtime::default();
        rt.values.insert(crate::WidgetId::ROOT, 0.5);
        let ui = build_ui(&sheet, Size::new(500.0, 400.0), &rt, &Theme::default());
        // Bord haut du panneau pleine-largeur (hauteur ≈ 140, pas le voile 500×400).
        let top = ui.scene().primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Rect { rect, .. }
                if (rect.width - 500.0).abs() < 1.0 && rect.height < 300.0 =>
            {
                Some(rect.y)
            }
            _ => None,
        });
        let top = top.expect("le panneau de la feuille doit être présent");
        // À t=0.5 linéaire, la feuille est remontée de `spring_ease(0.5)·hauteur`
        // depuis le bas : bord haut ≈ 400 − spring_ease(0.5)·140 (120 + 20 padding).
        let progress = crate::spring_ease(0.5);
        let sheet_h = 140.0;
        let expected = 400.0 - progress * sheet_h;
        assert!(
            (top - expected).abs() < 2.0,
            "bord haut attendu ≈ {expected}, obtenu {top}"
        );
    }
}
