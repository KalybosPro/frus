# Jalon 105 — Container : `alignment` + `decoration` composite (parité Flutter)

## Analyse

Deux manques de parité Flutter subsistaient sur `Container` :

1. **Ancrer l'enfant.** Sans réglage, l'enfant s'étire pour remplir la boîte
   (défaut flex `Start`/`Stretch`). Flutter expose `Container(alignment:)` pour le
   centrer, le coller à un coin, un bord…
2. **Décoration d'un bloc.** La boîte se décorait champ par champ (`.color`,
   `.border`, `.radius`, `.gradient`, `.shadow`). Flutter réunit tout dans un
   `BoxDecoration` réutilisable passé via `Container(decoration:)`.

## Décisions techniques

- **`Alignment` (frus-core).** Les neuf ancrages nommés de Flutter
  (`TopLeft`…`Center`…`BottomRight`), chacun projeté sur deux bords indépendants via
  `horizontal()` / `vertical()` → [`AlignEdge`] (`Start`/`Center`/`End`). Type
  géométrique pur, sans dépendance layout (frus-core ne connaît pas `Justify`).

- **Ancrage = leviers flex existants.** La boîte reste une **ligne flex** (axe
  principal horizontal → `justify` ; axe croisé vertical → `align`). `Container`
  traduit `alignment.horizontal()` → `Justify` et `alignment.vertical()` → `Align`
  dans `style()`. Aucune primitive de positionnement nouvelle : on réutilise taffy.
  Comme `style()` est la source partagée par `build_layout` **et** l'empreinte du
  cache de relayout (`layout_hash` couvre `justify`/`align`), le cache reste
  cohérent gratuitement.

- **`decoration(BoxDecoration)` = décomposition.** Le builder éclate le
  `BoxDecoration` composite dans les champs existants du conteneur (fond, dégradé,
  bordure, rayon, ombre). Zéro nouvel état, zéro nouveau chemin de paint : les
  animations (couleur/rayon…) restent applicables par-dessus. Seul le `spread`
  d'ombre n'est pas conservé — le modèle d'ombre du conteneur n'en a pas (déjà le
  cas de `.shadow`).

## Implémentation

- `frus-core/geometry.rs` : `enum Alignment` (9 variantes, `Default = Center`),
  `enum AlignEdge`, `Alignment::{horizontal, vertical}`. Ré-exportés par `lib.rs`.
- `frus-widgets/container.rs` : champ `alignment: Option<Alignment>` ; builders
  `.alignment(Alignment)` et `.decoration(BoxDecoration)` ; `style()` mappe
  l'ancrage vers `Justify`/`Align`. Imports `Align`, `Justify` de frus-layout.

## Tests

- `alignment_centers_the_child` : enfant 20×20 dans 100×100 → fond à ~(40, 40).
- `alignment_anchors_child_to_a_corner` : `BottomRight` → fond à ~(80, 80).
- `decoration_applies_composite_fields` : `BoxDecoration` (vert + rayon 8 + bordure
  2) → fond vert de rayon 8 peint, bordure réservée au layout (padding = 2).
- Suites vertes : frus-core 85, frus-widgets 196.

## Reste

- `Alignment` **fractionnel** (`{ x, y }` continu) + `Lerp` → `Tween<Alignment>`
  animable (exige un placement manuel de l'enfant, hors flex discret).
- Exposer `.alignment` / `.decoration` sur le widget nommé `AnimatedContainer`.
