//! [`Autocomplete`] : un champ de saisie avec une **liste de suggestions**
//! flottante. Contrôlé : l'application fournit la valeur **et** les suggestions
//! (déjà filtrées) ; la liste ne flotte que si elle est non vide. Largeur réglable
//! ([`width`](Autocomplete::width)) ; les suggestions prennent le focus clavier.
//!
//! Chaque suggestion **met en avant** la portion correspondant à la requête
//! (couleur `primary`), et la suggestion **active** ([`active`](Autocomplete::active),
//! parcourue au clavier) est surlignée — comme le menu d'un `Dropdown`.

use std::rc::Rc;

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::scroll::Scroll;
use crate::textinput::TextInput;
use crate::theme::Theme;
use crate::widget::Widget;

const DEFAULT_WIDTH: f32 = 260.0;
const ROW_H: f32 = 32.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 16.0;
/// Écart vertical entre suggestions.
const ROW_GAP: f32 = 2.0;

/// Portion (indices de **caractères**) du libellé qui correspond à la requête
/// (recherche insensible à la casse). `None` si la requête est vide ou absente.
fn match_range(label: &str, query: &str) -> Option<(usize, usize)> {
    if query.trim().is_empty() {
        return None;
    }
    let ll: Vec<char> = label.to_lowercase().chars().collect();
    let ql: Vec<char> = query.to_lowercase().chars().collect();
    if ql.is_empty() || ql.len() > ll.len() {
        return None;
    }
    (0..=ll.len() - ql.len())
        .find(|&i| ll[i..i + ql.len()] == ql[..])
        .map(|i| (i, i + ql.len()))
}

/// Une suggestion cliquable. La portion correspondant à `query` est mise en avant
/// (couleur `primary`) ; la suggestion **active** (parcourue au clavier) est surlignée.
struct Suggestion<Msg> {
    label: String,
    /// Requête courante, pour surligner la portion correspondante.
    query: String,
    width: f32,
    /// Suggestion **active** (celle qui serait choisie) : fond teinté.
    active: bool,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Suggestion<Msg> {
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
        // Suggestion active : fond teinté `primary` ; survol par-dessus.
        let base = if self.active {
            theme.surface.lerp(theme.primary, 0.14)
        } else {
            theme.surface
        };
        let bg = theme.state_layer(base, theme.on_surface, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));

        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        let chars: Vec<char> = self.label.chars().collect();
        let normal = theme.on_surface.fade(o);
        let hilite = theme.primary.fade(o);
        let mut x = bounds.x + PAD_X;
        // Segments [avant | correspondance | après] : la correspondance en `primary`.
        let segments: [(std::ops::Range<usize>, frus_core::Color); 3] =
            match match_range(&self.label, &self.query) {
                Some((i, j)) => [(0..i, normal), (i..j, hilite), (j..chars.len(), normal)],
                None => [(0..chars.len(), normal), (0..0, normal), (0..0, normal)],
            };
        for (range, color) in segments {
            if range.is_empty() {
                continue;
            }
            let text: String = chars[range].iter().collect();
            let width = frus_text::measure(&text, SIZE).width;
            scene.text(Point::new(x, ty), text, SIZE, color);
            x += width;
        }
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Un champ de saisie avec suggestions.
pub struct Autocomplete<Msg> {
    value: String,
    width: f32,
    /// Suggestion **active** (parcourue au clavier / surlignée), le cas échéant.
    active: Option<usize>,
    /// Nombre max de suggestions visibles : au-delà, la liste **défile**.
    max_visible: Option<usize>,
    on_input: Rc<dyn Fn(String) -> Msg>,
    on_pick: Rc<dyn Fn(String) -> Msg>,
    labels: Vec<String>,
    /// `[champ]` ou `[champ, liste]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Autocomplete<Msg> {
    /// Crée un champ : valeur courante, `on_input(texte)` à la frappe, et
    /// `on_pick(suggestion)` au choix d'une suggestion.
    pub fn new(
        value: impl Into<String>,
        on_input: impl Fn(String) -> Msg + 'static,
        on_pick: impl Fn(String) -> Msg + 'static,
    ) -> Self {
        let mut ac = Self {
            value: value.into(),
            width: DEFAULT_WIDTH,
            active: None,
            max_visible: None,
            on_input: Rc::new(on_input),
            on_pick: Rc::new(on_pick),
            labels: Vec::new(),
            children: Vec::new(),
        };
        ac.rebuild();
        ac
    }

    /// Largeur du champ et des suggestions, en pixels logiques (défaut 260).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Index de la suggestion **active** (surlignée ; choisie au clavier). L'app la
    /// fait avancer (flèches) et la valide (Entrée) — l'état reste chez elle.
    pub fn active(mut self, index: usize) -> Self {
        self.active = Some(index);
        self.rebuild();
        self
    }

    /// Limite le nombre de suggestions **visibles** : au-delà, la liste flottante
    /// **défile** (viewport borné à `n` lignes) au lieu de s'étirer sans fin.
    pub fn max_visible(mut self, rows: usize) -> Self {
        self.max_visible = Some(rows.max(1));
        self.rebuild();
        self
    }

    /// Ajoute une suggestion à la liste flottante.
    pub fn suggestion(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        // Champ : reconstruit à chaque réglage (largeur, valeur). Le rappel `on_input`
        // partagé (Rc) est capturé par le champ.
        let on_input = self.on_input.clone();
        let input = TextInput::new(self.value.clone())
            .width(self.width)
            .on_input(move |text| on_input(text));
        self.children = vec![Box::new(input)];

        if !self.labels.is_empty() {
            let mut list = Flex::column().gap(ROW_GAP);
            for (index, label) in self.labels.iter().enumerate() {
                list = list.child(Suggestion {
                    label: label.clone(),
                    query: self.value.clone(),
                    width: self.width,
                    active: self.active == Some(index),
                    message: (self.on_pick)(label.clone()),
                });
            }
            // Au-delà du seuil, la liste défile dans un viewport borné à `n` lignes.
            match self.max_visible {
                Some(n) if self.labels.len() > n => {
                    let viewport = n as f32 * ROW_H + (n as f32 - 1.0) * ROW_GAP;
                    self.children
                        .push(Box::new(Scroll::new().width(self.width).height(viewport).child(list)));
                }
                _ => self.children.push(Box::new(list)),
            }
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Autocomplete<Msg> {
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
        self.children
            .get(1)
            .map(|list| (list.as_ref(), Placement::Below))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Input(String),
        Pick(String),
    }

    #[test]
    fn no_suggestions_no_overlay() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick);
        assert!(Widget::<Msg>::overlay(&ac).is_none());
    }

    #[test]
    fn suggestions_float_and_pick() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot");
        assert!(Widget::<Msg>::overlay(&ac).is_some());
        // La liste contient les deux suggestions ; la 1ʳᵉ émet Pick("apple").
        let list = &Widget::<Msg>::children(&ac)[1];
        assert_eq!(list.children().len(), 2);
        assert_eq!(list.children()[0].on_click(), Some(Msg::Pick("apple".to_string())));
    }

    #[test]
    fn match_range_is_case_insensitive_substring() {
        assert_eq!(match_range("Apricot", "ap"), Some((0, 2)));
        assert_eq!(match_range("pineapple", "APPLE"), Some((4, 9)));
        assert_eq!(match_range("apple", ""), None);
        assert_eq!(match_range("apple", "xyz"), None);
    }

    #[test]
    fn matched_portion_is_drawn_as_its_own_segment() {
        // Requête "ap" sur "apricot" → segments "ap" (mis en avant) + "ricot".
        let ac = Autocomplete::new("ap", Msg::Input, Msg::Pick).suggestion("apricot");
        let (list, _) = Widget::<Msg>::overlay(&ac).unwrap();
        let ui = build_ui(list, Size::new(280.0, 80.0), &Runtime::default(), &Theme::default());
        let texts: Vec<String> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"ap".to_string()), "portion correspondante isolée : {texts:?}");
        assert!(texts.contains(&"ricot".to_string()), "reste isolé : {texts:?}");
    }

    #[test]
    fn active_suggestion_is_highlighted() {
        let ac = Autocomplete::new("ap", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot")
            .active(1);
        let (list, _) = Widget::<Msg>::overlay(&ac).unwrap();
        let ui = build_ui(list, Size::new(280.0, 120.0), &Runtime::default(), &Theme::default());
        let theme = Theme::default();
        let tint = theme.surface.lerp(theme.primary, 0.14);
        let has_tint = ui.scene().primitives().iter().any(|p| matches!(
            p,
            Primitive::Rect { color, .. } if color.fade(1.0) == tint.fade(1.0)
        ));
        assert!(has_tint, "la suggestion active est surlignée");
    }

    #[test]
    fn long_list_scrolls_when_capped() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .max_visible(2)
            .suggestion("a1")
            .suggestion("a2")
            .suggestion("a3")
            .suggestion("a4");
        let (overlay, _) = Widget::<Msg>::overlay(&ac).unwrap();
        // L'overlay est un Scroll borné à 2 lignes (viewport = 2*ROW_H + 1 écart).
        let expected = 2.0 * ROW_H + ROW_GAP;
        assert!(
            matches!(Widget::<Msg>::style(overlay).height, Dimension::Length(v) if (v - expected).abs() < 0.5),
            "viewport borné à 2 lignes",
        );
        // Il défile bien sur les 4 suggestions.
        assert_eq!(overlay.children()[0].children().len(), 4);
    }

    #[test]
    fn short_list_is_not_wrapped_in_scroll() {
        // Sous le seuil : liste nue (pas de viewport borné).
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .max_visible(5)
            .suggestion("a1")
            .suggestion("a2");
        let (overlay, _) = Widget::<Msg>::overlay(&ac).unwrap();
        // Liste directe : ses enfants sont les 2 suggestions (pas un Scroll d'un cran).
        assert_eq!(overlay.children().len(), 2);
        assert_eq!(overlay.children()[0].on_click(), Some(Msg::Pick("a1".to_string())));
    }

    #[test]
    fn field_and_suggestions_are_keyboard_reachable() {
        // La descente clavier passe par le système de focus : le champ puis les
        // suggestions entrent dans le cycle Tab (flèche bas depuis le champ mono-ligne
        // navigue le focus vers la 1ʳᵉ suggestion).
        let ac = Autocomplete::new("ap", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot");
        let ui = build_ui(&ac, Size::new(280.0, 200.0), &Runtime::default(), &Theme::default());
        let first = ui.focus_next(None, true);
        assert!(first.is_some(), "le champ est focusable");
        let second = ui.focus_next(first, true);
        assert!(second.is_some() && second != first, "une suggestion suit le champ dans le cycle");
    }
}
