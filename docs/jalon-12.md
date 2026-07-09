# Jalon 12 — Ombres, dégradés, scroll horizontal, barre & drag, animations de focus

Un jalon large, livré en sous-étapes validées.

## A. Ombres + dégradés
- `Primitive::Rect` enrichie : `color2` + `gradient_dir` (dégradé linéaire) et
  `blur` (bord doux). `Scene::gradient_rect` et `Scene::shadow`.
- Le fragment shader mélange `color`→`color2` selon `dir`, et adoucit le bord
  sur `blur` pixels (ombre). `Container::gradient(end, dir)` et
  `Container::shadow(dx, dy, blur, color)`.

## B. Défilement horizontal
- `Scroll::axis(Axis)` (`Vertical` / `Horizontal` / `Both`) ; offset désormais
  `(x, y)`. Le contenu est mis en page libre sur le(s) axe(s) scrollable(s)
  (`Layout::compute_scroll`). Molette + **Shift** = horizontal.

## C. Barre de défilement visible + glissable
- Piste + poignée dessinées par-dessus le contenu (non découpées), taille et
  position proportionnelles. `Ui::scrollbar_at` ; le shell suit le **glissement**
  de la poignée (premier drag).

## D. Drag-sélection dans les champs
- Clic-glissé étend la sélection (`Widget::cursor_at` + ancre) ; **double-clic**
  sélectionne le mot (`Widget::word_at`). Suivi de drag générique (`Drag`).

## E. Animation de focus
- `Runtime.anims` généralisé (`Anim { hover, focus }`) ; `Runtime::advance`
  anime survol **et** focus. La bordure du `TextInput` grandit/colore en fondu au
  focus (`Status.focus_progress`).

## Décisions & infra

- **Drag** : un état `Drag` côté shell (barre de défilement | sélection texte),
  posé au `MouseDown`, appliqué au `CursorMoved`, effacé au `MouseUp`. Réutilisable.
- **Clipping** : les barres sont dessinées avec le clip *extérieur* (elles ne
  sont pas coupées par le contenu du viewport).

## Tests

- `Runtime` : `advance` (survol) et `focus_animates_independently`.
- `TextInput` : `word_at_finds_word_bounds`.
- (Les tests A–C sont validés visuellement + via le rendu offscreen existant du
  shader, non régressé.)

## Reporté (honnêtement)

- **Apparition/disparition animée** (fondu au montage/démontage) : nécessite une
  passe de reconciliation qui **retient les widgets « sortants »** le temps de la
  transition + une opacité propagée à toutes les primitives (texte compris).
  C'est un jalon à part entière ; signalé comme le plus risqué dès le départ.
- Sélection multi-lignes, inertie de scroll, ombres physiquement floues
  (gaussiennes) : hors périmètre v1.
