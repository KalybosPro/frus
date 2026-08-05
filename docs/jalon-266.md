# Jalon 266 — Fill-then-scroll : défilement vertical par colonne **sans hauteur explicite**

## Objectif

Remplacer le **stopgap** du jalon 264 (l'app calcule et passe une hauteur de colonne via
`Kanban::card_area_height`) par un vrai **remplissage** façon Flutter (`Expanded` + `ListView`) : la
colonne prend la hauteur disponible du board, puis ses cartes **défilent**. L'app n'a plus de hauteur
à calculer.

## Cause racine du blocage (jalon 263), enfin élucidée

Un `Scroll` est un **nœud feuille** taffy (crates/frus-widgets/src/ui.rs, `build_layout` :
`scroll_content().is_some()` → `layout.leaf(...)`) : son contenu est mis en page **à part**. En mise
en page principale, le `Scroll` est donc une feuille **sans contenu mesuré** — sa base flex vaut 0. Or
`Scroll::new()` posait `height: Length(200)` par défaut : en mode `flex(1)` **sans hauteur explicite**,
cette hauteur restait une **base flexible de 200 px** — le viewport ne « remplissait » pas, il exigeait
200 px de libre pour grandir. Et surtout : `flex_grow` ne distribue de l'espace **que si le parent
direct a une taille d'axe principal définie**. Dès qu'un maillon de la chaîne (colonne, rangée,
`Container` englobant) était en hauteur `Auto` (donc calée sur son contenu), il n'y avait **aucun
espace libre** à distribuer → le `Scroll` s'effondrait à 0. Ce n'était **pas** une limite du moteur :
c'est la contrainte de Flutter aussi (un `Expanded` n'a de sens que dans un `Flex` à extent borné).

Preuve empirique (tests jetables, puis convertis en garde-fou) : une chaîne **entièrement à hauteur
définie et remplissante** donne au `Scroll` `flex(1)` un viewport égal au reste (ex. 300 − titre − pied
= 260) qui **défile** le débordement (`max_y` > 0). Un `Container` à hauteur `Auto` intercalé le
**recasse** (viewport 0).

## Correctifs

- **`frus-widgets/src/scroll.rs`** — la primitive : `Scroll` retient si sa taille a été **fixée**
  (`width_explicit` / `height_explicit`). En mode `flex` (`flex_grow > 0`), une dimension d'**axe de
  défilement non fixée** passe à `Auto` (base 0) au lieu de la valeur par défaut, pour que `flex_grow`
  **remplisse** au lieu de réserver 200. (`.width()` / `.height()` marquent la dimension comme fixée.)
- **`frus-widgets/src/kanban.rs`** — `Kanban::scrollable_columns()` (nouveau) active le mode
  remplissage : la `Row` prend `height: Percent(1.0)` (l'ancêtre est à hauteur définie) et **étire**
  ses colonnes (`Align::Stretch`) ; la zone de cartes de chaque colonne devient un `Scroll` vertical
  `flex(1)` (base 0) — titre fixe au-dessus, bouton « + Add card » fixe en dessous. Prime sur
  `card_area_height` (jalon 264), conservé comme repli quand aucun ancêtre n'est à hauteur définie. Le
  **mode par défaut est inchangé** (cartes nues, colonnes calées en haut) : le golden Kanban ne bouge
  pas.
- **`frus-demo/src/lib.rs`** (`board_screen`) — passe à `.scrollable_columns()` (plus de calcul de
  hauteur). La marge visuelle vient d'un **`Flex` `flex(1)` + padding** enveloppant le `Scroll`
  horizontal (et non plus d'un `Container` : un box à hauteur `Auto` **casserait** la chaîne — un
  `Flex` `flex(1)` remplit, lui, la hauteur définie de l'écran).

## Vérification

- **Desktop** : compile ; widgets **396** (dont le garde-fou
  `scrollable_columns_fill_the_board_height_then_scroll` : le `Scroll` d'une colonne remplit la hauteur
  du board — viewport > 300, bien au-delà du défaut 200 — **et** défile, `max_y` > 0) ; goldens **77**
  **inchangés** (mode par défaut préservé) ; shell **27**.
- **Appareil** : à confirmer au doigt (colonnes pleine hauteur atteignant le bas du board ; chaque
  colonne défile ses cartes ; glisser toujours opérant).

## Reste

- Éventuellement, faire **remplir** le contenu d'un `Scroll` sur l'axe **contraint** (façon Flutter,
  contrainte croisée serrée) directement dans `compute_scroll`, ce qui éviterait d'avoir à envelopper
  dans un `Flex` `flex(1)` côté app. Non nécessaire pour l'instant.
