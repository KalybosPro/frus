# Jalon 49 — Feuille modale (`BottomSheet`)

Une **feuille modale** qui glisse depuis le bas de la fenêtre — le pendant
horizontal du tiroir, pour un lot d'actions contextuelles ou un formulaire court
sans quitter l'écran courant. Elle réutilise **toute** la machinerie du tiroir :
overlay + voile, progression pilotée par le runtime, arrivée en courbe de ressort
— **zéro câblage d'animation côté application**.

## `Placement::Bottom`

Cinquième variante d'overlay (après `Below`, `Center`, `Tooltip`, `Left`,
`Right`). Traitée dans `process_overlays` (`ui.rs`) :

- **Axes** : largeur contrainte à la fenêtre (`free_x = false` — le panneau
  `Percent(1.0)` en largeur se déploie), hauteur naturelle (`free_y = true`).
- **Position** : glisse depuis le bas ; le bord bas reste collé à la fenêtre —
  `y = hauteur_fenêtre − progress · hauteur_feuille`, `x = 0`.
- **Courbe** : `spring_ease` appliquée à la progression (comme `Left`/`Right`) →
  décélération douce, sans dépassement.
- **Voile** : voile sombre plein-écran modulé par la progression (fondu
  synchronisé avec le glissement).

## `BottomSheet`

Même patron que `Drawer` (mode modal uniquement) :

```rust
BottomSheet::new(app.sheet_open)
    .on_dismiss(Msg::CloseSheet)
    .sheet(actions_column) // contenu de la feuille
    .body(main_screen)     // fond, toujours visible
```

- `SheetPanel` interne : pleine-largeur, **hauteur naturelle** (le contenu fixe
  la hauteur), fond `surface`, liseré haut + **poignée** (« grabber ») arrondie
  centrée, marge haute de 20 px pour laisser respirer la poignée.
- `overlay()` renvoie le panneau en `Placement::Bottom` ; `anim_target()` suit
  `open` (`0↔1`) — c'est la progression animée qui décide de l'affichage et du
  glissement (jalons 46/48).

## Démo

Bouton « ⋯ » dans l'en-tête → ouvre une feuille d'actions rapides (Save / Clear
completed / Close). Toute action referme la feuille ; le voile aussi. La feuille
bloque le geste de retour (`can_go_back`), comme le tiroir et les modales.

## Tests

- `frus-widgets` : `anim_target` reflète l'ouverture, placement `Bottom`, pas de
  voile fermée, voile + panneau pleine-largeur accosté au bas à l'ouverture,
  glissement à mi-animation dérivé de `spring_ease(0.5)·hauteur`.
- `frus-demo` : bascule de la feuille + fermeture sur action (`Save`,
  `AskClearDone`).

## Limites (v1)

- Pas de redimensionnement au glissement (drag-to-resize / drag-to-dismiss) : la
  poignée est décorative. Ouverture/fermeture uniquement programmatique + voile.
- Hauteur naturelle non plafonnée : un contenu très haut peut dépasser la
  fenêtre (pas de scroll interne automatique).
