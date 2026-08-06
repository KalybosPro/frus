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

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::interaction::Status;
use crate::scroll::{Axis, Scroll};
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::{ReorderAxis, Widget};

/// Pas d'encodage d'un emplacement `(colonne, position)` en index plat : borne le nombre de cartes
/// par colonne (largement suffisant pour un tableau). Voir [`kanban_slot`].
const STRIDE: usize = 1000;
/// Largeur d'une colonne.
const COL_W: f32 = 220.0;
/// Marge intérieure (uniforme) du panneau d'une colonne.
const COL_PAD: f32 = 12.0;
/// Hauteur d'une carte.
const CARD_H: f32 = 44.0;

/// Index **plat** d'un emplacement `(col, pos)` : `col * STRIDE + pos`. C'est la valeur de
/// [`reorder_index`](Widget::reorder_index) d'une carte (source **et** cible). Réutilisable pour
/// tester le routage du glisser-déposer.
pub fn kanban_slot(col: usize, pos: usize) -> usize {
    // Au-delà de STRIDE cartes dans une colonne, `pos` déborderait sur le champ colonne (l'index
    // plat viserait silencieusement la colonne suivante). Garde en debug ; STRIDE reste large.
    debug_assert!(
        pos < STRIDE,
        "position {pos} hors borne (STRIDE = {STRIDE}) : débordement de colonne"
    );
    col * STRIDE + pos
}

/// Décode un index plat en `(col, pos)` (inverse de [`kanban_slot`]).
fn decode(slot: usize) -> (usize, usize) {
    (slot / STRIDE, slot % STRIDE)
}

/// Une carte : source **et** cible de glisser-déposer. Peinte comme une tuile surélevée, elle affiche
/// soit un **libellé** (carte texte), soit un **contenu widget** fourni par l'application (carte riche :
/// libellé + étiquettes + bouton de suppression…), posé en enfant.
struct Card<Msg> {
    label: String,
    /// Contenu **riche** (0 ou 1 widget) : quand présent, la carte l'héberge au lieu du libellé.
    content: Vec<Box<dyn Widget<Msg>>>,
    /// Emplacement propre (index plat) : sert de `reorder_index` (source saisie **et** cible de dépôt).
    slot: usize,
    from_col: usize,
    from_pos: usize,
    on_move: Option<Rc<dyn Fn(usize, usize, usize, usize) -> Msg>>,
}

impl<Msg: Clone> Widget<Msg> for Card<Msg> {
    fn style(&self) -> Style {
        if self.content.is_empty() {
            // Carte texte : hauteur fixe, libellé peint par la carte.
            Style {
                width: Dimension::Auto,
                height: Dimension::Length(CARD_H),
                ..Default::default()
            }
        } else {
            // Carte riche : le contenu est un enfant, la carte s'adapte (plancher `CARD_H`), avec marge.
            Style {
                width: Dimension::Auto,
                height: Dimension::Auto,
                min_height: Dimension::Length(CARD_H),
                padding: Insets::uniform(8.0),
                align: Align::Center,
                ..Default::default()
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Tuile surélevée ; teintée au survol (repère de préhension).
        let base = theme.surface.lerp(theme.on_surface, 0.05);
        let fill = theme.state_layer(base, theme.on_surface, &status);
        scene.draw_rect(
            bounds,
            fill.fade(o),
            theme.radius,
            1.0,
            theme.border.fade(o),
        );
        // Libellé seulement pour une carte **texte** (une carte riche peint son propre contenu).
        if self.content.is_empty() {
            let ty = bounds.y + (bounds.height - frus_text::line_height(15.0)) * 0.5;
            scene.text(
                Point::new(bounds.x + 12.0, ty),
                self.label.clone(),
                15.0,
                theme.on_surface.fade(o),
            );
        }
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
        self.on_move
            .as_ref()
            .map(|f| f(self.from_col, self.from_pos, to_col, to_pos))
    }

    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Vertical // les cartes glissent verticalement
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
        scene.draw_rect(
            bounds,
            Color::TRANSPARENT,
            theme.radius,
            1.0,
            theme.border.fade(status.opacity * 0.5),
        );
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

    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Vertical
    }

    fn reorder_draggable(&self) -> bool {
        false // cible **seule** : on y dépose, on ne la soulève pas
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
            padding: Insets::uniform(COL_PAD),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Fond de panneau **thémé** (défaut surchargeable via le thème) : un voile discret.
        let bg = theme.surface.lerp(theme.on_surface, 0.04);
        scene.draw_rect(
            bounds,
            bg.fade(status.opacity),
            theme.radius,
            0.0,
            Color::TRANSPARENT,
        );
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
/// Fabrique du **contenu riche** d'une carte : rappelée à chaque reconstruction (widget frais), pour
/// des cartes au-delà du texte (libellé + étiquettes + bouton de suppression…).
pub type CardFactory<Msg> = Rc<dyn Fn() -> Box<dyn Widget<Msg>>>;

/// Cartes d'une colonne : **texte** simple, ou **widgets** riches (par carte).
enum ColCards<Msg> {
    Text(Vec<String>),
    Widgets(Vec<CardFactory<Msg>>),
}

impl<Msg> ColCards<Msg> {
    fn len(&self) -> usize {
        match self {
            ColCards::Text(v) => v.len(),
            ColCards::Widgets(v) => v.len(),
        }
    }
}

pub struct Kanban<Msg = ()> {
    on_move: Option<Rc<dyn Fn(usize, usize, usize, usize) -> Msg>>,
    /// Bouton « + Add card » en bas de chaque colonne (jalon 249) : `on_add(col)` à l'ajout.
    on_add: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// Hauteur explicite de la **zone défilable des cartes** de chaque colonne (jalon 264, façon
    /// Trello). `Some(h)` : les cartes défilent verticalement dans une région de hauteur `h` (titre
    /// fixe au-dessus, bouton « + Add card » fixe en dessous). `None` : la colonne s'étend à la
    /// hauteur de son contenu (comportement d'origine). Voir [`Kanban::card_area_height`].
    card_area_height: Option<f32>,
    /// Défilement vertical par colonne **façon Trello, sans hauteur explicite** (jalon 266) : les
    /// colonnes **remplissent** la hauteur du board (via `Row` étirée) et la zone de cartes de
    /// chaque colonne est un `Scroll` `flex(1)` qui prend le reste puis défile. Prime sur
    /// `card_area_height`. Voir [`Kanban::scrollable_columns`].
    fill_columns: bool,
    columns: Vec<(String, ColCards<Msg>)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Kanban<Msg> {
    /// Crée un tableau ; `on_move(from_col, from_pos, to_col, to_pos)` est émis quand une carte est
    /// **déposée** sur un emplacement (autre carte, ou fin d'une colonne).
    pub fn new(on_move: impl Fn(usize, usize, usize, usize) -> Msg + 'static) -> Self {
        Self {
            on_move: Some(Rc::new(on_move)),
            on_add: None,
            card_area_height: None,
            fill_columns: false,
            columns: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Ajoute une **colonne** titrée avec ses cartes (texte), dans l'ordre.
    pub fn column(
        mut self,
        title: impl Into<String>,
        cards: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let cards = ColCards::Text(cards.into_iter().map(Into::into).collect());
        self.columns.push((title.into(), cards));
        self.rebuild();
        self
    }

    /// Ajoute une colonne dont chaque carte est un **widget** (carte riche : libellé + étiquettes +
    /// bouton de suppression…), fourni par une **fabrique** rappelée à chaque reconstruction. La carte
    /// reste source/cible de glisser-déposer ; ses éléments cliquables (bouton ×…) captent leur clic.
    pub fn column_widgets(
        mut self,
        title: impl Into<String>,
        cards: impl IntoIterator<Item = Box<dyn Fn() -> Box<dyn Widget<Msg>>>>,
    ) -> Self {
        let cards = ColCards::Widgets(cards.into_iter().map(Rc::from).collect());
        self.columns.push((title.into(), cards));
        self.rebuild();
        self
    }

    /// Ajoute un bouton **« + Add card »** au bas de chaque colonne ; `on_add(col)` au clic (l'app
    /// ajoute une carte à la colonne). Sans cet appel, aucun bouton d'ajout.
    pub fn on_add(mut self, on_add: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_add = Some(Rc::new(on_add));
        self.rebuild();
        self
    }

    /// Rend les cartes de chaque colonne **défilables verticalement** dans une région de hauteur `h`
    /// (pixels logiques), façon Trello : le **titre** reste fixe au-dessus, le bouton « + Add card »
    /// fixe en dessous, et seules les cartes (avec la zone de dépôt finale) défilent. Sans cet appel,
    /// la colonne s'étend à la hauteur de son contenu.
    ///
    /// C'est un stopgap **contrôlé par l'application** (façon Flutter : l'app fournit la hauteur) :
    /// un `Scroll` en `flex(1)` ne reçoit pas de hauteur exploitable tant que sa chaîne d'ancêtres ne
    /// lui fournit pas une hauteur **définie** (jalon 263), d'où une hauteur explicite ici.
    pub fn card_area_height(mut self, h: f32) -> Self {
        self.card_area_height = Some(h);
        self.rebuild();
        self
    }

    /// Défilement vertical par colonne **façon Trello, sans hauteur explicite** (jalon 266) : les
    /// colonnes **remplissent** la hauteur du board et la zone de cartes de chaque colonne prend le
    /// reste (sous le titre, au-dessus du bouton « + Add card ») puis **défile**. Contrairement à
    /// [`Kanban::card_area_height`], l'app n'a pas à calculer de hauteur : le board se pose dans un
    /// ancêtre à **hauteur définie** (fenêtre, `Scroll` horizontal borné…) et le remplissage flex
    /// fait le reste. Prime sur `card_area_height` si les deux sont posés.
    pub fn scrollable_columns(mut self) -> Self {
        self.fill_columns = true;
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

    /// Construit une colonne : titre + cartes (chacune source/cible de dépôt) + zone de dépôt finale +
    /// éventuel bouton d'ajout.
    fn build_column(&self, col: usize, title: &str, cards: &ColCards<Msg>) -> Box<dyn Widget<Msg>> {
        let make_card = |pos: usize, content: Vec<Box<dyn Widget<Msg>>>, label: String| Card {
            label,
            content,
            slot: kanban_slot(col, pos),
            from_col: col,
            from_pos: pos,
            on_move: self.on_move.clone(),
        };
        // Les cartes (chacune source **et** cible) + la zone de dépôt finale : c'est la partie
        // **défilable** de la colonne (le titre et le bouton d'ajout restent fixes).
        let mut cards_v: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        match cards {
            ColCards::Text(labels) => {
                for (pos, label) in labels.iter().enumerate() {
                    cards_v.push(Box::new(make_card(pos, Vec::new(), label.clone())));
                }
            }
            ColCards::Widgets(factories) => {
                for (pos, make) in factories.iter().enumerate() {
                    cards_v.push(Box::new(make_card(pos, vec![make()], String::new())));
                }
            }
        }
        // Emplacement d'insertion en fin de colonne (et cible d'une colonne vide).
        cards_v.push(Box::new(DropZone {
            slot: kanban_slot(col, cards.len()),
        }));

        let mut children: Vec<Box<dyn Widget<Msg>>> =
            vec![Box::new(Text::new(title.to_string()).size(16.0))];
        // Largeur intérieure de la colonne (largeur de colonne moins son padding). La zone de cartes
        // défilable enveloppe les cartes dans un `Flex` vertical (une seule enfant pour le `Scroll`).
        let inner_w = COL_W - 2.0 * COL_PAD;
        let list = |cards: Vec<Box<dyn Widget<Msg>>>| {
            cards
                .into_iter()
                .fold(Flex::column().gap(8.0).width(inner_w), Flex::child_boxed)
        };
        if self.fill_columns {
            // **Remplissage** (jalon 266) : la colonne remplit la hauteur du board (Row étirée) ;
            // la zone de cartes en `Scroll` `flex(1)` prend le reste (sous le titre, au-dessus du
            // bouton) puis défile. Pas de hauteur explicite : le flex fait le calcul.
            children.push(Box::new(
                Scroll::new()
                    .axis(Axis::Vertical)
                    .width(inner_w)
                    .flex(1.0)
                    .child(list(cards_v)),
            ));
        } else if let Some(h) = self.card_area_height {
            // Hauteur **explicite** (jalon 264) : repli quand l'ancêtre n'est pas à hauteur définie.
            children.push(Box::new(
                Scroll::new()
                    .axis(Axis::Vertical)
                    .width(inner_w)
                    .height(h)
                    .child(list(cards_v)),
            ));
        } else {
            // Colonne qui s'étend à la hauteur de son contenu (comportement d'origine, cartes nues).
            children.extend(cards_v);
        }
        // Bouton « + Add card » (jalon 249), si demandé.
        if let Some(on_add) = &self.on_add {
            let on_add = on_add.clone();
            children.push(Box::new(
                Button::new("+ Add card")
                    .variant(Variant::Secondary)
                    .size(13.0)
                    .on_press(on_add(col)),
            ));
        }
        Box::new(Column { children })
    }
}

impl<Msg: Clone> Widget<Msg> for Kanban<Msg> {
    fn style(&self) -> Style {
        // Mode **remplissage** (jalon 266) : la rangée prend la hauteur de son parent
        // (`Percent(1.0)` — l'ancêtre est à hauteur définie) et **étire** ses colonnes
        // (`Align::Stretch`) pour que chacune remplisse ce board ; leur zone de cartes en `flex(1)`
        // prend alors le reste puis défile. Sinon : hauteur du contenu, colonnes calées en haut.
        let (align, height) = if self.fill_columns {
            (Align::Stretch, Dimension::Percent(1.0))
        } else {
            (Align::Start, Dimension::Auto)
        };
        Style {
            height,
            flex_direction: FlexDirection::Row,
            gap: 12.0,
            align,
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
        Add(usize),
        Del(usize),
    }

    /// Collecte, en ordre d'arbre, tous les messages `on_click` non nuls d'un sous-arbre.
    fn collect_clicks(w: &dyn Widget<Msg>, out: &mut Vec<Msg>) {
        if let Some(m) = w.on_click() {
            out.push(m);
        }
        for c in w.children() {
            collect_clicks(c.as_ref(), out);
        }
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
        assert_eq!(
            moved,
            Some(Msg::Move(0, 0, 1, 0)),
            "dépôt en (1,0) : déplacement inter-colonnes"
        );
    }

    #[test]
    fn cards_declare_vertical_reorder_axis() {
        fn first_axis(w: &dyn Widget<Msg>) -> Option<ReorderAxis> {
            if w.reorder_index().is_some() {
                return Some(w.reorder_axis());
            }
            w.children().iter().find_map(|c| first_axis(c.as_ref()))
        }
        let board = Kanban::new(Msg::Move).column("A", ["x"]);
        let col0 = &Widget::<Msg>::children(&board)[0];
        assert_eq!(
            first_axis(col0.as_ref()),
            Some(ReorderAxis::Vertical),
            "les cartes glissent verticalement"
        );
    }

    #[test]
    fn rich_cards_host_content_and_add_button_is_present() {
        // Colonne **riche** : chaque carte héberge un bouton × (suppression) ; + un bouton d'ajout.
        let cards: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>> = (0..2)
            .map(|i| {
                Box::new(move || {
                    Box::new(Button::new("x").on_press(Msg::Del(i))) as Box<dyn Widget<Msg>>
                }) as Box<dyn Fn() -> Box<dyn Widget<Msg>>>
            })
            .collect();
        let board = Kanban::new(Msg::Move)
            .on_add(Msg::Add)
            .column_widgets("Col", cards);
        let col0 = &Widget::<Msg>::children(&board)[0];
        // La carte riche reste **réordonnable** (glisser câblé) et route un Move.
        let (idx, moved) = first_card(col0.as_ref(), kanban_slot(0, 1)).expect("une carte");
        assert_eq!(
            idx,
            kanban_slot(0, 0),
            "la carte riche garde son index plat"
        );
        assert_eq!(
            moved,
            Some(Msg::Move(0, 0, 0, 1)),
            "et route toujours le déplacement"
        );
        // Les clics accessibles incluent la suppression (× de chaque carte) et l'ajout (bouton colonne).
        let mut clicks = Vec::new();
        collect_clicks(col0.as_ref(), &mut clicks);
        assert!(
            clicks.contains(&Msg::Del(0)) && clicks.contains(&Msg::Del(1)),
            "× de suppression par carte"
        );
        assert!(
            clicks.contains(&Msg::Add(0)),
            "bouton + Add card de la colonne"
        );
    }

    #[test]
    fn cards_are_draggable_but_the_drop_zone_is_target_only() {
        /// Renvoie `reorder_draggable` du premier réordonnable (carte) et cherche une zone de dépôt
        /// (réordonnable **non** saisissable) dans le sous-arbre.
        fn scan(w: &dyn Widget<Msg>, out: &mut Vec<bool>) {
            if w.reorder_index().is_some() {
                out.push(w.reorder_draggable());
            }
            for c in w.children() {
                scan(c.as_ref(), out);
            }
        }
        let board = Kanban::new(Msg::Move).column("To do", ["A", "B"]);
        let col0 = &Widget::<Msg>::children(&board)[0];
        let mut flags = Vec::new();
        scan(col0.as_ref(), &mut flags);
        // Deux cartes saisissables + une zone de dépôt non saisissable.
        assert_eq!(
            flags.iter().filter(|&&d| d).count(),
            2,
            "les deux cartes sont saisissables"
        );
        assert_eq!(
            flags.iter().filter(|&&d| !d).count(),
            1,
            "la zone de dépôt est cible seule"
        );
    }

    #[test]
    fn board_lays_out_one_widget_per_column() {
        let board = Kanban::new(Msg::Move)
            .column("A", ["x"])
            .column("B", Vec::<String>::new());
        assert_eq!(
            Widget::<Msg>::children(&board).len(),
            2,
            "un widget par colonne"
        );
    }
}
