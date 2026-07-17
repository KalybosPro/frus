//! [`AppBar`] : une **barre d'application adaptative** (façon Material).
//!
//! Le développeur déclare **un** titre, un `leading` optionnel et une liste
//! d'**actions** — sans jamais dire « ceci est pour mobile / desktop ». L'AppBar
//! décide seule, d'après la **largeur disponible**, combien d'actions tiennent en
//! ligne et **replie le reste dans un menu overflow `⋯`**. Large écran → tout en
//! ligne ; téléphone étroit → overflow. Un seul code, adaptation automatique.
//!
//! **Tout est personnalisable** (défauts thémés, jamais imposés) : le titre peut
//! être un widget arbitraire (`title_widget`) ou un texte stylé (`title_style`),
//! une action peut être un widget arbitraire (`action_widget`, toujours en ligne),
//! l'espacement, la taille des actions, le fond et la hauteur se surchargent.
//!
//! ```ignore
//! AppBar::new("My Tasks")
//!     .width(available_width)                 // la taille, pas une plateforme
//!     .title_style(TextStyle::new(22.0))      // ou .title_widget(logo_row)
//!     .leading(button("☰", Msg::ToggleMenu))
//!     .overflow(app.menu_open, Msg::ToggleMenu)
//!     .action("Pause", Msg::ToggleTimer)
//!     .action_widget(Badge::new("3"))         // widget libre, jamais replié
//!     .action("Settings →", Msg::OpenSettings)
//!     .build()
//! ```

use frus_core::{Color, FontWeight, TextStyle};
use frus_layout::{Align, Dimension};

use crate::button::Variant;
use crate::container::Container;
use crate::dsl::button;
use crate::flex::Flex;
use crate::menu::Menu;
use crate::text::Text;
use crate::widget::Widget;

/// Taille de police du titre (défaut, surchargée par [`AppBar::title_style`]).
const TITLE_SIZE: f32 = 20.0;
/// Taille de police des actions (défaut, surchargée par [`AppBar::action_size`]).
const ACTION_SIZE: f32 = 16.0;
/// Marge horizontale interne d'un bouton (doit suivre `button::PAD_X`).
const BTN_PAD_X: f32 = 20.0;
/// Espace entre éléments de la barre (défaut, surchargé par [`AppBar::gap`]).
const GAP: f32 = 8.0;
/// Largeur réservée à l'emplacement `leading` (icône de tête, façon Material).
const LEADING_SLOT: f32 = 56.0;
/// Marge horizontale de la barre : le contenu ne touche pas les bords (façon
/// Material). Comptée dans le budget de repli.
const H_PAD: f32 = 8.0;

/// Le titre : un texte stylé, ou n'importe quel widget (façon Flutter).
enum Title<Msg> {
    Text(String),
    Widget(Box<dyn Widget<Msg>>),
}

/// Une action : libellée (repliable dans l'overflow) ou widget libre (toujours
/// en ligne — un widget arbitraire ne peut pas devenir une ligne de menu texte).
enum Action<Msg> {
    Labeled { label: String, message: Msg },
    Custom(Box<dyn Widget<Msg>>),
}

/// Barre d'application adaptative. Constructeur fluide terminé par [`AppBar::build`].
pub struct AppBar<Msg> {
    title: Title<Msg>,
    title_style: TextStyle,
    width: f32,
    leading: Option<Box<dyn Widget<Msg>>>,
    overflow: Option<(bool, Msg)>,
    actions: Vec<Action<Msg>>,
    action_size: f32,
    gap: f32,
    background: Option<Color>,
    height: Option<f32>,
}

impl<Msg: Clone + 'static> AppBar<Msg> {
    /// Crée une barre avec un titre texte. Sans [`AppBar::width`], rien ne se replie.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Title::Text(title.into()),
            title_style: TextStyle::new(TITLE_SIZE).weight(FontWeight::Medium),
            width: f32::MAX,
            leading: None,
            overflow: None,
            actions: Vec::new(),
            action_size: ACTION_SIZE,
            gap: GAP,
            background: None,
            height: None,
        }
    }

    /// **Largeur disponible** pour la barre (px logiques) : ce qui pilote le
    /// repli. C'est une *taille*, pas un indicateur de plateforme.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Style du titre texte (taille/graisse/italique/couleur). Défaut : 20 px,
    /// graisse medium, couleur du thème.
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = style;
        self
    }

    /// Remplace le titre par un **widget arbitraire** (logo, rangée composée…),
    /// comme le `title: Widget` de Flutter.
    pub fn title_widget(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.title = Title::Widget(Box::new(widget));
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

    /// Ajoute une action libellée (bouton). Affichée en ligne si elle tient,
    /// sinon repliée dans le menu overflow.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.actions.push(Action::Labeled {
            label: label.into(),
            message,
        });
        self
    }

    /// Ajoute une action **widget libre** (badge, avatar, champ…). Toujours en
    /// ligne — un widget arbitraire ne peut pas se replier en ligne de menu.
    pub fn action_widget(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.actions.push(Action::Custom(Box::new(widget)));
        self
    }

    /// Taille de police des actions libellées (défaut : 16 px).
    pub fn action_size(mut self, size: f32) -> Self {
        self.action_size = size;
        self
    }

    /// Espace entre éléments de la barre (défaut : 8 px).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Couleur de fond de la barre (défaut : transparente, le parent décide).
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Hauteur imposée de la barre (défaut : hauteur naturelle du contenu).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Largeur qu'occuperait un bouton d'action pour ce libellé.
    fn action_width(label: &str, size: f32) -> f32 {
        frus_text::measure(label, size).width + BTN_PAD_X * 2.0
    }

    /// Largeur déclarée d'un widget (0 si elle dépend de la mise en page).
    fn widget_width(widget: &dyn Widget<Msg>) -> f32 {
        match widget.style().width {
            Dimension::Length(v) => v,
            _ => 0.0,
        }
    }

    /// Assemble la barre en un widget prêt à afficher (rangée `leading · titre ·
    /// ressort · actions en ligne · overflow`).
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let AppBar {
            title,
            title_style,
            width,
            leading,
            overflow,
            actions,
            action_size,
            gap,
            background,
            height,
        } = self;

        // Budget horizontal restant pour les actions en ligne, après le leading,
        // le titre et les marges. Conservateur : en cas de doute, on replie.
        let leading_w = if leading.is_some() { LEADING_SLOT } else { 0.0 };
        let title_w = match &title {
            Title::Text(content) => {
                frus_text::measure_styled(
                    content,
                    title_style.size,
                    title_style.weight,
                    title_style.italic,
                )
                .width
            }
            Title::Widget(widget) => Self::widget_width(widget.as_ref()),
        };
        // La marge horizontale ampute le budget des deux côtés (le contenu ne
        // touche pas les bords). `width` non fixé (f32::MAX) reste ~infini.
        let budget = (width - H_PAD * 2.0).min(width) - leading_w - title_w - gap * 3.0;
        let overflow_btn_w = Self::action_width("⋯", action_size) + gap;

        // Largeur de chaque action ; les widgets libres sont **toujours** en ligne.
        let widths: Vec<f32> = actions
            .iter()
            .map(|action| match action {
                Action::Labeled { label, .. } => Self::action_width(label, action_size) + gap,
                Action::Custom(widget) => Self::widget_width(widget.as_ref()) + gap,
            })
            .collect();
        let total: f32 = widths.iter().sum();
        let custom_total: f32 = actions
            .iter()
            .zip(&widths)
            .filter(|(action, _)| matches!(action, Action::Custom(_)))
            .map(|(_, w)| *w)
            .sum();

        // Combien d'actions **libellées** tiennent en ligne ? Si tout tient, pas
        // d'overflow ; sinon on réserve le bouton `⋯`, les widgets libres, et on
        // garde autant de libellées que possible (préfixe, dans l'ordre).
        let kept_labeled = if total <= budget {
            usize::MAX
        } else {
            let mut used = overflow_btn_w + custom_total;
            let mut kept = 0;
            for (action, w) in actions.iter().zip(&widths) {
                if matches!(action, Action::Custom(_)) {
                    continue;
                }
                if used + w <= budget {
                    used += w;
                    kept += 1;
                } else {
                    break;
                }
            }
            kept
        };

        let mut row = Flex::row().align(Align::Center).gap(gap);
        if let Some(leading) = leading {
            row = row.child(leading);
        }
        match title {
            Title::Text(content) => row = row.child(Text::styled(content, title_style)),
            Title::Widget(widget) => row = row.child(widget),
        }
        // Ressort : pousse les actions vers la droite.
        row = row.child(Container::new().flex(1.0));

        let mut labeled_seen = 0;
        let mut folded: Vec<(String, Msg)> = Vec::new();
        for action in actions {
            match action {
                Action::Custom(widget) => row = row.child(widget),
                Action::Labeled { label, message } => {
                    if labeled_seen < kept_labeled {
                        row = row.child(
                            button(label, message).variant(Variant::Secondary).size(action_size),
                        );
                    } else {
                        folded.push((label, message));
                    }
                    labeled_seen += 1;
                }
            }
        }

        if !folded.is_empty() {
            match overflow {
                // Menu overflow contrôlé : le `⋯` ouvre, les items émettent les actions.
                Some((open, toggle)) => {
                    let mut menu = Menu::new(
                        button("⋯", toggle.clone()).variant(Variant::Secondary).size(action_size),
                        open,
                        toggle,
                    );
                    for (label, message) in folded {
                        menu = menu.item(label, message);
                    }
                    row = row.child(menu);
                }
                // Pas d'overflow configuré : on affiche tout en ligne (peut déborder).
                None => {
                    for (label, message) in folded {
                        row = row.child(
                            button(label, message).variant(Variant::Secondary).size(action_size),
                        );
                    }
                }
            }
        }

        // Marge horizontale (+ chrome optionnel : fond, hauteur). La rangée est
        // toujours encadrée pour que le contenu ne touche pas les bords.
        let mut chrome = Container::new().padding_each(0.0, H_PAD, 0.0, H_PAD);
        if let Some(color) = background {
            chrome = chrome.color(color);
        }
        if let Some(h) = height {
            chrome = chrome.height(h);
        }
        Box::new(chrome.child(row))
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

    /// La barre garde une marge horizontale : le contenu (leading à gauche,
    /// dernière action à droite) ne touche pas les bords du viewport.
    #[test]
    fn content_keeps_a_horizontal_margin() {
        const W: f32 = 400.0;
        let bar = AppBar::new("Title")
            .width(W)
            .leading(button("M", Msg::Menu).size(16.0))
            .overflow(false, Msg::Menu)
            .action("One", Msg::A)
            .build();
        let ui = build_ui(bar.as_ref(), Size::new(W, 80.0), &Runtime::default(), &Theme::default());
        // Bornes horizontales des **textes** (titre + libellés) : sans ombre,
        // ils reflètent la position réelle du contenu (le flou des ombres, lui,
        // déborde légitimement).
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        for p in ui.scene().primitives() {
            if let frus_core::Primitive::Text { position, size, text, .. } = p {
                min_x = min_x.min(position.x);
                // Largeur approchée du texte (borne haute suffisante ici).
                max_x = max_x.max(position.x + text.chars().count() as f32 * size * 0.7);
            }
        }
        assert!(min_x >= H_PAD - 0.5, "contenu collé au bord gauche ({min_x})");
        assert!(max_x <= W - H_PAD + 0.5, "contenu débordant à droite ({max_x} > {})", W - H_PAD);
    }

    #[test]
    fn title_style_is_customizable() {
        // Style de titre surchargé : gras 24, au lieu du défaut medium 20.
        let bar = AppBar::<Msg>::new("Title")
            .title_style(TextStyle::new(24.0).weight(FontWeight::Bold))
            .build();
        let ui = build_ui(bar.as_ref(), Size::new(800.0, 80.0), &Runtime::default(), &Theme::default());
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                frus_core::Primitive::Text { text, size, weight, .. }
                    if text == "Title" && *size == 24.0 && *weight == FontWeight::Bold
            )
        });
        assert!(styled, "le titre doit porter le style surchargé");
    }

    #[test]
    fn title_can_be_an_arbitrary_widget() {
        let bar = AppBar::<Msg>::new("ignored")
            .title_widget(Text::new("Logo").size(18.0))
            .build();
        let ui = build_ui(bar.as_ref(), Size::new(800.0, 80.0), &Runtime::default(), &Theme::default());
        let texts: Vec<_> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Logo".to_string()), "le widget-titre est rendu");
        assert!(!texts.contains(&"ignored".to_string()), "le titre texte est remplacé");
    }

    #[test]
    fn custom_widget_action_never_folds() {
        // Barre très étroite : les actions libellées se replient, mais le widget
        // libre (non représentable en ligne de menu) reste en ligne.
        let bar = AppBar::new("Title")
            .width(260.0)
            .overflow(false, Msg::Menu)
            .action("A long labelled action", Msg::A)
            .action_widget(Text::new("★badge★").size(14.0))
            .action("Another long action", Msg::B)
            .build();
        let ui = build_ui(bar.as_ref(), Size::new(260.0, 80.0), &Runtime::default(), &Theme::default());
        let has_badge = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Text { text, .. } if text == "★badge★")
        });
        assert!(has_badge, "l'action-widget reste en ligne même à l'étroit");
    }
}
