# Jalon 4 — Interactivité : événements + état

Rend l'UI **vivante** : les widgets réagissent au clic et reflètent un état qui
change, via un **modèle à messages** (façon Elm/iced).

## Ce qui est livré

- **Widgets génériques `Widget<Msg>`** : un widget peut émettre un message au
  clic (`Container::on_click(msg)`).
- **`Ui<Msg>` + `build_ui`** : la construction produit à la fois la [`Scene`] à
  dessiner **et** une carte de hit-test. `Ui::hit(point)` renvoie le message du
  widget cliquable le plus au-dessus.
- **Boucle interactive** (démo shell) : `State`, `view(&State) -> Widget<Msg>`,
  `update(&mut State, Msg)`. La fenêtre suit le curseur et route les clics.

## Architecture

```
état ──view()──► Widgets<Msg> ──build_ui──► Ui { Scene, hits }
  ▲                                             │  scene ─► frus-gpu ─► écran
  │                                             │
  └── update(msg) ◄── ui.hit(curseur) ◄── clic souris (winit)
```

Le hit-test réutilise l'appariement widget ↔ rectangle absolu du pilote : on
collecte les zones cliquables `(Rect, Msg)` en ordre préfixe ; le clic prend la
**dernière** zone contenant le point (les enfants, peints après, sont au-dessus).

## Décisions

- **Modèle à messages** plutôt que callbacks : idiomatique Rust, évite
  `Rc<RefCell>`/emprunts croisés, testable. Widgets paramétrés par `Msg: Clone`.
- **`frus-widgets` ne dépend pas de winit** : le hit-test prend un `Point` ; la
  souris est traduite côté `frus-shell`.
- **Coordonnées** : pixels physiques (le curseur winit et le viewport partagent
  le même espace).

## Démo

Une barre-bouton verte ; chaque clic ajoute un carré coloré à une rangée. Clic →
`Msg::AddSquare` → `state.squares += 1` → rebuild → un carré de plus. Prouve la
boucle événement → état → rebuild → rendu de bout en bout.

## Tests

- `build_ui` peint les bons rectangles **et** mappe les bonnes zones cliquables.
- `Ui::hit` renvoie le bon message, et le widget **le plus au-dessus** en cas de
  recouvrement.

## Limites (prochains jalons)

- États visuels **survol/pressé**, **focus**, **clavier** : nécessitent
  l'identité des widgets entre deux frames → viendront avec la **reconciliation**.
- Reconstruction complète de l'arbre à chaque interaction (pas encore de diff).
- Toujours pas de **texte**.
