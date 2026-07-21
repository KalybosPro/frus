//! Cache de **frontière de relayout** : retient, entre les frames, les rectangles
//! calculés de chaque racine de mise en page, indexés par identité.
//!
//! Frus reconstruit l'arbre de widgets à chaque frame et, jusqu'ici, relançait
//! taffy *from scratch* à chaque racine de layout (`build_ui`, chaque défilable,
//! chaque écran, chaque overlay…). Or la géométrie ne dépend **que** du *style* et
//! de la *structure* de l'arbre et des *contraintes* du parent — pas des couleurs
//! ni du texte, qui ne touchent que la peinture. Un survol, un curseur qui
//! clignote, une couleur qui s'anime → même layout.
//!
//! Ce cache mémorise, par racine (`WidgetId`), `(empreinte, contraintes,
//! rectangles)`. Si l'empreinte de mise en page et les contraintes sont
//! inchangées, on **réutilise les rectangles** et on ne rappelle pas taffy. La
//! sortie est **bit-à-bit identique** au calcul complet — seule la performance
//! change. C'est aussi le socle du futur système de phases : « empreinte changée »
//! *est* le bit « layout sale » d'une racine.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use frus_core::{Rect, Size};

use crate::interaction::WidgetId;
use crate::runtime::Runtime;
use crate::ui::{build_layout, child_id, effective_style};
use crate::widget::Widget;

/// Contraintes passées à taffy pour une racine de layout. `free_x`/`free_y`
/// laissent un axe **libre** (le contenu prend sa taille naturelle) — c'est le
/// cas du contenu défilable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Constraints {
    pub w: f32,
    pub h: f32,
    pub free_x: bool,
    pub free_y: bool,
}

impl Constraints {
    /// Les deux axes contraints à `size` (mise en page classique).
    pub fn definite(size: Size) -> Self {
        Self {
            w: size.width,
            h: size.height,
            free_x: false,
            free_y: false,
        }
    }

    /// Un contenu défilable : chaque axe contraint ou libre.
    pub fn scroll(w: f32, h: f32, free_x: bool, free_y: bool) -> Self {
        Self {
            w,
            h,
            free_x,
            free_y,
        }
    }
}

/// Une entrée du cache : l'empreinte de la dernière racine calculée sous cette
/// identité, ses contraintes, et les rectangles produits (ordre préfixe).
struct Entry {
    signature: u64,
    constraints: Constraints,
    rects: Vec<Rect>,
}

/// Le cache de relayout, retenu dans le [`crate::Runtime`] d'une frame à l'autre.
#[derive(Default)]
pub struct LayoutCache {
    entries: HashMap<WidgetId, Entry>,
    /// Racines touchées durant la frame courante (pour évincer les disparues).
    touched: HashSet<WidgetId>,
    /// Diagnostic : réutilisations / recalculs de la **dernière** frame.
    last_hits: u32,
    last_misses: u32,
    hits: u32,
    misses: u32,
}

impl LayoutCache {
    /// Rectangles (ordre préfixe) de la racine `key` sous les contraintes `c`.
    /// Réutilise le cache si l'empreinte de mise en page **et** les contraintes
    /// sont inchangées ; sinon relance taffy et mémorise le résultat.
    pub(crate) fn rects<Msg>(
        &mut self,
        key: WidgetId,
        root: &dyn Widget<Msg>,
        runtime: &Runtime,
        c: Constraints,
    ) -> Vec<Rect> {
        self.touched.insert(key);
        let signature = layout_signature(root, key, runtime);
        if let Some(entry) = self.entries.get(&key) {
            if entry.signature == signature && entry.constraints == c {
                self.hits += 1;
                return entry.rects.clone();
            }
        }
        self.misses += 1;
        let rects = compute_rects(root, key, runtime, c);
        self.entries.insert(
            key,
            Entry {
                signature,
                constraints: c,
                rects: rects.clone(),
            },
        );
        rects
    }

    /// À appeler en fin de frame : oublie les racines non touchées (widgets
    /// disparus) et fige les compteurs de diagnostic de la frame.
    pub(crate) fn end_frame(&mut self) {
        let touched = std::mem::take(&mut self.touched);
        self.entries.retain(|id, _| touched.contains(id));
        self.last_hits = self.hits;
        self.last_misses = self.misses;
        self.hits = 0;
        self.misses = 0;
    }

    /// Réutilisations / recalculs de la dernière frame terminée (diagnostic).
    pub fn last_frame_stats(&self) -> (u32, u32) {
        (self.last_hits, self.last_misses)
    }
}

/// Calcule les rectangles absolus d'une racine — le chemin « complet » (build +
/// taffy + collecte), emprunté à chaque *miss* du cache.
fn compute_rects<Msg>(
    root: &dyn Widget<Msg>,
    key: WidgetId,
    runtime: &Runtime,
    c: Constraints,
) -> Vec<Rect> {
    let mut layout = frus_layout::Layout::new();
    let node = build_layout(root, key, runtime, &mut layout);
    // `compute_scroll(_, _, false, false)` équivaut à `compute` (deux axes
    // `Definite`) : un seul chemin couvre les deux cas.
    layout.compute_scroll(node, c.w, c.h, c.free_x, c.free_y);
    layout
        .absolute_rects(node)
        .into_iter()
        .map(|(rect, _)| rect)
        .collect()
}

/// Empreinte 64-bit de la **mise en page** d'un sous-arbre : styles + structure,
/// en suivant **exactement** le branchement de [`build_layout`] (défilable/
/// navigateur/liste/pile = feuille ; portail = ancre seule). Couleurs, textes et
/// messages en sont exclus (ils ne touchent que la peinture).
pub(crate) fn layout_signature<Msg>(root: &dyn Widget<Msg>, id: WidgetId, runtime: &Runtime) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_node(root, id, runtime, &mut hasher);
    hasher.finish()
}

fn hash_node<Msg, H: Hasher>(widget: &dyn Widget<Msg>, id: WidgetId, runtime: &Runtime, hasher: &mut H) {
    // Ces branches doivent rester alignées sur `build_layout` : la forme de
    // l'arbre taffy (donc le nombre et l'ordre des rectangles) en dépend.
    // On hache le style **effectif** (taille animée injectée) — même source que
    // `build_layout`, donc l'empreinte change tant que la taille bouge.
    if widget.scroll_content().is_some()
        || widget.navigator().is_some()
        || widget.virtual_list().is_some()
        || widget.layout_builder().is_some()
        || widget.stack()
    {
        1u8.hash(hasher);
        effective_style(widget, id, runtime).layout_hash(hasher);
        return;
    }
    if widget.overlay().is_some() {
        2u8.hash(hasher);
        effective_style(widget, id, runtime).layout_hash(hasher);
        let anchor = widget.children()[0].as_ref();
        hash_node(anchor, child_id(id, 0, anchor), runtime, hasher);
        return;
    }
    let children = widget.children();
    3u8.hash(hasher);
    effective_style(widget, id, runtime).layout_hash(hasher);
    // Feuille mesurée : son **contenu** (texte…) influe sur la géométrie sans
    // passer par le style — sans cette empreinte, deux contenus différents
    // seraient confondus et le cache garderait une vieille mise en page.
    widget.measure_key().hash(hasher);
    children.len().hash(hasher);
    for (i, child) in children.iter().enumerate() {
        hash_node(child.as_ref(), child_id(id, i, child.as_ref()), runtime, hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};

    fn sig<Msg>(w: &dyn Widget<Msg>) -> u64 {
        layout_signature(w, WidgetId::ROOT, &Runtime::default())
    }

    #[test]
    fn identical_trees_share_a_signature() {
        let a: Container<()> = Container::new().width(100.0).height(40.0);
        let b: Container<()> = Container::new().width(100.0).height(40.0);
        assert_eq!(sig(&a), sig(&b));
    }

    #[test]
    fn a_size_change_changes_the_signature() {
        let a: Container<()> = Container::new().width(100.0).height(40.0);
        let b: Container<()> = Container::new().width(101.0).height(40.0);
        assert_ne!(sig(&a), sig(&b));
    }

    #[test]
    fn child_count_changes_the_signature() {
        let one: Flex<()> = Flex::row().child(Container::new());
        let two: Flex<()> = Flex::row().child(Container::new()).child(Container::new());
        assert_ne!(sig(&one), sig(&two));
    }

    #[test]
    fn cache_hits_on_repeated_identical_root() {
        let mut cache = LayoutCache::default();
        let rt = Runtime::default();
        let key = WidgetId::ROOT;
        let c = Constraints::definite(Size::new(200.0, 100.0));

        let tree: Container<()> = Container::new().width(200.0).height(100.0);
        let first = cache.rects(key, &tree, &rt, c);
        let second = cache.rects(key, &tree, &rt, c);
        assert_eq!(first, second, "mêmes rectangles");
        // 1 miss (calcul), puis 1 hit (réutilisation).
        assert_eq!((cache.hits, cache.misses), (1, 1));
    }

    #[test]
    fn changed_constraints_miss() {
        let mut cache = LayoutCache::default();
        let rt = Runtime::default();
        let key = WidgetId::ROOT;
        let tree: Container<()> = Container::new();
        cache.rects(key, &tree, &rt, Constraints::definite(Size::new(200.0, 100.0)));
        cache.rects(key, &tree, &rt, Constraints::definite(Size::new(300.0, 100.0)));
        assert_eq!((cache.hits, cache.misses), (0, 2), "taille changée → recalcul");
    }

    #[test]
    fn end_frame_evicts_untouched_roots() {
        let mut cache = LayoutCache::default();
        let rt = Runtime::default();
        let c = Constraints::definite(Size::new(10.0, 10.0));
        let tree: Container<()> = Container::new();
        cache.rects(WidgetId::ROOT, &tree, &rt, c);
        cache.rects(WidgetId::ROOT.child(0), &tree, &rt, c);
        cache.end_frame();
        assert_eq!(cache.entries.len(), 2);

        // Frame suivante : une seule racine touchée → l'autre est évincée.
        cache.rects(WidgetId::ROOT, &tree, &rt, c);
        cache.end_frame();
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key(&WidgetId::ROOT));
    }
}
