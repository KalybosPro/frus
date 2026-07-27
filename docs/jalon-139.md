# Jalon 139 — Défilement du champ multi-lignes (molette)

## Analyse

Le champ multi-lignes (jalons 137–138) défilait **uniquement** pour suivre le caret,
recalculé dans `paint`. Impossible de **parcourir** un contenu plus haut que `rows` à la
molette : le défilement n'était pas *retenu* et le champ n'était pas connu du système de
défilement du framework. Ce jalon branche le champ sur ce système.

## Décisions techniques

- **Le champ devient une vraie région défilable.** Au lieu d'un mécanisme ad hoc, le
  champ multi-lignes s'**enregistre** (comme un `Scroll` ou une liste virtuelle) via
  `self.scrollables.push((id, viewport, 0, max_y))` quand son contenu déborde. Il hérite
  ainsi **gratuitement** de toute la machinerie existante : hit-test molette
  (`scroll_hit`), cible + inertie (`scroll_target`/`advance_scroll`), dépassement
  élastique.

- **Le défilement est retenu dans le runtime.** L'offset vit dans `runtime.scroll[id].1`
  (la même carte que les `Scroll`). `Status` gagne `scroll_y` (rempli par `full_status`
  depuis cette carte) : le `paint` du champ défile son texte de `scroll_y` (borné au
  dépassement) au lieu de le dériver du caret.

- **Le suivi du caret migre de `paint` vers le shell.** Comme `paint` ne peut pas écrire
  le runtime, c'est le shell qui, **à chaque frappe** (`apply_key` → `reveal_caret`),
  ajuste `runtime.scroll[id]` juste assez pour que le caret reste visible — via une
  fenêtre `[bas du caret visible, haut du caret visible]`. On tape → le champ recentre ;
  on molette librement sinon. Une méthode unique `Widget::text_metrics(width, cursor)`
  → `(hauteur contenu, hauteur visible, sommet caret, hauteur caret)` alimente **et**
  l'enregistrement (dépassement) **et** ce suivi.

- **Le clic reste juste.** `cursor_at` n'estime plus le défilement vertical (il le
  dérivait du caret) : le shell ajoute désormais le défilement **retenu** à `local_y`
  avant l'appel, si bien que cliquer sur une ligne défilée tombe pile.

## Implémentation

- `interaction.rs` : `Status.scroll_y`.
- `ui.rs` : `full_status` remplit `scroll_y` ; le walk enregistre le champ multi-lignes
  débordant comme scrollable ; accesseur `Ui::scrollable_viewport(id)`.
- `widget.rs` (+ relais `Box`/`Keyed`/`Responsive`) : méthode `text_metrics`.
- `textinput.rs` : helper `content_width` ; `text_metrics` ; `paint` défile de
  `status.scroll_y` ; `cursor_at` sans estimation verticale (fournie par le shell).
- `app.rs` : `reveal_caret` (suivi du caret à la frappe) ; les deux sites `cursor_at`
  ajoutent le défilement retenu à `local_y`.

## Vérification

- **Rendu à l'œil** : un champ de 3 lignes défilé de ~2 lignes montre « Line three/four/
  five », clippées à la boîte — golden `multiline_scrolled`.
- **Unitaires** : `text_metrics` rapporte le dépassement et `scroll_y` remonte le texte ;
  le champ débordant s'enregistre scrollable (`max_y > 0`), un champ court non.
- **Suite complète** verte, aucun golden n'a bougé.

## Reste

- **Défilement tactile** au doigt : sur un champ, l'appui démarre une **sélection** de
  texte (pas un défilement) — geste distinct à arbitrer (barre de défilement dédiée, ou
  deux doigts). Ici : molette (et inertie) uniquement.
- **Barre de défilement** visible pour le champ (aujourd'hui : molette seule).
- **Flèches haut/bas** déplaçant le caret entre lignes dans le champ (aujourd'hui elles
  naviguent le focus) — complément clavier du multi-lignes.
