# Jalon 205 — Curseur système par sous-région

## Analyse

Au survol d'une icône cliquable (le suffixe œil/✕ d'un champ, jalon 198/202), le pointeur restait
une flèche : rien ne signalait « ceci se clique ». Les navigateurs et Flutter changent le curseur en
**main** sur les zones cliquables. Il fallait donc qu'un widget puisse demander une forme de curseur
pour une **sous-région**, et que le shell la pose sur la fenêtre.

## Décisions techniques

- **Un avis local, façon `positional_click`.** Nouvelle méthode du trait :
  `Widget::cursor_icon(local_x, local_y, width, height) -> Option<Cursor>`. Elle reçoit la position
  **locale** du pointeur dans la boîte du widget et renvoie la forme souhaitée, ou `None` (pas
  d'avis → le shell garde le défaut). Purement visuel : n'affecte pas le clic.

- **Les widgets restent indépendants du fenêtrage.** Un petit enum `frus_widgets::Cursor`
  (`Default` / `Pointer` / `Text`) est l'unité d'échange ; le shell le traduit vers
  `winit::window::CursorIcon`. Aucun widget ne dépend de winit.

- **Recalcul à chaque mouvement.** Le shell (`pointer_move`) interroge le widget survolé via
  `widget_rect` + `find_widget` (même chemin que le clic positionnel) et pose le curseur à **chaque**
  déplacement — la sous-région peut changer sans que le widget survolé change (l'œil vs le corps du
  champ).

- **`TextInput`** renvoie `Pointer` sur son icône suffixe **active** (`on_suffix` posé) via le
  `suffix_hit` existant ; ailleurs, `None`. Un suffixe purement décoratif ne change pas le curseur.

## Implémentation

- `frus-widgets/src/interaction.rs` : enum `Cursor` (exporté depuis `lib.rs`).
- `frus-widgets/src/widget.rs` : méthode `cursor_icon` (défaut `None`) + forwarder `Box`.
- `frus-widgets/src/{keyed,responsive}.rs` : forwarders.
- `frus-widgets/src/textinput.rs` : override `cursor_icon` (main sur le suffixe actif).
- `frus-shell/src/app.rs` : `update_cursor_icon`, appelé par `pointer_move`, traduit vers
  `CursorIcon` et appelle `window.set_cursor`.

## Vérification

- `cursor_icon_is_pointer_over_active_suffix` (widgets) : `Pointer` sur le suffixe, `None` dans le
  corps, `None` si le suffixe est décoratif. Le reste du workspace compile et passe (forwarders).

## Reste

- **Surbrillance** de la sous-région survolée (halo sur l'icône) : demande la position locale du
  survol dans `Status` — non encore plombée. Curseur `Text` sur le corps des champs, `Pointer` sur
  boutons/liens génériques, et infobulles réutilisant ce même mécanisme de sous-région (barres/points
  des charts).
