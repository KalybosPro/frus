# Jalon 61 — AppBar/NavBar personnalisables (degré Flutter)

Directive utilisateur (permanente) : *l'AppBar de frus est différente de celle de
Flutter — mais **tout doit être personnalisable comme chez Flutter***. Le jalon 60
avait justement **codé en dur** la graisse medium des titres d'AppBar/NavBar : le
contre-exemple exact. Ce jalon corrige et établit la règle : **les constantes
privées ne sont que des défauts, jamais la seule option**.

## `AppBar` : chaque décision a une surcharge

Le design *adaptatif* de frus (repli automatique en overflow selon la largeur)
reste — c'est le **degré de personnalisation** qui rejoint Flutter :

- **`title_style(TextStyle)`** — taille/graisse/italique/couleur du titre
  (défaut : 20 px medium, couleur du thème). La mesure du budget suit le style.
- **`title_widget(impl Widget)`** — le titre devient un **widget arbitraire**
  (logo, rangée composée…), comme le `title: Widget` de Flutter. Sa largeur
  déclarée alimente le budget de repli.
- **`action_widget(impl Widget)`** — une action **widget libre** (badge, avatar,
  champ…), insérée dans l'ordre. Toujours **en ligne** : un widget arbitraire ne
  peut pas se replier en ligne de menu texte — les actions *libellées* restent les
  seules repliables (c'est le contrat du design adaptatif).
- **`action_size(f32)`**, **`gap(f32)`** — tailles/espacements (défauts 16/8).
- **`background(Color)`**, **`height(f32)`** — chrome optionnel (défaut : rangée
  nue, le parent décide).
- `leading` était déjà un slot widget libre.

L'algorithme de repli est généralisé : les largeurs des widgets libres sont
comptées d'office (toujours en ligne), les actions libellées se replient en
préfixe, dans l'ordre. **Comportement par défaut inchangé** — les deux tests
existants de repli passent tels quels.

## `NavBar` : idem

- **`title_style(TextStyle)`** (défaut : 20 px medium ; la couleur du style
  l'emporte sur celle du thème si précisée).
- **`height(f32)`** (défaut : 56 px).

## Règle établie (mémoire projet)

Chaque décision visuelle/structurelle d'un widget doit avoir un chemin de
surcharge : builders avec défauts thémés, slots `impl Widget` là où Flutter en
offre. À appliquer aux widgets existants au fil de l'eau et à tout nouveau widget.

## Validation

- `frus-widgets` : **136 tests** (+4 : style de titre surchargé, titre-widget
  remplaçant le texte, action-widget jamais repliée même à l'étroit, NavBar
  style+hauteur surchargés).
- Comportement par défaut inchangé : tests de repli existants verts, démo 15,
  total 15 suites vertes. Build sans avertissement.

## Suite

- Propager le même audit de personnalisation aux autres widgets composés
  (Scaffold, NavRail/BottomBar, Drawer, BottomSheet…), au fil de l'eau.
- Reprendre le fil typographique : `TextSpan` + `TextLayout` (cosmic-text).
