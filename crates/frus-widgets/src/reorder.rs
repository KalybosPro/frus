//! Réagencement **géométrique** des colonnes pour l'aperçu de réordonnancement
//! d'un tableau : pendant qu'on glisse un en-tête, les colonnes voisines coulissent
//! pour **ouvrir la place** de dépôt et **refermer** le trou de la colonne soulevée —
//! sans que le shell ait à connaître l'appartenance colonne → widgets.
//!
//! On regroupe les primitives par **propriétaire** (une cellule = un `owner`) : chaque
//! cellule coulisse **d'un bloc** (jamais de cisaillement entre son fond et son texte),
//! d'une quantité **continue** fonction de la position du curseur — le coulissement
//! **suit le doigt** au lieu de sauter d'une colonne à l'autre. Les blocs plus larges
//! qu'une colonne (fonds de page/ligne) sont laissés en place.

use std::collections::{HashMap, HashSet};

use frus_core::{Primitive, Rect};

/// Réagence les primitives `prims` pour l'aperçu, en fonction de l'abscisse `cursor_x`
/// du curseur. La colonne **source** (`src`, soulevée, `lifted_owner` = son en-tête) est
/// retirée ; les colonnes de part et d'autre coulissent d'un cran (largeur de la source),
/// **progressivement** à mesure que le curseur les dépasse, pour combler le trou et ouvrir
/// la place de dépôt.
pub fn reflow_reorder_columns(
    prims: &[Primitive],
    src: Rect,
    cursor_x: f32,
    lifted_owner: u64,
) -> Vec<Primitive> {
    let slot = src.width;
    // Au-delà de cette largeur, un bloc couvre plus qu'une cellule (fond de page/ligne) :
    // laissé en place pour ne pas déplacer un arrière-plan entier.
    let max_cell = src.width * 1.5;

    // Boîte englobante par propriétaire (regroupe fond + texte + icône d'une cellule).
    let mut bounds: HashMap<u64, Rect> = HashMap::new();
    for p in prims {
        let b = p.bounds();
        bounds.entry(p.owner()).and_modify(|r| *r = r.union(b)).or_insert(b);
    }

    // Décalage d'un propriétaire : `None` = retiré (colonne source), `Some(dx)` = translaté.
    let shift_of = |owner: u64| -> Option<f32> {
        let b = bounds[&owner];
        let cx = b.x + b.width * 0.5;
        if b.width >= max_cell {
            return Some(0.0); // arrière-plan large : laissé en place
        }
        // Colonne source : retirée (elle flotte en fantôme).
        if owner == lifted_owner || (cx > src.x && cx < src.x + src.width) {
            return None;
        }
        // Largeur de transition : celle de la cellule, ou un cran par défaut (cellules
        // sans fond, réduites à leur texte) pour un coulissement doux et non un saut.
        let w = if b.width > 1.0 { b.width } else { slot };
        if cx >= src.x + src.width {
            // Voisine de droite : coulisse vers la gauche à mesure que le curseur la passe.
            let t = ((cursor_x - (cx - w * 0.5)) / w).clamp(0.0, 1.0);
            Some(-slot * t)
        } else {
            // Voisine de gauche : coulisse vers la droite à mesure que le curseur la passe.
            let t = (((cx + w * 0.5) - cursor_x) / w).clamp(0.0, 1.0);
            Some(slot * t)
        }
    };

    prims
        .iter()
        .filter_map(|p| shift_of(p.owner()).map(|dx| p.translated(dx, 0.0)))
        .collect()
}

/// Réagencement **vertical** pour l'aperçu de réordonnancement des **cartes** d'un Kanban :
/// pendant qu'on glisse une carte, celles de sa colonne **source** qui la suivent **remontent**
/// (le trou de la carte soulevée se referme), et celles de la colonne **cible** situées au niveau
/// ou en dessous de la **ligne d'insertion** **descendent** (la place de dépôt s'ouvre).
///
/// Comme [`reflow_reorder_columns`], purement **géométrique** — aucune connaissance de l'arbre :
/// - `src` : bornes de la carte soulevée (bande x de la **colonne source** et hauteur d'un cran) ;
/// - `line` : la **ligne d'insertion** (bande x de la **colonne cible** et abscisse d'insertion) —
///   `None` si le curseur n'est sur aucune cible (seul le trou source se referme) ;
/// - `lifted` : les propriétaires du **sous-arbre** de la carte soulevée — retirés de l'aperçu
///   (dessinés à part en fantôme).
///
/// Le seuil de **cran** vaut la hauteur de la carte. Un bloc plus **haut** que `1.5×` ce cran est un
/// fond de colonne/page (pas une carte) : laissé en place — pendant vertical du garde `max_cell`.
/// Chaque primitive coulisse selon le **centre** de ses bornes ; les lignes d'insertion se posant aux
/// **bords** des cartes (jamais en plein centre), une carte ne se cisaille pas.
pub fn reflow_reorder_cards(
    prims: &[Primitive],
    src: Rect,
    line: Option<Rect>,
    lifted: &HashSet<u64>,
) -> Vec<Primitive> {
    let slot = src.height;
    // Au-delà : un bloc couvre plus qu'une carte (fond de colonne/page) — laissé en place.
    let max_card = src.height * 1.5;
    let in_band = |cx: f32, x0: f32, w: f32| cx >= x0 && cx <= x0 + w;

    prims
        .iter()
        .filter_map(|p| {
            // Carte soulevée : retirée de l'aperçu (elle flotte en fantôme).
            if lifted.contains(&p.owner()) {
                return None;
            }
            let b = p.bounds();
            // Grand fond (colonne/page) : immobile.
            if b.height >= max_card {
                return Some(p.clone());
            }
            let cx = b.x + b.width * 0.5;
            let cy = b.y + b.height * 0.5;
            let mut dy = 0.0;
            // Colonne **cible** : ce qui est au niveau/en dessous de la ligne descend (trou ouvert).
            if let Some(line) = line {
                if in_band(cx, line.x, line.width) && cy >= line.y {
                    dy += slot;
                }
            }
            // Colonne **source** : ce qui suit la carte soulevée remonte (trou refermé).
            if in_band(cx, src.x, src.width) && cy > src.y {
                dy -= slot;
            }
            Some(p.translated(0.0, dy))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Scene};

    /// Trois cellules côte à côte (100 px, owners 1..3), plus un large fond de page.
    fn scene() -> Scene {
        let mut s = Scene::new();
        s.fill_rect(Rect::new(0.0, 0.0, 300.0, 40.0), Color::WHITE); // fond (large)
        for i in 0..3 {
            s.set_owner((i + 1) as u64);
            s.fill_rect(Rect::new(i as f32 * 100.0, 0.0, 100.0, 40.0), Color::BLACK);
        }
        s
    }

    fn rect_x_of_owner(prims: &[Primitive], owner: u64) -> Option<f32> {
        prims.iter().find_map(|p| match p {
            Primitive::Rect { rect, owner: o, .. } if *o == owner && rect.width < 150.0 => Some(rect.x),
            _ => None,
        })
    }

    #[test]
    fn dragging_far_right_lifts_source_and_slides_middle_fully() {
        let base = scene();
        // Source = colonne 0 (owner 1) ; curseur loin à droite → voisines pleinement coulissées.
        let out = reflow_reorder_columns(base.primitives(), Rect::new(0.0, 0.0, 100.0, 40.0), 1000.0, 1);
        assert_eq!(rect_x_of_owner(&out, 1), None, "colonne source retirée");
        assert!(out.iter().any(|p| matches!(p, Primitive::Rect { rect, .. } if rect.width > 150.0)), "fond conservé");
        assert_eq!(rect_x_of_owner(&out, 2), Some(0.0), "col 1 → 0 (coulissée d'un cran)");
        assert_eq!(rect_x_of_owner(&out, 3), Some(100.0), "col 2 → 100 (place ouverte à droite)");
    }

    #[test]
    fn slide_is_partial_and_follows_the_cursor() {
        let base = scene();
        // Curseur au **centre** de la colonne 1 (owner 2, [100,200], centre 150).
        let out = reflow_reorder_columns(base.primitives(), Rect::new(0.0, 0.0, 100.0, 40.0), 150.0, 1);
        // t = clamp((150 - (150 - 50)) / 100) = 0.5 → coulissement à mi-course (−50).
        assert_eq!(rect_x_of_owner(&out, 2), Some(50.0), "col 1 à mi-coulissement");
        // Colonne 2 pas encore atteinte par le curseur → immobile.
        assert_eq!(rect_x_of_owner(&out, 3), Some(200.0), "col 2 immobile");
    }

    #[test]
    fn dragging_left_slides_middle_right() {
        let base = scene();
        // Source = colonne 2 (owner 3) ; curseur loin à gauche → voisines coulissées de +1 cran.
        let out = reflow_reorder_columns(base.primitives(), Rect::new(200.0, 0.0, 100.0, 40.0), -500.0, 3);
        assert_eq!(rect_x_of_owner(&out, 3), None, "colonne source retirée");
        assert_eq!(rect_x_of_owner(&out, 1), Some(100.0), "col 0 → 100");
        assert_eq!(rect_x_of_owner(&out, 2), Some(200.0), "col 1 → 200");
    }

    /// Deux colonnes (bandes x [0,100] et [120,220]) de 3 cartes (44 px, cran 52) sur fond haut.
    /// Cartes : col A owners 1..3, col B owners 4..6 ; fonds de colonne owners 100 / 200 (hauts).
    fn board() -> Scene {
        let mut s = Scene::new();
        s.set_owner(100);
        s.fill_rect(Rect::new(0.0, 0.0, 100.0, 300.0), Color::WHITE);
        s.set_owner(200);
        s.fill_rect(Rect::new(120.0, 0.0, 100.0, 300.0), Color::WHITE);
        for i in 0..3 {
            let y = i as f32 * 52.0;
            s.set_owner((i + 1) as u64);
            s.fill_rect(Rect::new(0.0, y, 100.0, 44.0), Color::BLACK);
            s.set_owner((i + 4) as u64);
            s.fill_rect(Rect::new(120.0, y, 100.0, 44.0), Color::BLACK);
        }
        s
    }

    fn rect_y_of_owner(prims: &[Primitive], owner: u64) -> Option<f32> {
        prims.iter().find_map(|p| match p {
            Primitive::Rect { rect, owner: o, .. } if *o == owner && rect.height < 100.0 => Some(rect.y),
            _ => None,
        })
    }

    #[test]
    fn lifting_a_card_closes_the_source_gap() {
        let base = board();
        // Carte soulevée = col A, carte du haut (owner 1) ; pas de cible (line = None).
        let lifted = HashSet::from([1]);
        let out = reflow_reorder_cards(base.primitives(), Rect::new(0.0, 0.0, 100.0, 44.0), None, &lifted);
        assert_eq!(rect_y_of_owner(&out, 1), None, "carte soulevée retirée");
        assert_eq!(rect_y_of_owner(&out, 2), Some(8.0), "carte suivante remonte d'un cran (52 − 44)");
        assert_eq!(rect_y_of_owner(&out, 3), Some(60.0), "et la dernière aussi (104 − 44)");
        assert_eq!(rect_y_of_owner(&out, 4), Some(0.0), "colonne voisine intacte");
    }

    #[test]
    fn insertion_line_opens_a_hole_in_the_target_column() {
        let base = board();
        // Carte soulevée = col B, carte du haut (owner 4) ; cible = col A, insertion avant la 2e
        // carte (ligne au bord supérieur y=52 de owner 2).
        let lifted = HashSet::from([4]);
        let line = Rect::new(0.0, 52.0, 100.0, 3.0);
        let out =
            reflow_reorder_cards(base.primitives(), Rect::new(120.0, 0.0, 100.0, 44.0), Some(line), &lifted);
        // Colonne source (B) : le trou se referme.
        assert_eq!(rect_y_of_owner(&out, 4), None, "carte soulevée retirée");
        assert_eq!(rect_y_of_owner(&out, 5), Some(8.0), "col source : carte suivante remonte");
        assert_eq!(rect_y_of_owner(&out, 6), Some(60.0), "col source : dernière remonte");
        // Colonne cible (A) : la place s'ouvre sous la ligne.
        assert_eq!(rect_y_of_owner(&out, 1), Some(0.0), "au-dessus de la ligne : immobile");
        assert_eq!(rect_y_of_owner(&out, 2), Some(96.0), "sous la ligne : descend d'un cran (52 + 44)");
        assert_eq!(rect_y_of_owner(&out, 3), Some(148.0), "et la suivante aussi (104 + 44)");
    }

    #[test]
    fn tall_backgrounds_stay_put() {
        let base = board();
        let lifted = HashSet::from([1]);
        let line = Rect::new(0.0, 52.0, 100.0, 3.0);
        let out =
            reflow_reorder_cards(base.primitives(), Rect::new(0.0, 0.0, 100.0, 44.0), Some(line), &lifted);
        // Les deux fonds de colonne (hauteur 300 > 1.5×44) restent à y = 0.
        let bgs: Vec<f32> = out
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } if rect.height > 150.0 => Some(rect.y),
                _ => None,
            })
            .collect();
        assert_eq!(bgs, vec![0.0, 0.0], "fonds de colonne immobiles");
    }
}
