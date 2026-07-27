# Jalon 138 — Repli automatique du texte (word-wrap)

## Analyse

Le jalon 137 a livré le champ multi-lignes à **retours explicites** ; il restait le
**repli automatique** (une longue ligne sans `\n` qui revient d'elle-même à la largeur du
champ), écarté alors car l'indexage des caractères à travers les replis semblait dériver.
Ce jalon le résout à la source, dans la couche texte, puis le câble au champ.

## Le piège (sondé, pas supposé)

En sondant cosmic-text, on a établi précisément sa segmentation d'une ligne repliée :

- `run.text` d'un `LayoutRun` est la **ligne dure entière** (`"aaaa bbbb cccc"`) — répétée
  à l'identique pour **chaque** ligne visuelle. Compter les caractères depuis `run.text`
  attribuait donc ~19 caractères à *chaque* repli : la dérive.
- La vérité est dans les **glyphes** : run 0 couvre les octets `0..4` (`aaaa`), run 1
  `5..9` (`bbbb`), etc. L'**espace de coupure** (octet 4, 9…) est **retiré** des glyphes.
- `glyph.x` est **local à la ligne visuelle** (repart de 0 à chaque repli), et
  `glyph.start` est l'octet **relatif à la ligne dure**.

## Décisions techniques

- **Délimiter par les glyphes, indexer par les octets.** `TextLayout::wrapped(max_width)`
  remplace la boucle de `new` : chaque ligne visuelle porte le segment d'octets
  `[premier glyphe, premier glyphe du repli suivant)` de sa ligne dure (ce qui **englobe**
  l'espace de coupure, à la fin de la ligne qui précède), ses `offsets` viennent des
  `glyph.x` (déjà locaux), et son `start_char` du **décalage d'octet de la ligne dure** +
  le segment — un indexage exact, sans caractère fantôme.

- **`new` = `wrapped(None)`.** L'algorithme général traite le cas non replié à
  l'identique (une ligne dure = un run, segment = ligne entière) : aucune régression de
  tout le texte du framework (labels, boutons…), validée par la suite complète.

- **Rendu et mesure repliés par la MÊME largeur.** Le champ multi-lignes shape sa mesure
  (caret/hit-test) *et* émet son texte (`scene.text_wrapped`) avec le **même** `max_width`
  = largeur de contenu. cosmic-text produit alors les mêmes points de coupure des deux
  côtés → le caret et la sélection tombent pile sur le texte affiché.

## Implémentation

- `frus-text/src/lib.rs` : `TextLayout::wrapped(text, size, weight, italic, max_width)` ;
  `new` délègue avec `None`. Test `soft_wrap_indexes_chars_correctly_across_lines`
  (débuts de mots à x≈0 sur des lignes croissantes, aller-retour au milieu d'un repli,
  dernière frontière = nombre exact de caractères).
- `frus-widgets/src/textinput.rs` : `layout(wrap_width)` ; en multi-lignes, `paint` et
  `cursor_at` replient à la largeur de contenu, et le rendu passe par `text_wrapped`.
  Test `multiline_wraps_long_lines_to_the_width`.
- `frus-test/tests/goldens/multiline_field.png` : régénéré — une phrase longue **sans**
  `\n` repliée sur trois lignes visuelles.

## Vérification

- **Rendu à l'œil** : le message se replie doucement à la largeur du champ (golden
  `multiline_field`).
- **Unitaires** : indexage exact à travers les replis (frus-text) ; un clic sur une ligne
  repliée place le curseur plus loin dans le texte (widgets).
- **Non-régression totale** : le nouvel algorithme de layout sous-tend **tout** le texte ;
  `cargo test --workspace` reste vert, aucun golden de texte n'a bougé.

## Reste

- **Molette / défilement tactile** dans un champ multi-lignes plus haut que `rows`.
- Coupure **au sein d'un mot** très long (dépassant la largeur) : dépend de la politique
  de cosmic-text ; à vérifier si un cas réel l'exige.
