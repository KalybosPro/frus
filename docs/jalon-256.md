# Jalon 256 — Consolidation : registres transformés (ui.rs) + facteur de réagencement partagé

## Analyse

La revue du jalon 254 a relevé deux duplications dans le domaine glisser-déposer :
1. **`ui.rs`** — deux blocs quasi identiques (frontière de `walk` et `emit_transformed_child`)
   capturaient les bornes des registres puis, après composition d'un calque transformé, contre-
   transformaient le hit-test et mappaient les rectangles de focus/défilement/glisser/**réordonnancement**/
   accessibilité. C'est exactement là que le jalon 250 avait **oublié `reorderables`** dans un seul des
   deux blocs — le risque « on met à jour une liste, pas l'autre ».
2. **`reorder.rs`** — les deux réagencements partagent le garde « fond vs cellule » (`× 1.5`).

## Décisions techniques

- **Un seul point de transformation des registres.** Nouveau `transform_interaction_registries(base,
  matrix)` : contre-transforme clics/appuis longs (`M⁻¹`) et, si `matrix` est alignée sur les axes,
  mappe les rectangles des cinq registres. Les deux sites l'appellent après avoir capturé `xform_base()`.
  Un `struct XformBase` porte les bornes basses — **distinct de `Snapshot`** car il **inclut
  `reorderables`** (jamais mis en cache, mais bien à transformer). Impossible désormais d'oublier une
  liste dans un seul chemin : il n'y en a plus qu'un.
- **Enveloppe de calque laissée en place.** Le `split_off`/`Layer` diffère par le propriétaire (`id`
  vs `owner`) et l'appel de parcours (`walk_node` vs `walk`) : gardé inline dans chaque site, seule la
  partie **identique** (et fragile) est factorisée.
- **Facteur `OVERSIZE_FACTOR` partagé** par `reflow_reorder_columns`/`reflow_reorder_cards`. Les
  **corps** ne sont **pas** fusionnés : le modèle d'interaction diffère (colonnes = coulissement
  **continu** suivant le curseur ; cartes = décalage **binaire** selon la ligne d'insertion) ; les
  fondre obscurcirait les deux. Seule la constante (même idée, axes transposés) est mutualisée, avec un
  commentaire explicitant la relation.

## Implémentation

- `frus-widgets/src/ui.rs` : `struct XformBase`, `xform_base()`, `transform_interaction_registries()` ;
  les deux blocs de composition transformée appellent le helper.
- `frus-widgets/src/reorder.rs` : `const OVERSIZE_FACTOR = 1.5` remplace les deux `× 1.5`.

## Vérification

- **Refactor sans changement de comportement.** Widgets **392**, **goldens 77 inchangés** (les chemins
  de frontière cachée + calques transformés — `RotatedBox`/`FittedBox`/`InteractiveViewer` — sont
  couverts et restent bit-à-bit identiques), shell **27**, doctests **6**.

## Notes

- Le helper centralise le point d'échec du jalon 250 : tout nouveau registre à transformer sous calque
  s'ajoute désormais à **un** endroit.

## Reste

- Couverture du réagencement **même-colonne** (chevauchement source/cible → décalage net nul).
- Inertie/ressort **vertical** du coulissement (parité avec l'horizontal).
- Unifier l'ombre de `Card`/`Toast` sur `theme.scheme.shadow` (relevé au jalon 255).
