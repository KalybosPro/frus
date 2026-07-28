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

use std::collections::HashMap;

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
}
