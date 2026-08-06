//! [`Button`] : un bouton themé avec libellé, variantes et états d'interaction.

use frus_core::{BorderRadius, Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 20.0;
const PAD_Y: f32 = 12.0;

/// Variante visuelle d'un bouton.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Variant {
    /// Accent principal du thème.
    #[default]
    Primary,
    /// Surface neutre bordée.
    Secondary,
    /// Action destructive.
    Danger,
}

/// Un bouton cliquable.
pub struct Button<Msg> {
    label: String,
    size: f32,
    variant: Variant,
    /// Rayons surchargés ; `None` = rayon du thème (uniforme).
    radius: Option<BorderRadius>,
    on_press: Option<Msg>,
    /// Actif ? Désactivé (`false`) : grisé, sans ombre, ni clic ni focus.
    enabled: bool,
}

impl<Msg> Button<Msg> {
    /// Crée un bouton avec un libellé.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 18.0,
            variant: Variant::Primary,
            radius: None,
            on_press: None,
            enabled: true,
        }
    }

    /// Active ou **désactive** le bouton : désactivé, il est grisé, sans ombre, et
    /// n'émet plus rien (ni clic ni focus clavier) — le rendu d'un contrôle indisponible
    /// (façon Material), p. ex. « Suivant » tant qu'une étape est invalide.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Surcharge les rayons des coins (uniforme via `f32`, par coin via
    /// [`BorderRadius`] — segments connectés, groupes de boutons…). Défaut :
    /// rayon du thème.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// Choisit la variante visuelle.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Taille de police.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Message émis au clic.
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// (base, texte, bordure) selon la variante et le thème.
    fn palette(&self, theme: &Theme) -> (Color, Color, Option<Color>) {
        match self.variant {
            Variant::Primary => (theme.primary, theme.on_primary, None),
            Variant::Secondary => (theme.surface, theme.on_surface, Some(theme.border)),
            Variant::Danger => (theme.error, theme.on_error, None),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Button<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.label, self.size);
        Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0).ceil()),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (base, on_color, border) = self.palette(theme);
        let radius = self.radius.unwrap_or_else(|| theme.radius.into());

        // Désactivé : aplat neutre, texte discret, **sans ombre** — un contrôle indisponible.
        if !self.enabled {
            let fill = theme.surface.lerp(theme.muted, 0.12);
            scene.draw_rect(bounds, fill.fade(o), radius, 1.0, theme.border.fade(o));
            scene.text(
                Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
                self.label.clone(),
                self.size,
                theme.muted.fade(o),
            );
            return;
        }

        // État survol/pression/focus via la state-layer bakée du thème.
        let color = theme.state_layer(base, on_color, &status);

        let blur = 10.0;
        let shadow_rect = Rect::new(
            bounds.x - blur,
            bounds.y + 3.0 - blur,
            bounds.width + 2.0 * blur,
            bounds.height + 2.0 * blur,
        );
        scene.shadow(
            shadow_rect,
            theme.scheme.shadow.with_alpha(0.35).fade(o),
            radius.inflate(blur),
            blur,
        );
        let (bw, bc) = match border {
            Some(c) => (1.0, c.fade(o)),
            None => (0.0, Color::TRANSPARENT),
        };
        scene.draw_rect(bounds, color.fade(o), radius, bw, bc);
        scene.text(
            Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
            self.label.clone(),
            self.size,
            on_color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        if self.enabled {
            self.on_press.clone()
        } else {
            None
        }
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let semantics =
            frus_core::Semantics::new(frus_core::Role::Button).label(self.label.clone());
        // Un bouton désactivé n'annonce pas d'action cliquable.
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pressed,
    }

    #[test]
    fn on_click_returns_message() {
        let button = Button::new("OK").on_press(Msg::Pressed);
        assert_eq!(Widget::on_click(&button), Some(Msg::Pressed));
    }

    #[test]
    fn disabled_button_is_inert_and_unfocusable() {
        let button = Button::new("Next").on_press(Msg::Pressed).enabled(false);
        assert_eq!(Widget::on_click(&button), None, "désactivé : aucun message");
        assert!(
            !Widget::<Msg>::focusable(&button),
            "désactivé : hors tabulation"
        );
        // Sémantique sans action cliquable.
        let semantics = Widget::<Msg>::semantics(&button).expect("sémantique présente");
        assert!(!semantics.clickable, "désactivé : non annoncé cliquable");
        // Réactivé : le clic repasse.
        let enabled = Button::new("Next").on_press(Msg::Pressed).enabled(true);
        assert_eq!(Widget::on_click(&enabled), Some(Msg::Pressed));
    }

    #[test]
    fn disabled_button_paints_no_shadow() {
        use frus_core::Primitive;
        let paint = |enabled: bool| {
            let button = Button::<Msg>::new("Next").enabled(enabled);
            let mut scene = Scene::new();
            Widget::<Msg>::paint(
                &button,
                Rect::new(0.0, 0.0, 90.0, 44.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Rect { blur, .. } if *blur > 0.0))
        };
        assert!(paint(true), "actif : une ombre est dessinée");
        assert!(!paint(false), "désactivé : aucune ombre");
    }
}
