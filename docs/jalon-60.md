# Jalon 60 — Typographie : `TextStyle` + `TextTheme` (graisse et italique rendus)

Suite du système de design (§5). Frus n'avait **aucun modèle typographique** : une
primitive de texte réduite à `(taille, couleur)`, des tailles en dur dans chaque
widget, ni graisse ni italique. Ce jalon pose le vocabulaire (`TextStyle`), l'échelle
nommée (`TextTheme`, 15 crans Material) et — surtout — **rend réellement** la graisse
et l'italique, de la mesure jusqu'au GPU.

## `TextStyle` (frus-core, pur, `Copy`)

`TextStyle { size, weight: FontWeight, italic, color: Option<Color> }` avec builders
`const` et **`merge`** (la cascade : les attributs typographiques de la surcouche
gagnent, la couleur **hérite** si absente — `None` = résolue contre le thème au
paint). `FontWeight` = Regular/Medium/SemiBold/Bold, mappé sur les poids OpenType.

## Le pipeline rend le style, de bout en bout

- **`Primitive::Text`** porte désormais `weight` + `italic` ; `Scene::text_styled`
  les émet (`Scene::text` reste inchangé, graisse normale).
- **`frus-gpu`** transmet graisse/italique à cosmic-text via les `Attrs` glyphon —
  la face correspondante de la famille est choisie (repli gracieux sinon).
- **`frus-text::measure_styled`** mesure le texte **stylé** — un gras est plus
  large, la mise en page doit le savoir. `measure` délègue (graisse normale).
- **Face grasse embarquée** : `DejaVuSans-Bold.ttf` rejoint les polices embarquées.
  Sans elle, `Bold` serait retombé **en silence** sur la face normale partout où
  seules les polices embarquées existent (Android) — trahissant la promesse du
  rendu déterministe. (~700 Ko ; l'oblique n'est pas embarquée : l'italique se
  replie proprement là où le système n'en fournit pas.)

## `TextTheme` : l'échelle typographique nommée

`theme.text` = les **15 crans Material 3** (`display_large 57 … label_small 11`,
crans title/label en graisse medium). Les widgets choisissent un cran
(`Text::styled("Titre", theme.text.title_large)`), pas une taille en dur. La
typographie ne participe pas au fondu de thème (identique clair/sombre).

## Adoption

- **`Text`** : builders `.weight()` / `.italic()` + constructeur `Text::styled(…)` ;
  mesure **stylée** (le layout d'un gras est correct) ; couleur du style héritée du
  thème si absente.
- **`NavBar`** et **`AppBar`** : titres en graisse **medium** (un titre de barre est
  un « title », pas un corps de texte), mesure de centrage/budget alignée.
- **Démo** : titre de dialogue en medium, message d'état vide en *italique*.

## Validation

- **Preuve que le gras est réel** : `bold_measures_wider_than_regular` (frus-text)
  échouerait si `Bold` retombait sur la face normale (largeurs égales) — c'est la
  validation de bout en bout que la face embarquée est résolue. Doublé côté widget
  par `bold_text_lays_out_wider`.
- `frus-core` **49** (+3 : builders, poids OpenType, cascade `merge`),
  `frus-widgets` **132** (+2), `frus-text` **3** (+1) ; démo/gpu/shell/layout verts —
  **214 tests** au total. Build sans avertissement ; démo lancée sans panique.

## Suite (§5 texte)

- `TextSpan` (arbre riche `{texte, style, enfants}` aplati en *runs* pour
  cosmic-text) — `merge` est déjà prêt pour sa cascade.
- `TextLayout` sur cosmic-text (intrinsèques → taffy, `hit_test`, curseur,
  sélection) — unifiera aussi la mesure et le rendu multi-runs.
- `letter_spacing`/`line_height`/décorations dans `TextStyle` quand le rendu les
  supportera ; face oblique embarquée si l'italique déterministe devient requis.
