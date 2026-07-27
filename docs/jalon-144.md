# Jalon 144 — Encoche du label (style `outlined`)

## Analyse

Le label flottant (jalon 134) montait dans un **bandeau réservé au-dessus** de la boîte.
Material propose aussi le style **`OutlineInputBorder`** : le label flottant se pose **sur**
la bordure du haut, qui s'**ouvre** d'une encoche derrière lui — plus compact, très
reconnaissable. Il manquait.

## Décisions techniques

- **Opt-in, défaut inchangé.** Un constructeur `TextInput::outlined()` active le style.
  Sans lui, le rendu « bandeau » d'origine est conservé au pixel près : les goldens
  existants ne bougent pas.

- **Encoche par aplat, pas par découpe de tracé.** La bordure est un unique
  `Primitive::Rect` (rectangle arrondi tracé sur GPU) : impossible d'y percer un trou.
  On peint donc, **après** la bordure et **sous** le texte du label, un petit aplat couleur
  `surface` qui masque le segment de bordure traversé — le label vient ensuite par-dessus.
  L'aplat n'apparaît qu'à mesure que le label monte (`fade(o * float_t)`), donc l'encoche
  « s'ouvre » avec l'animation de flottement.

- **Cible flottée sur la bordure.** La géométrie du label est interpolée entre le repos
  (dans la boîte, à la place de l'indice) et une cible **différente selon le style** :
  `outlined` → `(field.x + PAD_X, field.y − ½·hauteur_label)`, centrée sur la bordure du
  haut ; sinon → coin haut-gauche du bandeau. Un seul chemin d'interpolation, deux cibles.

- **Ordre de peinture inversé.** En bandeau, le label est peint **avant** la bordure (il
  vit au-dessus) ; en contour, **après** (il se pose dessus). La géométrie est calculée une
  fois, le `scene.text` du label est simplement émis au bon moment selon le style.

- **Réserve verticale réduite.** `label_block` ne réserve, en `outlined`, que la **moitié
  haute** du label (le reste mord sur la boîte, puisqu'il chevauche la bordure) au lieu
  d'un bandeau plein — le champ est d'autant plus compact.

## Implémentation

- `textinput.rs` : champ `outlined` + constructeur ; `label_block` (½ label en contour) ;
  `paint` — géométrie de label factorisée, cible selon le style, aplat d'encoche
  (`fill_rect` couleur surface) puis label après la bordure ; constante `NOTCH_GAP`.
- `goldens.rs` : golden `outlined_field` — un champ rempli (encoche ouverte) + un champ
  vide (label au repos, bordure intacte).

## Vérification

- **Golden** `outlined_field` rendu et **inspecté** : « Full name » se pose sur la bordure
  du haut avec une coupure nette (encoche), valeur dans la boîte ; « Email » vide garde sa
  bordure fermée, label au repos. Conforme à Material.
- **Non-régression** : tous les goldens existants (bandeau) inchangés ; `cargo test
  --workspace` vert.

## Reste

- **Bordure animée de l'encoche** : la largeur de l'encoche pourrait s'animer avec le
  flottement (ici l'aplat se **fond** plutôt qu'il ne s'ouvre en largeur).
- **Rayon de coin configurable** de la bordure `outlined` (aujourd'hui `theme.radius`).
