# Jalon 137 — Champ multi-lignes

## Analyse

Le champ de saisie était mono-ligne : Entrée soumettait, la boîte faisait une ligne, le
hit-test était 1D. Un vrai formulaire a besoin d'un champ **multi-lignes** (message,
commentaire, notes). Bonne nouvelle : la couche texte est déjà multi-lignes —
`TextLayout` shape les `\n`, et `caret_rect`/`hit_test`/`selection_rects` sont **2D** (le
`y` choisit la ligne). Le travail est donc surtout dans le widget et le routage du clic.

## Décisions techniques

- **Un mode sur `TextInput`, pas un widget séparé.** `multiline()` / `rows(n)` réutilisent
  toute la logique d'édition, de décoration et de rendu, comme Flutter (`TextField`
  `maxLines`). En multi-lignes : Entrée **insère un `\n`** (au lieu de soumettre), la boîte
  fait `rows` lignes, et le dessin — déjà écrit en 2D (`text_top + r.y`) — affiche
  naturellement toutes les lignes.

- **Retours explicites d'abord, pas de repli doux.** Ce jalon gère les `\n` **explicites**
  (que la layout compte déjà correctement). Le **repli automatique** (word-wrap) a été
  écarté volontairement : il exige, dans cosmic-text, de mapper chaque ligne visuelle à sa
  plage d'octets dans le texte source (le texte par run peut omettre l'espace de coupure) —
  un indexage subtil, à faire proprement dans un jalon dédié plutôt qu'approximé ici.

- **Défilement vertical minimal, symétrique de l'horizontal.** En multi-lignes, un
  `vscroll` garde la **ligne du caret** visible, recalculé depuis le curseur exactement
  comme le défilement horizontal (`(caret.y + h − content_h).max(0)`) — donc le rendu et le
  clic partagent la même géométrie.

- **Hit-test 2D : `cursor_at` gagne `local_y`.** Placer le curseur au clic sur la bonne
  ligne exige la coordonnée verticale. La signature du trait passe à
  `cursor_at(local_x, local_y, width, scroll_cursor)` ; le champ retranche la bande du
  label et le padding, ajoute le `vscroll`, et délègue au `hit_test` 2D de la layout. En
  mono-ligne, `local_y` est sans effet (une seule ligne). Tous les relais (`Box`, `Keyed`,
  `Responsive`) et les sites du shell (placement + sélection au glissement, sondes
  « est-ce éditable ? ») sont mis à jour.

## Implémentation

- `frus-widgets/src/textinput.rs` : champs `multiline`/`rows` + builders ; `field_height`
  multi-lignes ; Entrée insère `\n` en multi-lignes ; `paint` calcule `vscroll` et dessine
  à `text_top` ; `cursor_at(local_x, local_y, …)` en 2D. Tests : saut de ligne vs
  soumission, hauteur `rows`, hit-test par ligne.
- `frus-widgets` : trait `Widget::cursor_at` + relais `Box`/`Keyed`/`Responsive`.
- `frus-shell/src/app.rs` : sites `cursor_at` passent désormais `local_y` (et `0.0` pour
  les sondes).
- `frus-test/tests/goldens.rs` : golden `multiline_field` (label flottant + 3 lignes dans
  une boîte de 4 lignes).

## Vérification

- **Rendu à l'œil** : trois lignes de texte dans une boîte haute, label flotté — golden
  `multiline_field.png`.
- **Unitaires** : Entrée insère `\n` en multi-lignes (soumet en mono-ligne) ; `rows(4)`
  réserve la hauteur ; un clic une ligne plus bas place le curseur sur la 2ᵉ ligne.
- **Non-régression** : la signature `cursor_at` étendue n'altère pas le hit-test
  mono-ligne (tests de défilement existants verts) ; `cargo test --workspace` vert.

## Reste

- **Repli automatique** (word-wrap) : `TextLayout::wrapped(max_width)` avec un indexage
  correct des lignes visuelles (octets → caractères) — le complément naturel.
- **Molette / défilement tactile** dans un champ multi-lignes plus haut que `rows`.
- Répétition d'Entrée (maintenir la touche) pour insérer plusieurs `\n` d'affilée.
