//! Réagencement **géométrique** des colonnes pour l'aperçu de réordonnancement
//! d'un tableau : pendant qu'on glisse un en-tête, les colonnes voisines coulissent
//! pour **ouvrir la place** de dépôt et **refermer** le trou laissé par la colonne
//! soulevée — sans que le shell ait à connaître l'appartenance colonne → widgets.
//!
//! L'opération est purement géométrique : elle classe chaque primitive par son
//! centre en `x` (colonne source retirée, colonnes intermédiaires translatées d'un
//! cran) et **ignore** les primitives trop larges (fonds de page/ligne) pour ne pas
//! déplacer un arrière-plan entier.

use frus_core::{Primitive, Rect};

/// Réagence les primitives `prims` pour l'aperçu : la colonne **source** (`src`,
/// soulevée, `lifted_owner` = son en-tête) est retirée ; les colonnes entre la source
/// et la **cible** (`target`) coulissent d'un cran (largeur de la source) pour combler
/// le trou et ouvrir la place de dépôt. `to_right` = la cible est à droite de la source.
pub fn reflow_reorder_columns(
    prims: &[Primitive],
    src: Rect,
    target: Rect,
    to_right: bool,
    lifted_owner: u64,
) -> Vec<Primitive> {
    let slot = src.width;
    // Au-delà de cette largeur, une primitive couvre plus qu'une cellule (fond de page
    // ou de ligne) : on la laisse en place pour ne pas déplacer un arrière-plan entier.
    let max_cell = src.width * 1.5;
    // Bande des colonnes intermédiaires à faire coulisser, et sens du coulissement.
    let (band0, band1, shift) = if to_right {
        (src.x + src.width, target.x + target.width, -slot)
    } else {
        (target.x, src.x, slot)
    };

    let mut out = Vec::with_capacity(prims.len());
    for p in prims {
        let b = p.bounds();
        let cx = b.x + b.width * 0.5;
        let cell_scale = b.width < max_cell;
        // Colonne source : retirée (elle flotte en fantôme).
        let in_source = cx > src.x && cx < src.x + src.width;
        if cell_scale && (p.owner() == lifted_owner || in_source) {
            continue;
        }
        // Colonnes intermédiaires : coulissées d'un cran.
        if cell_scale && cx > band0 && cx <= band1 {
            out.push(p.translated(shift, 0.0));
        } else {
            out.push(p.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Scene};

    /// Trois cellules côte à côte (100 px), plus un large fond de page.
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
    fn dragging_right_lifts_source_and_slides_middle_left() {
        let base = scene();
        // Source = colonne 0 (owner 1), cible = colonne 2 (owner 3), vers la droite.
        let out = reflow_reorder_columns(
            base.primitives(),
            Rect::new(0.0, 0.0, 100.0, 40.0),
            Rect::new(200.0, 0.0, 100.0, 40.0),
            true,
            1,
        );
        // La source (owner 1) est retirée ; le fond large subsiste.
        assert_eq!(rect_x_of_owner(&out, 1), None, "colonne source retirée");
        assert!(out.iter().any(|p| matches!(p, Primitive::Rect { rect, .. } if rect.width > 150.0)), "fond conservé");
        // Colonnes intermédiaires (owners 2 et 3) coulissées de −100.
        assert_eq!(rect_x_of_owner(&out, 2), Some(0.0), "col 1 → 0");
        assert_eq!(rect_x_of_owner(&out, 3), Some(100.0), "col 2 → 100 (place de dépôt ouverte à droite)");
    }

    #[test]
    fn dragging_left_slides_middle_right() {
        let base = scene();
        // Source = colonne 2 (owner 3), cible = colonne 0 (owner 1), vers la gauche.
        let out = reflow_reorder_columns(
            base.primitives(),
            Rect::new(200.0, 0.0, 100.0, 40.0),
            Rect::new(0.0, 0.0, 100.0, 40.0),
            false,
            3,
        );
        assert_eq!(rect_x_of_owner(&out, 3), None, "colonne source retirée");
        // Colonnes 0 et 1 (owners 1, 2) coulissées de +100.
        assert_eq!(rect_x_of_owner(&out, 1), Some(100.0), "col 0 → 100");
        assert_eq!(rect_x_of_owner(&out, 2), Some(200.0), "col 1 → 200");
    }
}
