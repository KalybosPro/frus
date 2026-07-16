# Jalon 63 — `TextLayout` : caret, hit-test et sélection sur cosmic-text

Dernière brique du fil typographique de base (§5) : la **géométrie d'édition**.
`TextInput` calculait ses positions à la main — `prefix_width` re-mesurait une
**sous-chaîne par frontière** (perdant le crénage à la coupe), et `cursor_at`
mesurait *toutes* les frontières à chaque clic (O(n²)).

## `TextLayout` (frus-text)

Le texte est shapé **une seule fois** (cosmic-text, non contraint) ; on en extrait
les offsets `x` de chaque **frontière de caractère** depuis les glyphes réels
(kerning et clusters/ligatures compris — frontières internes d'un cluster
interpolées). Indices en **caractères** (la convention d'édition de frus),
coordonnées locales au texte, multi-lignes géré (le `\n` compte un caractère).

- **`caret_rect(index)`** — position du caret à une frontière (largeur nulle : au
  widget de choisir l'épaisseur du trait) ; borné au texte.
- **`hit_test(point)`** — la frontière la **plus proche** (le `y` choisit la
  ligne, le `x` la frontière) ; c'est la sémantique du placement de curseur.
- **`selection_rects(start, end)`** — un rectangle par ligne traversée.
- **`size()`** — taille naturelle. Texte vide → ligne synthétique (caret à `x = 0`,
  hauteur de repli), jamais d'indéfini.

## `TextInput` migré

`prefix_width` supprimé. Le champ shape sa valeur **une fois** par peinture
(`self.layout()`) : défilement (caret visible), rectangles de sélection, caret et
`cursor_at` (clic → frontière) passent tous par la même géométrie — **cohérente**
(mêmes glyphes shapés) là où les mesures de préfixes pouvaient dériver du rendu au
niveau du crénage. `cursor_at` passe de O(n²) à un shape + un balayage.

Le comportement est préservé : les tests épinglés du champ
(`cursor_at_accounts_for_scroll`, `text_is_clipped_to_content_box`, et toute la
logique d'édition) passent inchangés.

## Validation

- `frus-text` : **9 tests** (+5) — offsets **monotones** dont le dernier atteint la
  largeur naturelle, **aller-retour** `caret_rect ↔ hit_test` sur chaque frontière,
  rectangles de sélection collés aux carets, mapping multi-lignes (`ab\ncd`),
  texte vide.
- `frus-widgets` **138** (TextInput migré, comportement épinglé intact),
  **231 tests** au total, tout vert ; build sans avertissement ; démo sans panique.

## Non couvert (assumé, documenté)

- **RTL/bidi** : les offsets supposent la lecture gauche→droite (comme l'ancien
  code) ; le bidi viendra avec le chantier RTL (§14).
- **Intrinsèques min/max → taffy** : reportées à l'intégration des *closures de
  mesure* dans `frus-layout` (paragraphe à retour à la ligne) — pour ne pas livrer
  d'API morte.

## Suite

Le socle §5 « texte » est complet (styles, échelle, riche, géométrie d'édition).
Candidats suivants : paragraphe à retour à la ligne (mesure sous contrainte taffy),
décorations (souligné/barré), ou retour au versant couleurs (`ColorScheme`
complète, state-layers restants).
