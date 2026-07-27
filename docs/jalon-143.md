# Jalon 143 — Saut de mot (Ctrl+Flèches) & bornes de champ (Ctrl+Début/Fin)

## Analyse

La navigation clavier du champ texte s'arrêtait au caractère (Gauche/Droite) et au champ
entier (Début/Fin). Manquaient les raccourcis d'éditeur attendus :

- **Ctrl+Gauche / Ctrl+Droite** : sauter d'un **mot** à la fois.
- **Ctrl+Début / Ctrl+Fin** : aller au **début / à la fin du champ** entier — et, corollaire
  en multi-lignes, **Début/Fin simples** devraient viser la **ligne courante**, pas tout le
  champ.

## Décisions techniques

- **Le modificateur voyage avec la touche.** Plutôt que d'ajouter des variantes de `Key`,
  on enrichit les existantes : `Key::Left/Right { shift, word }` et
  `Key::Home/End { shift, doc }`. Le shell remplit `word`/`doc` depuis `self.ctrl` (déjà
  suivi via `ModifiersChanged`). Le widget reste seul juge du **sens** de ces drapeaux.

- **Frontières de mot façon éditeur.** Un caractère « de mot » = alphanumérique ou `_`.
  À gauche on saute d'abord les séparateurs puis le mot (arrêt **au début** du mot
  précédent) ; à droite, séparateurs puis mot (arrêt **après** le mot suivant). Deux
  helpers purs sur `&[char]`, indices en caractères comme le reste de l'édition.

- **Début/Fin deviennent relatifs à la ligne.** `line_start`/`line_end` scannent le `\n`
  encadrant le curseur. En champ **mono-ligne**, bornes de ligne = bornes du champ : le
  comportement antérieur est préservé sans cas particulier. `doc` (Ctrl) court-circuite
  vers `0` / `len`.

- **Sélection au Shift inchangée.** Tous ces déplacements passent par `move_cursor`, donc
  `Shift` étend la sélection (saut de mot / bond de ligne sélectionnent), sans code en
  plus.

## Implémentation

- `interaction.rs` : `Key::Left/Right` gagnent `word`, `Home/End` gagnent `doc`.
- `textinput.rs` : helpers `is_word`, `word_boundary_left/right`, `line_start/line_end` ;
  branches `on_edit` correspondantes.
- `app.rs` : mappe Ctrl → `word`/`doc` en construisant les `Key`.

## Vérification

- **Unitaire** : `"foo bar baz"` — Ctrl+Left s'arrête au début de chaque mot, Ctrl+Right
  après chaque mot ; `"ab\ncd\nef"` — Début/Fin simples bornent la **2e ligne** (3 / 5),
  Ctrl+Début/Fin bornent le **champ** (0 / 8). Les tests Shift+Flèche et Home/End existants
  restent verts après l'ajout des drapeaux.
- **Non-régression** : `cargo test --workspace` vert, aucun golden déplacé.

## Reste

- **Ctrl+Retour arrière / Ctrl+Suppr** : effacer le mot précédent / suivant (réutiliserait
  `word_boundary_*`).
- **Double/triple-clic** : le mot est déjà sélectionné au double-clic (shell) ; un
  triple-clic pour la ligne resterait à faire.
