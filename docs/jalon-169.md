# Jalon 169 — Accessibilité : sélection de ligne annoncée

## Analyse

Le jalon 167 énonçait le **tri** et les **cases à cocher**, mais pas la **sélection de
ligne au clic** (`on_select_row`) — cliquer une rangée changeait l'état silencieusement pour
un utilisateur de lecteur d'écran. Il fallait l'énoncer, avec le **numéro de ligne** pour
situer l'action.

## Décisions techniques

- **La cellule connaît sa ligne.** `Cell` (donnée) et `WidgetCell` gagnent l'index de
  ligne ; leur `announce()` (le point d'accroche du jalon 167, déjà lu par le shell au clic)
  énonce l'état **résultant** : « Row N selected » / « Row N deselected » (bascule de l'état
  courant `selected`). Toute cellule de la rangée porte l'annonce — cliquer n'importe où dans
  la ligne la sélectionne, donc l'énonce.

- **La navigation de focus n'est *pas* dupliquée.** Annoncer « button, Save » au Tab serait
  **redondant** : AccessKit publie déjà le nœud **focalisé** (`focus` de l'arbre), que le
  lecteur d'écran énonce nativement. Y ajouter une région live ferait **parler deux fois**.
  On s'appuie donc sur le focus AccessKit existant — décision, pas oubli.

## Implémentation

- `table.rs` : champ `row` sur `Cell` (`Option`, `None` pour un en-tête) et `WidgetCell`
  (`usize`) ; `Cell::announce` (branche donnée) et `WidgetCell::announce` énoncent
  « Row N selected/deselected ». Renseignés à la reconstruction.

## Vérification

- **Unitaire** : `row_click_selection_is_announced` — ligne texte non sélectionnée →
  « Row 1 selected » ; ligne-widget sélectionnée → « Row 2 deselected » ; table non
  sélectionnable → aucune annonce.
- `cargo test --workspace` **vert**.

## Reste

- **Numéro vs libellé** : on énonce « Row N » ; certaines apps préféreraient le contenu de la
  ligne (« Ada selected »). L'app pourrait le fournir via une future surcharge d'annonce.
- Étendre aux **cases à cocher** le **compte** (« 3 rows selected ») plutôt que ligne à ligne.
