# Jalon 6 — Identité des widgets + états d'interaction

Rend l'UI réactive au pointeur : survol, pression, et clic correct (validé au
relâchement). Pose l'**identité de widget**, brique fondatrice d'une future
reconciliation.

## Ce qui est livré

- **`WidgetId`** : identité positionnelle d'un widget (hash du chemin
  racine → indices d'enfants), stable entre frames tant que la structure l'est.
- **`Interaction`** (`None`/`Hovered`/`Pressed`) transmis à `Widget::paint`.
- **`InputState`** (`hovered`/`pressed`) : état d'interaction retenu au runtime.
- **`Container`** : `hover_color` / `pressed_color` ; `paint` choisit la couleur
  selon le statut.
- **`Ui`** : `hit(point) -> Option<WidgetId>` et `msg_for(id) -> Option<Msg>` ;
  `build_ui(root, size, &InputState)`.
- **Runtime** (shell) : survol suivi au déplacement, pression au *mouse down*,
  message émis au *mouse up* **si press et release sur le même widget**.

## Position d'architecture

La reconciliation *complète* (diff d'arbres pour préserver l'état interne de
composants entre rebuilds) n'a de sens qu'avec des composants à état (saisie,
scroll, animation), qu'on n'a pas encore. Ce jalon livre donc la brique
**fondatrice** — l'identité — plus l'**état d'interaction piloté par le
pointeur**, suffisant pour survol/pression/clic. Le diff de sous-arbres viendra
avec le premier composant à état.

## Boucle runtime

```
CursorMoved → cursor ; h=ui.hit(cursor) ; si h≠hovered { hovered=h ; redraw }
MouseDown   → pressed = ui.hit(cursor) ; redraw
MouseUp     → si pressed==ui.hit(cursor) { update(state, ui.msg_for(id)) } ; pressed=None ; redraw
Redraw      → ui = build_ui(view(state), size, {hovered,pressed}) ; render
```

Statut d'un widget : `Pressed` si pressé **et** survolé, sinon `Hovered` si
survolé, sinon `None` (comportement type `:active`/`:hover`).

## Démo

Le bouton « + Ajouter un carré » s'éclaircit au survol, s'assombrit à la
pression, et n'ajoute un carré qu'au relâchement (clic réel).

## Tests

- `WidgetId` : même chemin → même id ; chemins différents → ids différents.
- `InputState::status_for` : précédence pressé > survol ; pas "Pressed" si le
  pointeur a quitté le widget.
- `Ui` : `hit`/`msg_for` routent correctement ; **le survol change la couleur
  peinte** du bouton (vérifié sur la primitive produite).

## Limites (prochains jalons)

- Pas de **focus clavier** ni de diff de sous-arbres (préservation d'état).
- Identité **positionnelle** : fragile si la structure de l'arbre change autour
  d'un widget interactif (clés explicites à envisager plus tard).
