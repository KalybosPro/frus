# Jalon 111 — `FractionallySizedBox` : taille en fraction du parent

## Analyse

Deuxième widget de disposition manquant : **`FractionallySizedBox`**. Il permet
de dire « prends la moitié de la largeur disponible » ou « le quart de la
hauteur » sans connaître la taille absolue du parent — indispensable pour des
mises en page fluides (une barre à 70 %, un panneau à 30 %…). Flutter le fait via
`FractionallySizedBox(widthFactor:, heightFactor:)`.

## Décisions techniques

- **Réutilise `Dimension::Percent`** (déjà présent dans `Style`) : aucune
  nouveauté dans `frus-layout`. Un facteur réglé → `Percent(f)` sur l'axe ; un axe
  non réglé → `Auto` (il suit le contenu). Brique fine.

- **La boîte se dimensionne elle-même**, plutôt que de contraindre l'enfant. Dans
  notre modèle **flex** (et non à contraintes descendantes comme Flutter),
  poser sa propre taille en pourcentage du parent donne le même résultat visuel
  dans le cas courant (enfant qui remplit). L'enfant remplit ensuite la boîte
  (étirement sur l'axe croisé, `flex` sur l'axe principal).

- **Facteurs bornés à `>= 0`.** `width_factor` / `height_factor` indépendants ;
  l'un peut être réglé sans l'autre.

## Implémentation

- `frus-widgets/fractional.rs` : le widget `FractionallySizedBox`
  (`width_factor`, `height_factor`, `child`, `style()` mappe chaque facteur vers
  `Percent` ou `Auto`).
- Export `FractionallySizedBox` dans `lib.rs`. Aucun changement dans
  `frus-layout` (le champ `Percent` existait déjà).

## Tests

- `width_factor_takes_a_fraction_of_the_parent` : `width_factor(0.5)` dans une
  colonne large de 100 → l'enfant qui remplit fait 50 de large.
- `height_factor_takes_a_fraction_of_the_parent` : `height_factor(0.25)` dans une
  colonne haute de 200 → boîte de 50 de haut.
- Suite frus-widgets verte (202) ; workspace complet vert.

## Reste

- `Transform` (rotation / échelle / translation d'un enfant) — dernier widget de
  disposition de cette série, plus lourd (matrice de scène / peinture).
- Un `alignment` sur `FractionallySizedBox` (positionner la boîte fractionnaire
  dans l'espace restant) — réutiliserait la machinerie d'ancrage (J106–J108).
