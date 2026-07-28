# Jalon 158 — Réordonnancement : fantôme fidèle (texte compris)

## Analyse

L'aperçu de réordonnancement (jalon 155) soulevait une **carte pleine** sans contenu :
on voyait bouger un rectangle, pas la colonne. Le jalon 155 avait laissé le « fantôme
texte compris » au Reste, buté sur un obstacle : **rejouer** les primitives de l'en-tête
translatées bute sur leur **découpe** — chaque primitive porte le rectangle de découpe
hérité, qui rognerait le fantôme à la colonne source.

## Décisions techniques

- **Dé-découpe d'une primitive.** Nouveau `Primitive::with_clip(clip)` (frus-core) :
  copie une primitive en **remplaçant** sa découpe. Le shell capture les primitives de
  l'en-tête saisi (`owner == id`), les **translate** (`translated(dx, −2)`) puis les
  **dé-découpe** (`with_clip(UNBOUNDED)`) — le fantôme s'affiche en entier, où qu'il aille.

- **Face fidèle, repli propre.** `draw_reorder_overlay` reçoit désormais les primitives du
  fantôme : ombre portée, **face = primitives de l'en-tête** (fond + texte + tri) rejouées,
  bord `primary` par-dessus. Si la capture est **vide** (cas dégénéré), une face pleine sert
  de repli — la fonction reste pure et testable (le test passe `&[]`).

- **`Primitive` ré-exporté** par frus-widgets pour que le shell (qui ne dépend pas de
  frus-core) nomme le type dans la signature.

## Implémentation

- `scene.rs` (frus-core) : `Primitive::with_clip(&self, clip) -> Primitive` (toutes les
  variantes).
- `lib.rs` (frus-widgets) : ré-export `Primitive`.
- `app.rs` (shell) : `paint_reorder_preview` capture + translate + dé-découpe les
  primitives de l'en-tête ; `draw_reorder_overlay(…, ghost: &[Primitive])` rejoue la face
  fidèle (repli plein si vide).
- `goldens.rs` : golden `table_reorder_preview` reconstruisant la superposition du shell
  (source estompée, indicateur, carte fidèle « Role »).

## Vérification

- **Unitaire** (forme, sans GPU) : `draw_reorder_overlay` avec fantôme vide → repli plein
  (4 primitives avec cible, 3 sans) ; `Primitive::with_clip` couvert par le rendu.
- **Golden** `table_reorder_preview` **inspecté** : l'en-tête « Role » **estompé** à sa
  place, une **carte soulevée bordée `primary` portant le texte « Role »** décalée vers la
  droite, et l'**indicateur de dépôt** au bord de la colonne cible. Le fantôme reprend
  fidèlement l'en-tête, texte compris.
- `cargo test --workspace` **vert**, sans avertissement.

## Reste

- **Décalage animé des colonnes voisines** (elles s'écartent pour ouvrir la place de
  dépôt) — le dernier morceau « façon `ReorderableListView` ».
- **Opacité du fantôme** (< 1) pour un effet plus « soulevé » : demande d'envelopper les
  primitives capturées dans un `Primitive::Layer { opacity }`.
