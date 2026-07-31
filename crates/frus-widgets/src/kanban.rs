//! [`Kanban`] : un tableau **colonnes + cartes** avec **glisser-déposer** d'une colonne à l'autre.
//!
//! Comme le reste de frus, le widget est **contrôlé** : l'application tient les colonnes et leurs
//! cartes, et réagit à un unique message `on_move(from_col, from_pos, to_col, to_pos)` émis au dépôt
//! d'une carte. Le glisser-déposer réutilise le mécanisme de **réordonnancement** du framework
//! (`reorder_index` / `on_reorder`) : chaque emplacement porte un **index plat** `col * STRIDE + pos`,
//! et la carte saisie décode l'emplacement **cible** (une autre carte, ou la zone de dépôt en bas
//! d'une colonne) pour en déduire la destination.

use std::rc::Rc;

use frus_core::{Color, Insets, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

/// Pas d'encodage d'un emplacement `(colonne, position)` en index plat : borne le nombre de cartes
/// par colonne (largement suffisant pour un tableau). Voir [`kanban_slot`].
const STRIDE: usize = 1000;
/// Largeur d'une colonne.
const COL_W: f32 = 220.0;
/// Hauteur d'une carte.
const CARD_H: f32 = 44.0;

/// Index **plat** d'un emplacement `(col, pos)` : `col * STRIDE + pos`. C'est la valeur de
/// [`reorder_index`](Widget::reorder_index) d'une carte (source **et** cible). Réutilisable pour
/// tester le routage du glisser-déposer.
pub fn kanban_slot(col: usize, pos: usize) -> usize {
    col * STRIDE + pos
}

/// Décode un index plat en `(col, pos)` (inverse de [`kanban_slot`]).
fn decode(slot: usize) -> (usize, usize) {
    (slot / STRIDE, slot % STRIDE)
}

/// Une carte : source **et** cible de glisser-déposer. Peinte comme une tuile surélevée.
struct Card<Msg> {
    label: String,
    /// Emplacement propre (index plat) : sert de `reorder_index` (source saisie **et** cible de dépôt).
    slot: usize,
    from_col: usize,
    from_pos: usize,
    on_move: Option<Rc<dyn Fn(usize, usize, usize, usize) -> Msg>>,
}

impl<Msg: Clone> Widget<Msg> for Card<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto, // étirée à la largeur de la colonne (align Stretch)
            height: Dimension::Length(CARD_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Tuile surélevée ; teintée au survol (repère de préhension).
        let base = theme.surface.lerp(theme.on_surface, 0.05);
        let fill = theme.state_layer(base, theme.on_surface, &status);
        scene.draw_rect(bounds, fill.fade(o), theme.radius, 1.0, theme.border.fade(o));
        let ty = bounds.y + (bounds.height - frus_text::line_height(15.0)) * 0.5;
        scene.text(Point::new(bounds.x + 12.0, ty), self.label.clone(), 15.0, theme.on_surface.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn reorder_index(&self) -> Option<usize> {
        Some(self.slot)
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        // La cible `to` est l'index plat de l'emplacement survolé (autre carte ou zone de dépôt).
        let (to_col, to_pos) = decode(to);
        self.on_move.as_ref().map(|f| f(self.from_col, self.from_pos, to_col, to_pos))
    }
}

/// Zone de dépôt en bas d'une colonne : cible d'insertion **en fin** de colonne (et seule cible d'une
/// colonne vide). Non-source utile (son `on_reorder` ne déplace rien).
struct DropZone {
    slot: usize,
}

impl<Msg: Clone> Widget<Msg> for DropZone {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            height: Dimension::Length(CARD_H * 0.8),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Contour discret en pointillé simulé (bord estompé) : « déposer ici ».
        scene.draw_rect(bounds, Color::TRANSPARENT, theme.radius, 1.0, theme.border.fade(status.opacity * 0.5));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn reorder_index(&self) -> Option<usize> {
        Some(self.slot)
    }

    fn on_reorder(&self, _to: usize) -> Option<Msg> {
        None // la zone de dépôt n'est pas une source de déplacement
    }
}

/// Le **panneau** d'une colonne : titre + cartes + zone de dépôt, sur un fond thémé discret. C'est
/// un conteneur `Flex` vertical qui peint son fond (le clip du panneau contient ses cartes).
struct Column<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for Column<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(COL_W),
            flex_direction: FlexDirection::Column,
            gap: 8.0,
            padding: Insets::uniform(12.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Fond de panneau **thémé** (défaut surchargeable via le thème) : un voile discret.
        let bg = theme.surface.lerp(theme.on_surface, 0.04);
        scene.draw_rect(bounds, bg.fade(status.opacity), theme.radius, 0.0, Color::TRANSPARENT);
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Un tableau **Kanban** : des colonnes titrées de cartes, avec glisser-déposer inter-colonnes.
///
/// ```
/// use frus_widgets::Kanban;
/// let board: Kanban = Kanban::new(|fc, fp, tc, tp| { let _ = (fc, fp, tc, tp); })
///     .column("To do", ["Design", "Spec"])
///     .column("Doing", ["Build"])
///     .column("Done", ["Kickoff"]);
/// ```
pub struct Kanban<Msg = ()> {
    on_move: Option<Rc<dyn Fn(usize, usize, usize, usize) -> Msg>>,
    columns: Vec<(String, Vec<String>)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Kanban<Msg> {
    /// Crée un tableau ; `on_move(from_col, from_pos, to_col, to_pos)` est émis quand une carte est
    /// **déposée** sur un emplacement (autre carte, ou fin d'une colonne).
    pub fn new(on_move: impl Fn(usize, usize, usize, usize) -> Msg + 'static) -> Self {
        Self { on_move: Some(Rc::new(on_move)), columns: Vec::new(), children: Vec::new() }
    }

    /// Ajoute une **colonne** titrée avec ses cartes (texte), dans l'ordre.
    pub fn column(mut self, title: impl Into<String>, cards: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.columns.push((title.into(), cards.into_iter().map(Into::into).collect()));
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        self.children = self
            .columns
            .iter()
            .enumerate()
            .map(|(c, (title, cards))| self.build_column(c, title, cards))
            .collect();
    }

    /// Construit une colonne : titre + cartes (chacune source/cible de dépôt) + zone de dépôt finale.
    fn build_column(&self, col: usize, title: &str, cards: &[String]) -> Box<dyn Widget<Msg>> {
        let mut children: Vec<Box<dyn Widget<Msg>>> =
            vec![Box::new(Text::new(title.to_string()).size(16.0))];
        for (pos, label) in cards.iter().enumerate() {
            children.push(Box::new(Card {
                label: label.clone(),
                slot: kanban_slot(col, pos),
                from_col: col,
                from_pos: pos,
                on_move: self.on_move.clone(),
            }));
        }
        // Emplacement d'insertion en fin de colonne (et cible d'une colonne vide).
        children.push(Box::new(DropZone { slot: kanban_slot(col, cards.len()) }));
        Box::new(Column { children })
    }
}

impl<Msg: Clone> Widget<Msg> for Kanban<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 12.0,
            align: Align::Start,
            padding: Insets::ZERO,
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Move(usize, usize, usize, usize),
    }

    /// Trouve la première carte (widget avec `reorder_index`) d'un sous-arbre et renvoie
    /// `(reorder_index, on_reorder(cible))`.
    fn first_card(w: &dyn Widget<Msg>, target: usize) -> Option<(usize, Option<Msg>)> {
        if let Some(idx) = w.reorder_index() {
            return Some((idx, w.on_reorder(target)));
        }
        for c in w.children() {
            if let Some(found) = first_card(c.as_ref(), target) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn slot_encoding_roundtrips() {
        assert_eq!(decode(kanban_slot(0, 0)), (0, 0));
        assert_eq!(decode(kanban_slot(2, 5)), (2, 5));
        assert_eq!(decode(kanban_slot(1, 3)), (1, 3));
    }

    #[test]
    fn dropping_a_card_routes_a_cross_column_move() {
        let board = Kanban::new(Msg::Move)
            .column("To do", ["A", "B"])
            .column("Doing", ["C"]);
        // Première carte = colonne 0, position 0 ("A") : son index plat, et le déplacement produit
        // quand on la dépose sur l'emplacement (1, 0) de la colonne « Doing ».
        let col0 = &Widget::<Msg>::children(&board)[0];
        let (idx, moved) = first_card(col0.as_ref(), kanban_slot(1, 0)).expect("une carte");
        assert_eq!(idx, kanban_slot(0, 0), "index plat de la carte source");
        assert_eq!(moved, Some(Msg::Move(0, 0, 1, 0)), "dépôt en (1,0) : déplacement inter-colonnes");
    }

    #[test]
    fn board_lays_out_one_widget_per_column() {
        let board = Kanban::new(Msg::Move).column("A", ["x"]).column("B", Vec::<String>::new());
        assert_eq!(Widget::<Msg>::children(&board).len(), 2, "un widget par colonne");
    }
}
