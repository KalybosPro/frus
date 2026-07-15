# Jalon 55 — Cache de frontière de relayout (layout retenu au-dessus de taffy)

Première brique du **layout retenu** réclamé par `docs/idees-flutter.md` (§1) — et
prérequis des deux items moteur restants (listes « dirty » par phase, invalidation
ciblée).

## Le problème

Frus reconstruit l'arbre de widgets à chaque frame (Elm). Jusqu'ici, **chaque
racine de mise en page** (`build_ui`, chaque défilable, chaque écran de navigation,
chaque overlay, chaque élément de liste virtualisée, chaque couche de pile) relançait
taffy *from scratch* : `Layout::new()` → `build_layout` (allocation de tout l'arbre
taffy) → `compute` → `absolute_rects`. Or la géométrie ne dépend **que** du *style*
et de la *structure* de l'arbre et des *contraintes* du parent — **pas** des couleurs
ni du texte, qui ne touchent que la peinture. Un survol, un curseur qui clignote, une
couleur ou une opacité qui s'anime → layout **identique**, pourtant intégralement
recalculé.

## La solution : un cache par racine, indexé par identité

Nouveau module `frus-widgets/relayout.rs` : `LayoutCache` mémorise, par racine
(`WidgetId`), `(empreinte, contraintes, rectangles)`. À chaque racine :

1. **Empreinte de mise en page** (`layout_signature`) : un hash 64-bit du sous-arbre
   qui suit **exactement** le branchement de `build_layout` (défilable/navigateur/
   liste/pile = feuille ; portail = ancre seule), mêlant pour chaque nœud son
   `Style::layout_hash` (nouveau — champs géométriques hachés par motif binaire) et le
   nombre d'enfants. Couleurs/texte/messages **exclus**.
2. Si empreinte **et** contraintes sont inchangées → on **réutilise les rectangles**
   et taffy n'est pas rappelé. Sinon, recalcul et mémorisation.

Le résultat est **bit-à-bit identique** au calcul complet : sur un *hit* on renvoie
les rectangles qu'on aurait produits. Seule la performance change. Le pire cas d'une
collision de hash (astronomiquement improbable, 64 bits) est un layout figé d'une
frame — jamais un crash.

Les **7 sites** de layout de `ui.rs` passent par le cache (racine principale,
défilable, écran, overlay, élément de liste, couche de pile, `LayoutBuilder`),
chacun sous une identité distincte. Le cache vit dans le `Runtime` derrière un
`RefCell` (mutabilité intérieure : `build_ui` ne tient qu'un `&Runtime`). En fin de
frame, `end_frame()` **évince** les racines non touchées (widgets disparus) et fige
des compteurs `(hits, misses)` de diagnostic.

## Pourquoi c'est le bon socle

- **Non régressif** : sortie identique, prouvée par les 122 tests existants inchangés.
- **Gain réel** : pendant toute animation de couleur/opacité/survol (le cas le plus
  fréquent), l'empreinte est stable → taffy est **entièrement sauté** chaque frame.
  Idem au défilement (offset ≠ layout) et pendant une transition d'écran (les écrans
  sont statiques, seul le décalage de peinture bouge).
- **Prérequis des phases** : « empreinte changée » *est* le bit « layout sale » d'une
  racine — la base du futur pipeline `build → layout → paint → composite` à listes
  « dirty » séparées.

## Validation

- `frus-widgets` : **129 tests** (+7 : 6 unitaires du cache — empreinte stable/qui
  change, hit/miss, éviction — et 1 bout-en-bout par `build_ui` : frame 2 réutilise la
  racine, un redimensionnement la recalcule).
- Toute la suite existante verte (sortie inchangée) : `frus-core` 37, `frus-demo` 15,
  shell 7, layout 3, gpu 4, text 2.
- `cargo build --workspace` sans avertissement ; démo lancée 8 s sans panique
  (défilement, transitions, overlays, chrono continus) — cache actif dans le chemin
  chaud, aucun conflit d'emprunt `RefCell`.

## Limites / suite

- Le cache retient le **résultat** (rectangles) par racine ; il ne retient pas encore
  l'arbre taffy lui-même (pas de `mark_dirty` par nœud intra-racine). Un changement
  minime dans une grande racine recalcule toute la racine — l'étape suivante, si
  besoin, est un arbre taffy persistant réconcilié par identité.
- Prochain jalon (§1) : **phases de frame + listes « dirty » séparées**
  (`build → layout → paint → composite`), chaque `Msg`/`Command` posant le bit le plus
  étroit possible — le cache de relayout en est la moitié « layout ».
