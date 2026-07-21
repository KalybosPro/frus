# Jalon 98 — `AnimatedContainer` : taille animée (au layout)

## Analyse

J97 a livré la couleur animée (propriété **picturale**, layout-free). La moitié
manquante d'`AnimatedContainer` est **géométrique** : animer la **taille**. C'est
plus profond, car une propriété de **disposition** doit être connue **au moment
du layout** (taffy lit `style()` *avant* la peinture), pas seulement au paint.

## Décisions techniques

- **Injection par un `effective_style` unique.** La clé : `build_layout` (qui
  construit l'arbre taffy) **et** `hash_node` (l'empreinte du cache de relayout)
  appellent tous deux `widget.style()`. On remplace ces appels par un
  [`effective_style(widget, id, runtime)`] commun : le `style()` du widget, dont
  la **taille est remplacée par la taille interpolée** du runtime si le widget est
  animé. Comme les deux chemins partagent cette source, ils restent
  **automatiquement cohérents** — et l'empreinte **change tant que la taille
  bouge**, invalidant le cache frame après frame (relayout pendant l'animation,
  puis re-cache une fois figée). Aucune divergence possible.

- **Runtime : timeline de taille.** [`SizeAnim`] `{ current, from, to, elapsed }`
  par nœud, tweenée par `advance_sizes` sur le **même modèle** que valeur/couleur
  (rebase au changement, snap au montage, courbe/durée du widget) — interpolation
  linéaire par composante (largeur/hauteur).

- **Identités alignées.** `build_layout` et `hash_node` propagent désormais l'`id`
  via `child_id`, **exactement** comme la marche de peinture — indispensable pour
  que la taille animée d'un nœud atterrisse sur le bon rectangle.

- **`Container` API** : `.animated_size(width, height, duration, curve)` ; trait
  `Widget::anim_size() -> Option<Size>` (cible) + forwarders. Opacité/couleur/
  taille d'une même boîte partagent une `(durée, courbe)`.

## Portée & limites

- Un widget mis en page **à part** (défilable/pile/navigateur/liste = feuille dans
  `build_layout`) n'anime pas sa taille par ce chemin (limite assumée) ; les
  conteneurs de flux normaux, si.
- La taille animée **défait le cache de relayout pendant l'animation** (par
  construction : la géométrie change à chaque frame) — comme Flutter. Une fois
  figée, le cache reprend.

## Implémentation

- `frus-widgets` : `Runtime` (`SizeAnim`, `sizes`, `anim_size`, `advance_sizes`) ;
  trait `anim_size()` + forwarders ; `ui::effective_style` + `build_layout`
  (id/runtime) ; `relayout` (`rects`/`compute_rects`/`layout_signature`/`hash_node`
  threadés) ; `Container.animated_size`.
- `frus-shell` : `advance_sizes` dans la boucle (avant `build_ui`, donc la taille
  est prête **au layout**).

## Tests

- `animated_size_tweens_between_frames` (runtime) : snap au montage (20×20), tween
  linéaire → 40×40 (mi-parcours ≈ 30×30), oubli du widget disparu.
- `animated_size_drives_the_layout` (bout en bout) : à mi-parcours, le **rectangle
  de fond peint** mesure ~30×30 — preuve que la taille interpolée traverse
  `runtime → effective_style → taffy → rects → paint`.
- Cache de relayout : signatures/hits inchangés sans animation (`effective_style`
  = `style()`), donc goldens et suites existantes **intacts**.

## Reste

- Padding/rayon/marge animés (même mécanique d'injection au layout).
- Widgets nommés `AnimatedContainer`/`Opacity`/`AnimatedOpacity` (sucre au-dessus
  de `Container`).
- `Tween` typés génériques ; animations pilotées explicitement (contrôleur).
