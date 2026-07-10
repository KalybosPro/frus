# Jalon 17 — Overlay / portail (menus flottants, tooltips, modales)

Ajoute une couche d'**overlay** : afficher du contenu **au-dessus** de tout, hors
du flux de layout et non découpé par les parents.

## Mécanisme

Un [`Portal`] a une **ancre** (dans le flux) et un **overlay** flottant optionnel.
Comme le `Scroll`, l'overlay est mis en page à part puis **différé** :

1. `Widget::overlay() -> Option<(&dyn Widget, Placement)>` ;
2. pendant le parcours, l'ancre (enfant 0) est peinte inline ; l'overlay (enfant 1)
   est **collecté** avec les bornes de l'ancre ;
3. **après** tout l'arbre, `build_ui` traite les overlays : sous-layout (taille
   naturelle), positionnement, rendu **par-dessus** (clip = fenêtre). Leurs zones
   cliquables sont ajoutées en dernier → elles **priment** au hit-test.

`Portal::children() = [ancre, overlay]` : `find_widget` / clavier / drag atteignent
aussi le contenu de l'overlay. Les overlays imbriqués sont gérés (boucle de
traitement).

## Placements

| `Placement` | Position | Extra |
|---|---|---|
| `Below` | sous l'ancre | menu déroulant |
| `Center` | centré fenêtre | **voile** (scrim) sombre derrière (modale) |
| `Tooltip` | au-dessus de l'ancre | affiché **seulement si l'ancre est survolée** |

Le tooltip est activé quand l'id de l'ancre (enfant 0, cliquable) est le widget
survolé (`Runtime.input.hovered`).

## API

```rust
Portal::new(anchor).overlay(content, Placement::Below)     // menu flottant
Portal::new(button).overlay(tip, Placement::Tooltip)       // tooltip au survol
Portal::new(trigger).overlay(modal, Placement::Center)     // modale + voile
```

`Dropdown` est **réécrit sur ce mécanisme** : ses options flottent désormais
au-dessus du contenu (fini le déploiement inline du Jalon 16).

## Démo

- Le `Dropdown` flotte au-dessus du reste.
- Le bouton « Retirer » porte un **tooltip** au survol.
- Un bouton « Modale » ouvre une **modale centrée** (carte + voile + bouton Fermer).

## Tests

- `Portal::overlay` renvoie le contenu si fourni.
- Un overlay `Center` dessine un **voile plein écran** + son contenu par-dessus.

## Limites (v1)

- Positionnement basique (Below/Center/Tooltip) ; pas d'auto-flip si l'overlay
  déborde de l'écran, ni d'ancrage fin (start/end/aligné).
- Le clic **hors** d'une modale ne la ferme pas (pas de scrim cliquable) — on
  ferme via le bouton dédié.
