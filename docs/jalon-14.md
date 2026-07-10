# Jalon 14 — Disparition en fondu (rétention des sortants)

Complète les animations de cycle de vie : un widget retiré de l'arbre **sort en
fondu** au lieu de disparaître d'un coup.

## Principe

Un widget absent de l'arbre n'a plus de primitives. On le fait donc disparaître
par **instantané + rejeu** :

1. **Tag** : chaque `Primitive` porte un `owner: u64` (= `WidgetId`), posé par
   `build_ui` via `Scene::set_owner` avant de peindre chaque widget.
2. **Détection** : `mounted (N-1) − présents (N)` = ids **sortants**.
3. **Capture** : à la sortie, on copie de la dernière scène les primitives dont
   `owner` ∈ sortants → `Runtime.leaving[clé] = (primitives, 1.0)`.
4. **Rejeu** : chaque frame, `build_ui` rejoue ces primitives via
   `Scene::push_faded` avec l'opacité qui descend (`advance_leaving`, `1 → 0`),
   puis les oublie à 0.

## Ce qui est livré

- **`frus-core`** : `owner` sur les primitives ; `Scene::set_owner` ;
  `Scene::push_faded(&Primitive, opacity)` ; `Primitive::owner()`.
- **`frus-widgets`** : `WidgetId::as_u64` ; `Runtime.leaving` +
  `Runtime::advance_leaving` ; `build_ui` tague l'`owner` et rejoue les sortants.
- **`frus-shell`** : capture les sortants depuis la dernière scène, avance la
  sortie, redessine tant qu'une sortie est en cours.
- **Démo** : un bouton « − Retirer » ; l'élément retiré **sort en fondu**.

## Boucle (shell)

```
présents = collect_ids(&tree)
sortants = mounted − présents
pour chaque sortie : capture (dernière scène, owner ∈ sortants) → runtime.leaving
montages : nouveaux ids → opacité 0
advance (entrée/survol/focus) | advance_leaving (sortie)
ui = build_ui  (rejoue runtime.leaving en fondu)
render ; si animation -> redraw
```

## Tests

- `Scene::push_faded` : alpha réduite, `owner` conservé.
- `Runtime::advance_leaving` : l'opacité descend vers 0 puis l'entrée disparaît.

## Simplifications (v1)

- L'instantané est **figé** (apparence/position de la dernière frame) : pas de
  layout ni d'animation interne pendant la sortie — comportement standard d'une
  animation de sortie. Les sortants ne reçoivent plus d'événements.
