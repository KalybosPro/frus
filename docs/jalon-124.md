# Jalon 124 — `FittedBox` + `RotatedBox` : transformations qui affectent la mise en page

## Analyse

`Transform` (J112–J117) transforme **à la peinture** seulement : la boîte ne bouge
pas. Il manquait les deux transformations de Flutter qui **participent à la mise en
page** :

- **`RotatedBox(quarterTurns)`** — tourne l'enfant d'un quart de tour entier **et**
  échange largeur/hauteur de la boîte pour un quart impair (un libellé vertical dans
  une barre latérale, une étiquette d'axe tournée).
- **`FittedBox(fit)`** — met l'enfant à l'échelle pour l'**ajuster** à la boîte selon un
  [`BoxFit`] (`Contain`/`Cover`/`Fill`/…), l'échelle découlant de la taille de la boîte.

## Décisions techniques

- **Feuilles de layout, enfant posé à part.** Comme `Scroll` / `InteractiveViewer`,
  les deux sont des **feuilles** dans l'arbre taffy : l'enfant est mesuré et rendu à
  part, sinon il serait étiré à la boîte au lieu d'être pris à sa taille **naturelle**.
  `build_layout` calcule la boîte du `RotatedBox` à partir de la taille naturelle de
  l'enfant (échangée pour un quart impair) ; `FittedBox` prend sa propre boîte
  (`width`/`height`/`flex`).

- **Un facteur commun de rendu.** `RotatedBox`, `FittedBox` et `InteractiveViewer`
  partagent désormais `emit_transformed_child` : peindre l'enfant à plat, l'envelopper
  dans un calque composité transformé par `M`, poser `M⁻¹` sur les hits, et — si `M`
  reste **alignée sur les axes** (échelle/translation, mais pas rotation d'un quart
  impair) — transformer les bornes de focus / défilement / glisser / accessibilité.

- **Math d'ajustement dans `frus-core`.** `BoxFit::scale(src, dst) -> (sx, sy)` (pure,
  testée) : `Fill` par axe, tous les autres uniformes (aspect conservé), `ScaleDown` ne
  réduit jamais au-delà de 1, source dégénérée → neutre.

- **Taille naturelle réutilisable.** `natural_size` met en page un sous-arbre sous des
  axes libres (`MaxContent`) — la brique commune pour la boîte du `RotatedBox` (layout)
  et le facteur du `FittedBox` (rendu).

- **Cohérence du cache.** `layout_signature` (empreinte de relayout) suit `build_layout`
  à la lettre : `RotatedBox` **hache son enfant** (sa boîte en dépend) ; `FittedBox` et
  `InteractiveViewer` sont des feuilles (empreinte = style). Au passage, `interactive()`
  manquait dans cette liste (introduit en J122) — corrigé.

## Implémentation

- `frus-core` : `BoxFit::scale` (+ test `scale_fits_content_per_mode`).
- `frus-widgets` : modules `fittedbox` (`FittedBox<Msg>`) et `rotatedbox`
  (`RotatedBox<Msg>`) ; méthodes `Widget::fitted()` / `rotated_quarter_turns()`
  forwardées (`Box<dyn>`, `Keyed`, `Responsive`, animés) ; `build_layout` (feuille
  custom pour la rotation, feuille pour l'ajustement) + `natural_size` ; branches de
  marche via `emit_transformed_child` (la branche `InteractiveViewer` refactorée pour
  l'employer) ; garde `plain_subtree_len` ; `layout_signature` aligné.

## Tests

- `frus-core` : `BoxFit::scale` par mode.
- `rotatedbox` : un quart **échange** la boîte (le frère suit à `y=80`) ; un demi-tour
  la **conserve** ; un calque tourné est émis.
- `fittedbox` : `Fill` porte l'échelle **par axe** ; `Contain` **conserve l'aspect**
  (facteur uniforme).
- Rendu visuel (hors commit) confirmé : rotation 1/2/3, ajustement Contain/Cover/Fill,
  **aucun chevauchement** des frères. Workspace complet vert : frus-widgets 227,
  frus-core 91.

## Reste

- **Rotation d'angle libre** affectant la mise en page (au-delà des quarts) et
  contraintes tournées exactes (le v1 mesure l'enfant en **non contraint**).
- Vitrine : une tuile `RotatedBox` (texte vertical) + `FittedBox` dans `frus-transforms`.
