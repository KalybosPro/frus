# Jalon 9 — Scroll vertical + clipping

Ajoute le découpage (clipping) et une zone défilable.

## Ce qui est livré

- **Clipping par primitive** : chaque `Primitive` porte un `clip: Rect` ;
  `Scene::set_clip` fixe la découpe courante. Les rectangles sont découpés dans
  le **fragment shader** (rejet hors clip) ; le texte via `TextArea.bounds` de
  glyphon. Réutilisable partout (menus, cartes, viewports).
- **`Scroll`** : conteneur à défilement vertical (viewport de taille fixe). Son
  contenu est mis en page **à hauteur libre** (`Layout::compute_unbounded_height`)
  puis découpé au viewport et translaté selon l'offset.
- **Offset de scroll** : état runtime, **clé par `WidgetId`**, mis à jour à la
  molette et borné à `[0, contenu − viewport]`.
- **Pilote récursif** : `build_ui` porte un contexte `(translation, clip)` ;
  `Ui::scroll_hit(point)` renvoie la zone défilable et son offset max.

## Architecture

```
build_ui parcourt l'arbre avec un contexte { translation, clip } :
  - widget normal : peint à (rect + translation), clip courant
  - Scroll (feuille du layout principal) :
        sous-layout du contenu à hauteur libre
        translation += (0, −offset) ; clip = viewport
        enregistre (id, viewport, hauteur_max) pour la molette
Scene : chaque primitive porte son clip → GPU (shader / bounds glyphon)
```

Le contenu d'un `Scroll` est **exclu de la passe de layout principale** (le
`Scroll` y est une feuille) et mis en page dans une passe dédiée à hauteur libre,
ce qui évite que `flex-shrink` n'écrase un contenu plus grand que le viewport.

## Décisions

- **Clip par primitive** (shader + bounds texte) plutôt que `set_scissor_rect` :
  compatible avec notre dessin en un seul batch et avec le texte.
- **Offset runtime clé par identité** (comme le focus) plutôt que dans l'état
  applicatif : ce n'est pas de la donnée métier.
- **Sous-layout à hauteur libre** pour obtenir la hauteur naturelle du contenu.

## Démo

Une **liste défilante** d'éléments (plus haute que son viewport) : la molette la
fait défiler, les éléments sont **découpés** aux bords ; le bouton ajoute des
éléments à la liste.

## Tests

- `frus-core` : `set_clip` attache le bon clip aux primitives.
- `frus-gpu` : rendu offscreen — un rect dont le `clip` exclut le centre laisse
  le centre au fond (le shader découpe).
- `frus-widgets` : le contenu d'un `Scroll` est translaté de l'offset (y attendu)
  et son clip = viewport ; l'offset max = contenu − viewport.

## Limites (prochains jalons)

- **Vertical seulement**, pas de barre de défilement visible, pas d'inertie.
- Clip **rectangulaire** (pas d'arrondi de clip).
- Le contenu est entièrement peint puis découpé au GPU (pas de culling des
  éléments hors-champ).
