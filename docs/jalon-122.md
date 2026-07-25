# Jalon 122 — `InteractiveViewer` : déplacer (pan) + zoomer (pinch/molette)

## Analyse

Débouché naturel de la pile transformation (J112–J117) + découpe (J121) : une
**fenêtre interactive** où l'utilisateur **déplace** et **zoome** son enfant, façon
`InteractiveViewer` de Flutter — la brique d'une carte, d'une image détaillée, d'un
plan, d'un diagramme. Tout est déjà là (calque transformé + découpé, hit-test par
`M⁻¹`) ; il restait l'**état retenu** de la transformation et le **routage des gestes**.

## Décisions techniques

- **Un seul calque fait tout.** L'enfant remplit la fenêtre à l'échelle 1, puis est
  enveloppé dans **un** `Primitive::Layer` portant à la fois la matrice `M` (échelle +
  translation) **et** la découpe à la fenêtre — le compositing applique déjà
  l'échantillonnage `M⁻¹` et le test de clip dans la même instance. Le hit-test passe
  le point par `M⁻¹` (comme `Transform`).

- **La transformation est un état retenu, pas de l'état d'app.** `InteractiveView
  { scale, tx, ty }` vit dans le `Runtime` (comme les offsets de défilement), indexé par
  fenêtre ; absent = identité. La `view` reste une fonction pure de l'état d'app.

- **Math des gestes pure et testée.** `InteractiveView::pan` (le curseur pousse le
  contenu) et `zoom_at` (**zoom ancré au curseur** : `t' = cursor·(1−f) + f·t`, échelle
  bornée `[min, max]`) sont des fonctions pures — le shell ne fait que les appeler et
  restituer `matrix()`. Le point du contenu sous le curseur reste fixe au zoom.

- **Gestes shell, avec désambiguïsation tap/pan.** Glisser (souris **ou** doigt) →
  `Drag::Pan`, engagé seulement au-delà de `TOUCH_SLOP` : un simple tap passe alors à
  l'enfant (un bouton dans la fenêtre reste cliquable). Molette → **zoom** ancré au
  curseur (~1.1×/cran), bornes lues sur le widget. `interactive_at(point)` localise la
  fenêtre la plus au-dessus.

- **Taille bornée requise.** Comme `Scroll`, la fenêtre a besoin d'une taille
  (`width`/`height` ou `flex`) sous peine de s'effondrer — voir
  [[scroll-viewport-sizing-gotcha]].

## Implémentation

- `frus-widgets` : module `interactive` — `InteractiveView` (état + math) et
  `InteractiveViewer<Msg>` (`min_scale`/`max_scale`, `width`/`height`/`flex`) ; méthode
  `Widget::interactive()` forwardée (`Box<dyn>`, `Keyed`, `Responsive`, animés) ; champ
  `Runtime::interactive` ; branche de marche (`ui.rs`) émettant le calque transformé +
  découpé et posant `M⁻¹` sur les hits ; collecte `interactives` + `Ui::interactive_at`.
- `frus-shell` : variante `Drag::Pan` (tap/pan par seuil) ; `pointer_down` amorce le
  pan ; `handle_drag` l'applique ; `MouseWheel` zoome sur une fenêtre interactive.

## Tests

- `interactive` (unitaires, purs) : identité = neutre ; `pan` décale du delta exact ;
  `zoom_at` **garde le point sous le curseur fixe** ; zoom **borné** à `max`.
- `interactive` (marche) : le calque émis **porte la matrice et la découpe** à la
  fenêtre ; après un pan, le hit-test **suit** (ancienne position ratée, nouvelle
  atteinte) — preuve que `M⁻¹` traverse la transformation.
- Workspace complet vert : frus-widgets 221 (+6), frus-gpu 16, frus-core 90.

## Reste

- **Pincement multi-touch** (deux doigts) : le modèle d'entrée est mono-curseur ; la
  molette couvre le zoom desktop, le pinch tactile viendra avec le suivi 2 doigts.
- **Inertie** (fling au pan) et **bornage** du déplacement (boundaryMargin de Flutter).
- Vitrine : une rangée `InteractiveViewer` dans `frus-transforms`.
