# Jalon 13 — Opacité + apparition en fondu

Ajoute une opacité propagée au rendu et un **fondu d'apparition** au montage des
widgets.

## Ce qui est livré

- **`Color::fade(opacity)`** (frus-core) : multiplie le canal alpha.
- **`Anim.opacity`** (défaut **1.0**, démarrée à 0 au montage) ; `Runtime::advance`
  la fait tendre vers 1. `Runtime::opacity(id)`.
- **`Runtime.mounted: HashSet<WidgetId>`** : ensemble des widgets présents ; un id
  **nouveau** démarre à opacité 0 (donc en fondu).
- **`collect_ids(&arbre)`** : parcours léger par identité, pour diffuser le
  montage/démontage avant `build_ui`.
- **`Status.opacity`** : lu par les widgets, qui **multiplient l'alpha** de toutes
  leurs couleurs (`Container`, `Text`, `TextInput` — texte, fond, bordure,
  curseur, sélection, ombre, dégradé).

## Boucle (shell, RedrawRequested)

```
tree = view(state)
ids = collect_ids(&tree)
pour id nouveau (pas dans runtime.mounted) : mounted.insert(id) ; anims[id].opacity = 0
mounted.retain(présents)               // ré-apparition si ré-ajouté plus tard
animating = runtime.advance(dt)        // opacité + survol + focus
ui = build_ui(&tree, size, &runtime)   // opacité -> Status.opacity -> alpha
render ; si animating -> redraw
```

## Démo

Au démarrage, toute l'UI **apparaît en fondu**. Chaque **nouvel élément** ajouté à
la liste (clic sur le bouton) **entre en fondu**.

## Tests

- `Color::fade` : mise à l'échelle de l'alpha.
- `Runtime::advance` : l'opacité monte de 0 vers 1 ; défaut 1 sans entrée.

## Reporté (honnêtement)

- **Disparition (fade-out)** : un widget retiré de l'arbre n'a plus de primitives
  à dessiner. L'animer demande de **retenir les widgets sortants** (leurs
  primitives ou leur sous-arbre) le temps de la transition — une passe de
  reconciliation avec liste de « sortants », qui fera l'objet d'un jalon dédié.
