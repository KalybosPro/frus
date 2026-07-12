//! [`AppBar`] : une **barre d'application adaptative** (façon Material).
//!
//! Le développeur déclare **un** titre, un `leading` optionnel et une liste
//! d'**actions** — sans jamais dire « ceci est pour mobile / desktop ». L'AppBar
//! décide seule, d'après la **largeur disponible**, combien d'actions tiennent en
//! ligne et **replie le reste dans un menu overflow `⋯`**. Large écran → tout en
//! ligne ; téléphone étroit → overflow. Un seul code, adaptation automatique.
//!
//! ```ignore
//! AppBar::new("My Tasks")
//!     .width(available_width)                 // la taille, pas une plateforme
//!     .leading(button("☰", Msg::ToggleMenu))
//!     .overflow(app.menu_open, Msg::ToggleMenu)
//!     .action("Pause", Msg::ToggleTimer)
//!     .action("Settings →", Msg::OpenSettings)
//!     .build()
//! ```

use crate::button::Variant;
use crate::container::Container;
use crate::dsl::{button, text};
use crate::flex::Flex;
use crate::menu::Menu;
use crate::widget::Widget;
use frus_layout::Align;

/// Taille de police du titre.
const TITLE_SIZE: f32 = 20.0;
/// Taille de police des actions (boutons en ligne et items d'overflow).
const ACTION_SIZE: f32 = 16.0;
/// Marge horizontale interne d'un bouton (doit suivre `button::PAD_X`).
const BTN_PAD_X: f32 = 20.0;
/// Espace entre éléments de la barre.
const GAP: f32 = 8.0;
/// Largeur réservée à l'emplacement `leading` (icône de tête, façon Material).
const LEADING_SLOT: f32 = 56.0;

/// Barre d'application adaptative. Constructeur fluide terminé par [`AppBar::build`].
pub struct AppBar<Msg> {
    title: String,
    width: f32,
    leading: Option<Box<dyn Widget<Msg>>>,
    overflow: Option<(bool, Msg)>,
    actions: Vec<(String, Msg)>,
}

impl<Msg: Clone + 'static> AppBar<Msg> {
    /// Crée une barre avec un titre. Sans [`AppBar::width`], rien ne se replie.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: f32::MAX,
            leading: None,
            overflow: None,
            actions: Vec::new(),
        }
    }

    /// **Largeur disponible** pour la barre (px logiques) : ce qui pilote le
    /// repli. C'est une *taille*, pas un indicateur de plateforme.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Élément de tête (bouton menu, retour…), optionnel.
    pub fn leading(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.leading = Some(Box::new(widget));
        self
    }

    /// Active le menu overflow : son état d'ouverture (contrôlé par l'app) et le
    /// message de bascule (émis par le bouton `⋯` et au clic extérieur).
    pub fn overflow(mut self, open: bool, toggle: Msg) -> Self {
        self.overflow = Some((open, toggle));
        self
    }

    /// Ajoute une action (libellé + message). Affichée en ligne si elle tient,
    /// sinon repliée dans le menu overflow.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.actions.push((label.into(), message));
        self
    }

    /// Largeur qu'occuperait un bouton d'action pour ce libellé.
    fn action_width(label: &str) -> f32 {
        frus_text::measure(label, ACTION_SIZE).width + BTN_PAD_X * 2.0
    }

    /// Assemble la barre en un widget prêt à afficher (rangée `leading · titre ·
    /// ressort · actions en ligne · overflow`).
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let AppBar { title, width, leading, overflow, actions } = self;

        // Budget horizontal restant pour les actions en ligne, après le leading,
        // le titre et les marges. Conservateur : en cas de doute, on replie.
        let leading_w = if leading.is_some() { LEADING_SLOT } else { 0.0 };
        let title_w = frus_text::measure(&title, TITLE_SIZE).width;
        let budget = width - leading_w - title_w - GAP * 3.0;
        let overflow_btn_w = Self::action_width("⋯") + GAP;

        // Combien d'actions tiennent en ligne ? Si toutes tiennent, pas d'overflow ;
        // sinon on réserve le bouton `⋯` et on en garde autant que possible.
        let widths: Vec<f32> = actions.iter().map(|(l, _)| Self::action_width(l) + GAP).collect();
        let total: f32 = widths.iter().sum();
        let inline_count = if total <= budget {
            actions.len()
        } else {
            let mut used = overflow_btn_w;
            let mut n = 0;
            for w in &widths {
                if used + w <= budget {
                    used += w;
                    n += 1;
                } else {
                    break;
                }
            }
            n
        };

        let mut row = Flex::row().align(Align::Center).gap(GAP);
        if let Some(leading) = leading {
            row = row.child(leading);
        }
        row = row.child(text(title).size(TITLE_SIZE));
        // Ressort : pousse les actions vers la droite.
        row = row.child(Container::new().flex(1.0));
        for (label, message) in actions.iter().take(inline_count) {
            row = row.child(
                button(label.clone(), message.clone())
                    .variant(Variant::Secondary)
                    .size(ACTION_SIZE),
            );
        }
        if inline_count < actions.len() {
            match overflow {
                // Menu overflow contrôlé : le `⋯` ouvre, les items émettent les actions.
                Some((open, toggle)) => {
                    let mut menu = Menu::new(
                        button("⋯", toggle.clone()).variant(Variant::Secondary).size(ACTION_SIZE),
                        open,
                        toggle,
                    );
                    for (label, message) in actions.iter().skip(inline_count) {
                        menu = menu.item(label.clone(), message.clone());
                    }
                    row = row.child(menu);
                }
                // Pas d'overflow configuré : on affiche tout en ligne (peut déborder).
                None => {
                    for (label, message) in actions.iter().skip(inline_count) {
                        row = row.child(
                            button(label.clone(), message.clone())
                                .variant(Variant::Secondary)
                                .size(ACTION_SIZE),
                        );
                    }
                }
            }
        }
        Box::new(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Menu,
        A,
        B,
        C,
    }

    /// Compte les boutons (rectangles avec ombre) hors items de menu flottant.
    fn inline_buttons(width: f32, open: bool) -> usize {
        let bar = AppBar::new("Title")
            .width(width)
            .overflow(open, Msg::Menu)
            .action("Action One", Msg::A)
            .action("Action Two", Msg::B)
            .action("Action Three", Msg::C)
            .build();
        let ui = build_ui(bar.as_ref(), Size::new(width, 80.0), &Runtime::default(), &Theme::default());
        // Chaque bouton dessine une ombre (un `Rect` flouté, `blur > 0`) : on les compte.
        ui.scene()
            .primitives()
            .iter()
            .filter(|p| matches!(p, frus_core::Primitive::Rect { blur, .. } if *blur > 0.0))
            .count()
    }

    #[test]
    fn wide_bar_shows_all_actions_inline() {
        // Assez large : les 3 actions tiennent, pas de bouton overflow.
        assert_eq!(inline_buttons(1200.0, false), 3);
    }

    #[test]
    fn narrow_bar_collapses_into_overflow() {
        // Étroit : au plus une ou deux actions en ligne + le bouton `⋯`.
        let n = inline_buttons(300.0, false);
        assert!(n < 3, "attendu un repli en overflow, obtenu {n} boutons en ligne");
        assert!(n >= 1, "le bouton overflow doit être présent");
    }
}
