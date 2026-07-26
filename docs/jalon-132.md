# Jalon 132 — Champ de formulaire décoré (label, indice, aide, erreur)

## Analyse

Les trois plateformes existent (bureau, Android, Web) ; il est temps de revenir au
**produit**. La première brique d'une vraie application, c'est le **formulaire** — or le
champ `TextInput` était nu : une valeur, `on_input`, `on_submit`, et rien d'autre. Pas
de label, pas d'indice, pas de texte d'aide, et surtout **aucun état d'erreur**. Un
formulaire réel a besoin d'annoncer chaque champ et de signaler visuellement une saisie
invalide.

Flutter résout cela avec `TextField` + `InputDecoration` : un **seul** widget porte le
label, le *hint*, le texte d'aide et l'erreur. On adopte la même forme.

## Décisions techniques

- **Un seul widget, décoré — pas un `FormField` séparé.** On enrichit `TextInput` plutôt
  que d'empiler un widget parent, exactement comme `InputDecoration` de Flutter. Toute la
  logique d'édition (curseur, sélection, IME, défilement horizontal) est **réutilisée
  telle quelle** ; seule la mise en page ajoute une ligne de label au-dessus et une ligne
  d'aide/erreur en dessous.

- **La boîte de saisie est un sous-rectangle.** `style()` réserve désormais
  `label_block + field_height + sub_block` en hauteur ; `paint()` calcule la boîte
  `field` (inset vertical) et y ancre bordure, texte, caret et sélection. La boîte occupe
  toute la **largeur** — donc le hit-test horizontal (`cursor_at`, mono-ligne) est
  inchangé : le clic pour placer le curseur reste exact, sans une ligne de code touchée.

- **La validité appartient à l'application.** Le champ n'évalue rien : il **affiche** le
  résultat. `error(msg)` bascule bordure + label en couleur d'erreur du thème et affiche
  `msg` sous le champ (l'erreur masque l'aide). En architecture Elm, l'app calcule
  l'erreur comme fonction pure de son état et la passe à la `view` — pas de `GlobalKey`
  ni de `FormState` mutable à la Flutter.

- **Personnalisable, tokens du thème.** Les couleurs viennent du `Theme` (`error`,
  `muted`, `border`, `focus`) : rien n'est codé en dur, cohérent en clair/sombre, et
  surchargeable via le thème — conforme à la ligne « customisable comme Flutter ».

- **Accessibilité.** La sémantique du champ porte le label (et l'erreur, concaténée)
  comme `label`, pour que les lecteurs d'écran annoncent « Email, Enter a valid email
  address ».

## Implémentation

- `crates/frus-widgets/src/textinput.rs` : champs `label`/`placeholder`/`helper`/`error`
  et leurs builders ; métriques `label_block`/`sub_block`/`field_height` ; `style()` et
  `paint()` étendus (label au-dessus, indice quand vide, aide/erreur en dessous, couleurs
  d'erreur) ; `semantics()` enrichi. Tests : croissance de hauteur, bordure d'erreur,
  indice affiché seulement à vide.
- `crates/frus-test/tests/goldens.rs` : golden `decorated_form` — un champ en erreur
  au-dessus d'un champ au repos (indice + aide), les deux états de la décoration figés.

## Vérification

- **Rendu à l'œil** (pratique « rendre pour voir ») : le champ Email en erreur a label,
  bordure et message rouges ; le champ Password affiche son indice discret et son texte
  d'aide. Figé comme golden `decorated_form.png`.
- **Unitaires** : `cargo test -p frus-widgets textinput` vert (dont les 3 nouveaux) ;
  aucune régression des tests d'édition existants (la boîte pleine largeur préserve le
  hit-test).
- **Workspace** : `cargo test --workspace` reste vert.

## Reste

- **Composants dérivés** : un `TextInput` multi-lignes, un champ mot de passe (masquage),
  des icônes de préfixe/suffixe (`InputDecoration.prefixIcon`).
- **Aide au formulaire** : un helper de validation groupée (valider tous les champs,
  focaliser le premier en erreur) — toujours en pur, côté app.
- **Label flottant** animé (repos dans la boîte → flotte au-dessus au focus), façon
  Material.
