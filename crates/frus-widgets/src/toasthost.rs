//! [`ToastHost`] : la **couche de notifications** — positionne et empile les [`crate::Toast`]
//! dans un coin de l'écran, avec une **transition d'entrée** optionnelle.
//!
//! À poser en dernière couche d'un [`crate::Stack`] au-dessus de l'interface. Le widget remplit
//! la surface disponible et aligne ses toasts dans le coin choisi ([`ToastPosition`]) ; plusieurs
//! toasts s'**empilent** en colonne. `fade_in` enveloppe chaque toast d'une opacité animée
//! (couche d'animation existante, [`crate::AnimatedOpacity`]) pour une apparition en fondu.
//!
//! Le **contenu** (quel(s) toast(s) afficher, leur file d'attente/auto-fermeture) reste piloté
//! par l'application via [`crate::SnackbarQueue`] : `ToastHost` ne fait que placer.

use frus_core::{Curve, Insets, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::animated::AnimatedOpacity;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Marge par défaut entre les toasts et les bords.
const HOST_PAD: f32 = 16.0;
/// Écart vertical entre toasts empilés.
const STACK_GAP: f32 = 8.0;

/// Coin d'ancrage des notifications.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToastPosition {
    TopStart,
    TopCenter,
    TopEnd,
    BottomStart,
    BottomCenter,
    BottomEnd,
}

impl ToastPosition {
    /// Alignement vertical (axe principal de la colonne) : haut vs bas.
    fn justify(self) -> Justify {
        match self {
            ToastPosition::TopStart | ToastPosition::TopCenter | ToastPosition::TopEnd => {
                Justify::Start
            }
            _ => Justify::End,
        }
    }

    /// Alignement horizontal (axe transverse) : gauche / centre / droite.
    fn align(self) -> Align {
        match self {
            ToastPosition::TopStart | ToastPosition::BottomStart => Align::Start,
            ToastPosition::TopCenter | ToastPosition::BottomCenter => Align::Center,
            ToastPosition::TopEnd | ToastPosition::BottomEnd => Align::End,
        }
    }
}

/// Couche de notifications ancrée dans un coin.
pub struct ToastHost<Msg> {
    position: ToastPosition,
    padding: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> ToastHost<Msg> {
    /// Une couche vide ancrée à `position`.
    pub fn new(position: ToastPosition) -> Self {
        Self {
            position,
            padding: HOST_PAD,
            children: Vec::new(),
        }
    }

    /// Marge entre les toasts et les bords (défaut 16 px).
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Ajoute un toast à empiler (appeler plusieurs fois pour en empiler plusieurs).
    pub fn toast(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(widget));
        self
    }

    /// Enveloppe **chaque** toast d'une opacité animée (`duration` secondes) : apparition en
    /// fondu à l'aide de la couche d'animation existante. À appeler après les `toast`.
    pub fn fade_in(self, duration: f32) -> Self {
        self.wrap_opacity(1.0, duration)
    }

    /// Symétrique de [`fade_in`](Self::fade_in) : anime l'opacité vers **0** — la transition de
    /// **sortie** (le toast s'efface avant son retrait, façon Material). L'application la joue
    /// quand la notification passe « en sortie » (voir [`crate::SnackbarQueue::is_leaving`]).
    pub fn fade_out(self, duration: f32) -> Self {
        self.wrap_opacity(0.0, duration)
    }

    /// Enveloppe chaque toast d'une opacité animée vers `target`.
    fn wrap_opacity(mut self, target: f32, duration: f32) -> Self {
        self.children = self
            .children
            .into_iter()
            .map(|child| {
                Box::new(AnimatedOpacity::new(
                    target,
                    duration,
                    Curve::ease_in_out(),
                    child,
                )) as Box<dyn Widget<Msg>>
            })
            .collect();
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ToastHost<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            justify: self.position.justify(),
            align: self.position.align(),
            gap: STACK_GAP,
            padding: Insets::new(self.padding, self.padding, self.padding, self.padding),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Text;

    #[test]
    fn empty_host_has_no_children() {
        let host = ToastHost::<()>::new(ToastPosition::TopEnd);
        assert!(Widget::<()>::children(&host).is_empty());
    }

    #[test]
    fn position_maps_to_justify_and_align() {
        let host = ToastHost::<()>::new(ToastPosition::BottomEnd).toast(Text::new("x"));
        let style = Widget::<()>::style(&host);
        assert!(matches!(style.justify, Justify::End), "bas → justify End");
        assert!(matches!(style.align, Align::End), "droite → align End");
        assert_eq!(Widget::<()>::children(&host).len(), 1);

        let top_center = Widget::<()>::style(&ToastHost::<()>::new(ToastPosition::TopCenter));
        assert!(matches!(top_center.justify, Justify::Start));
        assert!(matches!(top_center.align, Align::Center));
    }

    #[test]
    fn stacks_multiple_and_fade_in_preserves_count() {
        let host = ToastHost::<()>::new(ToastPosition::BottomCenter)
            .toast(Text::new("a"))
            .toast(Text::new("b"))
            .fade_in(0.2);
        assert_eq!(
            Widget::<()>::children(&host).len(),
            2,
            "deux toasts, enveloppés en fondu"
        );
    }

    #[test]
    fn fade_out_wraps_children() {
        let host = ToastHost::<()>::new(ToastPosition::BottomCenter)
            .toast(Text::new("bye"))
            .fade_out(0.3);
        assert_eq!(
            Widget::<()>::children(&host).len(),
            1,
            "toast enveloppé en fondu de sortie"
        );
    }
}
