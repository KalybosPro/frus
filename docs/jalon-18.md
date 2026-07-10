# Jalon 18 — Navigation (pile d'écrans + transitions glissées)

Ajoute une navigation multi-écrans avec transition animée au push/pop.

## Mécanisme

Le [`Navigator`] affiche un **écran plein-fenêtre**. Pendant une transition, il
rend **deux** écrans (sortant + entrant) décalés horizontalement selon un
avancement `0 → 1`.

- `Widget::navigator() -> Option<(progression, push?)>` ;
- `build_layout` : un navigateur est une **feuille** (ses écrans sont mis en page
  à part, plein-fenêtre) ;
- `build_ui` : pour chaque écran, sous-layout à la taille fenêtre puis rendu
  décalé (`render_screen`). Décalages :
  - **push** : entrant depuis la droite (`x = (1−p)·w`), sortant vers la gauche
    (`x = −p·w`) ;
  - **pop** : sens inverse.

Le `Navigator` est **contrôlé** : l'application tient la pile de routes et
l'avancement, et reconstruit les écrans à chaque frame.

## API

```rust
Navigator::new(current_screen, w, h)                    // pas de transition
Navigator::new(current, w, h).from(previous, p, forward) // transition en cours
```

## Boucle (shell)

```
Msg::Push(route) → nav_from = écran courant ; routes.push(route) ; progress = 0 ; forward
Msg::Pop         → nav_from = écran courant ; routes.pop() ; progress = 0 ; back
Redraw           → si nav_from : progress += dt/durée ; à 1 → nav_from = None
                   view = Navigator autour de l'écran courant (+ from si transition)
```

## Démo

Trois écrans : **Accueil** (avec boutons « Détails → » / « Réglages → »),
**Détails**, **Réglages** (la carte de contrôles). Chaque écran a un bouton
« ← Retour ». Les changements d'écran **glissent**.

## Tests

- `Navigator::navigator()` expose progression/direction.
- Une transition rend **les deux écrans** (sortant + entrant).

## Limites (v1)

- Transition **glissée horizontale** uniquement (pas de fondu/échelle au choix).
- Écrans reconstruits à chaque frame pendant la transition (pas de cache de rendu).
- Pas de « geste retour » (swipe) ni de barre de navigation intégrée.
