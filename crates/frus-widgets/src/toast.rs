//! [`Toast`] : une notification transitoire (carte stylée). Le *système*
//! (empilement, minuterie d'auto-fermeture) est du ressort de l'application
//! (typiquement via un `Command` minuté).

use std::collections::VecDeque;

use frus_core::{Color, Insets, Point, Rect, Role, Scene, Semantics};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 12.0;
const SIZE: f32 = 16.0;
const ACCENT: f32 = 4.0;
/// Bouton d'action (Material « UNDO ») : police, marge et hauteur.
const ACTION_SIZE: f32 = 14.0;
const ACTION_PAD_X: f32 = 12.0;
const ACTION_GAP: f32 = 8.0;
const ACTION_H: f32 = 32.0;

/// Nature d'une notification (couleur d'accent).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

/// Une notification transitoire, avec une **action** optionnelle (façon Snackbar Material :
/// « UNDO »). L'action est un bouton texte à droite qui émet un message au clic.
pub struct Toast<Msg> {
    text: String,
    kind: ToastKind,
    /// Largeur additionnelle réservée à l'action (0 si aucune).
    action_w: f32,
    /// Vide, ou `[bouton d'action]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Toast<Msg> {
    /// Crée une notification d'information.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: ToastKind::Info,
            action_w: 0.0,
            children: Vec::new(),
        }
    }

    /// Variante succès.
    pub fn success(mut self) -> Self {
        self.kind = ToastKind::Success;
        self
    }

    /// Variante erreur.
    pub fn error(mut self) -> Self {
        self.kind = ToastKind::Error;
        self
    }

    /// Ajoute un **bouton d'action** (libellé en capitales, façon Material) émettant `message`
    /// au clic — typiquement « UNDO » pour annuler l'action qui a déclenché la notification.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        let label = label.into().to_uppercase();
        let width = (frus_text::measure(&label, ACTION_SIZE).width + ACTION_PAD_X * 2.0).ceil();
        self.action_w = width + ACTION_GAP;
        self.children = vec![Box::new(ActionButton {
            label,
            width,
            message,
        })];
        self
    }
}

impl<Msg> Toast<Msg> {
    fn accent(&self, theme: &Theme) -> Color {
        match self.kind {
            ToastKind::Info => theme.primary,
            ToastKind::Success => Color::rgb8(70, 190, 120),
            ToastKind::Error => Color::rgb8(210, 96, 96),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Toast<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.text, SIZE);
        let mut style = Style {
            width: Dimension::Length(
                (measured.width + PAD_X * 2.0 + ACCENT + self.action_w).ceil(),
            ),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).max(ACTION_H).ceil()),
            ..Default::default()
        };
        // Avec une action : la placer à droite, centrée verticalement.
        if !self.children.is_empty() {
            style.justify = Justify::End;
            style.align = Align::Center;
            style.padding = Insets::new(0.0, PAD_X, 0.0, 0.0);
        }
        style
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Ombre + carte.
        scene.shadow(
            Rect::new(
                bounds.x - 8.0,
                bounds.y - 4.0,
                bounds.width + 16.0,
                bounds.height + 16.0,
            ),
            theme.scheme.shadow.with_alpha(0.3).fade(o),
            theme.radius + 8.0,
            8.0,
        );
        scene.draw_rect(
            bounds,
            theme.surface.fade(o),
            theme.radius,
            1.0,
            theme.border.fade(o),
        );
        // Barre d'accent à gauche.
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y, ACCENT, bounds.height),
            self.accent(theme).fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        scene.text(
            Point::new(bounds.x + ACCENT + PAD_X, bounds.y + PAD_Y),
            self.text.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Bouton d'action d'une notification (texte en capitales, couleur d'accent), cliquable.
struct ActionButton<Msg> {
    label: String,
    width: f32,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for ActionButton<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(ACTION_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Fond de survol/focus (state-layer bakée : invisible au repos, teintée à l'interaction).
        let bg = theme.state_layer(theme.surface, theme.primary, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        let w = frus_text::measure(&self.label, ACTION_SIZE).width;
        scene.text(
            Point::new(
                bounds.x + (bounds.width - w) * 0.5,
                bounds.y + (bounds.height - frus_text::line_height(ACTION_SIZE)) * 0.5,
            ),
            self.label.clone(),
            ACTION_SIZE,
            theme.primary.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }

    fn semantics(&self) -> Option<Semantics> {
        Some(
            Semantics::new(Role::Button)
                .label(self.label.clone())
                .clickable(),
        )
    }
}

/// **File d'attente de notifications** — pure, côté application (esprit [`crate::form::Form`]).
///
/// Une seule notification est visible à la fois ; les suivantes patientent. L'application
/// appelle [`tick`](Self::tick) à chaque frame (avec le temps écoulé) pour faire **expirer**
/// la notification courante et présenter la suivante — l'auto-fermeture façon Material sans
/// minuterie côté widget. [`dismiss`](Self::dismiss) ferme la courante immédiatement (clic sur
/// l'action ou la croix). Générique sur la charge `T` (au minimum le texte ; souvent aussi le
/// type et le message d'action).
pub struct SnackbarQueue<T> {
    /// `(charge, secondes restantes, en cours de sortie)` ; l'avant est la notification affichée.
    /// Le drapeau « en sortie » permet à l'hôte de jouer une **transition de sortie** (fondu)
    /// avant le retrait (voir [`start_leaving`](Self::start_leaving) / [`is_leaving`](Self::is_leaving)).
    items: VecDeque<(T, f32, bool)>,
}

impl<T> Default for SnackbarQueue<T> {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }
}

impl<T> SnackbarQueue<T> {
    /// Une file vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute une notification qui restera visible `seconds` secondes une fois **en tête**.
    pub fn push(&mut self, item: T, seconds: f32) {
        self.items.push_back((item, seconds.max(0.0), false));
    }

    /// La notification actuellement visible (l'avant de la file), s'il y en a une.
    pub fn current(&self) -> Option<&T> {
        self.items.front().map(|(item, _, _)| item)
    }

    /// Fait s'écouler `dt` secondes sur la notification en tête ; si son temps est épuisé, elle
    /// est retirée (la suivante démarre son propre décompte). Rend `true` si la notification
    /// visible **a changé** (expiration) — utile pour redemander un rendu.
    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(front) = self.items.front_mut() else {
            return false;
        };
        front.1 -= dt;
        if front.1 <= 0.0 {
            self.items.pop_front();
            true
        } else {
            false
        }
    }

    /// Marque la notification courante **en sortie** : l'hôte peut alors jouer sa transition de
    /// sortie (fondu) avant que l'application ne la retire (via [`dismiss`](Self::dismiss)).
    pub fn start_leaving(&mut self) {
        if let Some(front) = self.items.front_mut() {
            front.2 = true;
        }
    }

    /// La notification courante est-elle **en sortie** ? (fondu de disparition en cours.)
    pub fn is_leaving(&self) -> bool {
        self.items.front().is_some_and(|(_, _, leaving)| *leaving)
    }

    /// Ferme la notification courante immédiatement (action/croix/fin de sortie) ; rend sa charge.
    pub fn dismiss(&mut self) -> Option<T> {
        self.items.pop_front().map(|(item, _, _)| item)
    }

    /// `true` si aucune notification n'est en attente.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Nombre de notifications en file (visible + en attente).
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Undo,
    }

    #[test]
    fn paints_card_accent_and_text() {
        let toast = Toast::<()>::new("Enregistré").success();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &toast,
            Rect::new(0.0, 0.0, 160.0, 44.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        // Accent succès présent + texte.
        let green = Color::rgb8(70, 190, 120);
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == green)));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Enregistré")));
    }

    #[test]
    fn action_is_clickable_and_uppercased() {
        // Sans action : aucun enfant.
        let plain = Toast::<Msg>::new("Item deleted");
        assert!(Widget::<Msg>::children(&plain).is_empty());
        // Avec action : un bouton en capitales qui émet le message.
        let toast = Toast::new("Item deleted").action("Undo", Msg::Undo);
        let kids = Widget::<Msg>::children(&toast);
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].on_click(), Some(Msg::Undo));
        assert!(kids[0].focusable());
    }

    #[test]
    fn queue_shows_one_at_a_time_and_expires() {
        let mut q: SnackbarQueue<&str> = SnackbarQueue::new();
        assert!(q.is_empty());
        q.push("first", 3.0);
        q.push("second", 3.0);
        assert_eq!(q.len(), 2);
        assert_eq!(q.current(), Some(&"first"), "l'avant est visible");
        // Le décompte ne touche que la tête.
        assert!(!q.tick(1.0));
        assert_eq!(q.current(), Some(&"first"));
        // Expiration → la suivante prend le relais.
        assert!(q.tick(2.5), "changement à l'expiration");
        assert_eq!(q.current(), Some(&"second"));
        // Fermeture manuelle (action/croix).
        assert_eq!(q.dismiss(), Some("second"));
        assert!(q.is_empty());
        assert!(!q.tick(1.0), "file vide : rien ne change");
    }

    #[test]
    fn leaving_phase_precedes_dismissal() {
        let mut q: SnackbarQueue<&str> = SnackbarQueue::new();
        assert!(!q.is_leaving(), "file vide : pas de sortie");
        q.push("hello", 3.0);
        assert!(!q.is_leaving(), "affichée : pas encore en sortie");
        // On déclenche la sortie (fondu) sans retirer tout de suite.
        q.start_leaving();
        assert!(q.is_leaving(), "en sortie");
        assert_eq!(
            q.current(),
            Some(&"hello"),
            "toujours visible pendant la sortie"
        );
        // Puis retrait effectif.
        assert_eq!(q.dismiss(), Some("hello"));
        assert!(!q.is_leaving());
        assert!(q.is_empty());
    }
}
