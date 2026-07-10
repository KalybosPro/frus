# Jalon 15 — Système de thèmes

Centralise les *design tokens* (couleurs, rayon, espacement) dans un `Theme`
injecté au rendu, avec presets clair/sombre.

## Ce qui est livré

- **`Theme`** (frus-widgets) : tokens `background, surface, primary, on_primary,
  on_surface, muted, border, focus, selection, radius, spacing` ; presets
  `Theme::dark()` / `Theme::light()` (défaut = sombre).
- **Injection** : `build_ui(root, size, &runtime, &theme)` transmet le thème à
  `Widget::paint(bounds, status, &theme, scene)`.
- **Défauts themés** :
  - `Text` sans couleur explicite → `theme.on_surface` ;
  - `TextInput` : fond/bordure/focus/sélection/texte du thème ;
  - barres de défilement → `theme.muted`.
- **Surcharge** : une couleur explicite (`Container::color`, `Text::color`) reste
  prioritaire.
- **Démo** : fond racine `theme.background`, boutons `theme.primary`, et un bouton
  **« Thème clair/sombre »** qui bascule tout l'UI.

## Décisions

- Thème **lu au paint** (signature `paint` enrichie d'un `&Theme`) plutôt que
  résolu à la construction : les widgets s'adaptent sans que l'appelant réinjecte
  les couleurs.
- Le fond d'application est un `Container(theme.background)` racine (pas de
  couplage au renderer / à sa couleur d'effacement).

## Tests

- `Theme::dark` / `Theme::light` : tokens distincts.
- `Text` sans couleur peint avec la couleur du thème (via le paint themé).

## Périmètre (v1)

- `Container` garde ses couleurs **explicites** (conteneurs colorés à dessein) ;
  le thème sert aux **défauts** (texte, champ, barres) et à la démo.
- Pas encore de styles de composants nommés (Button/Checkbox…), ni de transition
  animée entre thèmes (le changement est instantané).
