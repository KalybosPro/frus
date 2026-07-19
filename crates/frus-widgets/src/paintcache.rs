//! Cache de **frontière de repaint** : retient, entre les frames, la sortie
//! peinte (primitives + cartes d'interaction) d'un sous-arbre marqué
//! [`crate::Widget::repaint_boundary`], et la **réutilise telle quelle** tant
//! que sa géométrie et l'état d'interaction de ses descendants n'ont pas bougé.
//!
//! C'est le pendant *peinture* du cache de relayout (`relayout.rs`, jalon 55) :
//! là où le cache de layout réutilise les **rectangles** quand le *style/la
//! structure* sont stables, celui-ci réutilise les **primitives** quand l'*état
//! lu à la peinture* (survol, focus, valeur animée, opacité, curseur) et la
//! **géométrie** sont stables. Un widget qui s'anime ailleurs sur l'écran ne
//! force plus le repaint d'un sous-arbre statique.
//!
//! ## Correction
//! - Toute reconstruction de la `view` (changement d'état, de thème, de taille)
//!   **incrémente la génération** ; une entrée d'une génération périmée est
//!   ignorée. La configuration des widgets étant alors identique d'une frame à
//!   l'autre (l'arbre est le **même** objet retenu tant que `build` ne tourne
//!   pas), une entrée de génération courante correspond à une config identique.
//! - L'**empreinte** couvre le reste : l'état d'interaction (`Status`) de chaque
//!   descendant et les rectangles absolus du sous-arbre. Empreinte + génération
//!   égales ⇒ la peinture serait **bit-à-bit identique** → on rejoue le cache.
//!
//! Le cache ne connaît pas le type `Msg` de l'application (le `Runtime` est
//! générique-agnostique) : la donnée est stockée **effacée** derrière un
//! `Box<dyn Any>`, et `ui.rs` la redescend vers son `BoundaryData<Msg>` concret
//! (une seule instance de `Msg` par app → le `downcast` réussit toujours).

use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::interaction::WidgetId;

/// Une entrée : la génération et l'empreinte sous lesquelles la sortie a été
/// capturée, le nombre de rectangles consommés par le sous-arbre (pour avancer
/// l'index de parcours sur un *hit*), et la donnée peinte effacée.
struct Slot {
    generation: u64,
    fingerprint: u64,
    rect_count: usize,
    data: Box<dyn Any>,
}

/// Le cache de peinture, retenu dans le [`crate::Runtime`] d'une frame à l'autre.
#[derive(Default)]
pub struct PaintCache {
    entries: HashMap<WidgetId, Slot>,
    /// Frontières touchées durant la frame courante (pour évincer les disparues).
    touched: HashSet<WidgetId>,
    /// Génération courante : incrémentée à chaque reconstruction de la `view`.
    generation: u64,
    hits: u32,
    misses: u32,
    last_hits: u32,
    last_misses: u32,
}

impl PaintCache {
    /// Invalide tout le cache **logiquement** : la `view` a été reconstruite, la
    /// config des widgets a pu changer. Les entrées de l'ancienne génération ne
    /// seront plus jamais des *hits*.
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// La donnée effacée d'une frontière si sa génération **et** son empreinte
    /// correspondent (un *hit*), avec le nombre de rectangles qu'elle couvre.
    /// Marque la frontière comme touchée (pour la survie en fin de frame).
    pub(crate) fn get(&mut self, key: WidgetId, fingerprint: u64) -> Option<(usize, &dyn Any)> {
        self.touched.insert(key);
        let slot = self.entries.get(&key)?;
        if slot.generation == self.generation && slot.fingerprint == fingerprint {
            return Some((slot.rect_count, slot.data.as_ref()));
        }
        None
    }

    /// Mémorise (ou remplace) la sortie peinte d'une frontière sous la
    /// génération courante.
    pub(crate) fn put(&mut self, key: WidgetId, fingerprint: u64, rect_count: usize, data: Box<dyn Any>) {
        self.entries.insert(
            key,
            Slot {
                generation: self.generation,
                fingerprint,
                rect_count,
                data,
            },
        );
    }

    /// Compteur de diagnostic : une frontière réutilisée cette frame.
    pub(crate) fn note_hit(&mut self) {
        self.hits += 1;
    }

    /// Compteur de diagnostic : une frontière repeinte cette frame (miss, ou
    /// non-cachable).
    pub(crate) fn note_miss(&mut self) {
        self.misses += 1;
    }

    /// À appeler en fin de frame : oublie les frontières non touchées (widgets
    /// disparus) et fige les compteurs de diagnostic de la frame.
    pub(crate) fn end_frame(&mut self) {
        let touched = std::mem::take(&mut self.touched);
        self.entries.retain(|id, _| touched.contains(id));
        self.last_hits = self.hits;
        self.last_misses = self.misses;
        self.hits = 0;
        self.misses = 0;
    }

    /// Réutilisations / repaints de la dernière frame terminée (diagnostic).
    pub fn last_frame_stats(&self) -> (u32, u32) {
        (self.last_hits, self.last_misses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: WidgetId = WidgetId::ROOT;

    #[test]
    fn stored_entry_is_a_hit_until_generation_bumps() {
        let mut c = PaintCache::default();
        c.put(A, 42, 3, Box::new(7u32));
        // Génération courante + empreinte égales → hit, avec le rect_count.
        let (rc, any) = c.get(A, 42).expect("hit");
        assert_eq!(rc, 3);
        assert_eq!(*any.downcast_ref::<u32>().unwrap(), 7);
        // Empreinte différente → pas de hit.
        assert!(c.get(A, 99).is_none());
        // Génération périmée → l'entrée n'est plus un hit.
        c.bump_generation();
        assert!(c.get(A, 42).is_none());
    }

    #[test]
    fn end_frame_evicts_untouched_boundaries() {
        let mut c = PaintCache::default();
        c.put(A, 1, 1, Box::new(()));
        c.put(A.child(0), 1, 1, Box::new(()));
        // `put` seul ne marque pas « touché » ; simule une frame où seule A est vue.
        c.get(A, 1);
        c.end_frame();
        assert!(c.entries.contains_key(&A));
        assert!(!c.entries.contains_key(&A.child(0)), "frontière disparue évincée");
    }

    #[test]
    fn frame_stats_freeze_at_end_frame() {
        let mut c = PaintCache::default();
        c.note_hit();
        c.note_hit();
        c.note_miss();
        c.end_frame();
        assert_eq!(c.last_frame_stats(), (2, 1));
        // Les compteurs repartent de zéro pour la frame suivante.
        c.end_frame();
        assert_eq!(c.last_frame_stats(), (0, 0));
    }
}
