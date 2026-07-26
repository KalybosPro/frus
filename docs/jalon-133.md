# Jalon 133 — Champ mot de passe (masquage) + icônes préfixe/suffixe

## Analyse

Le jalon 132 a donné au champ sa décoration (label, indice, aide, erreur). Deux besoins
de formulaire manquaient encore : **masquer** un mot de passe, et loger une **icône**
dans le champ (recherche, cadenas, devise…). Ce sont les compléments directs de
l'`InputDecoration` : `obscureText`, `prefixIcon`, `suffixIcon` chez Flutter.

## Décisions techniques

- **Masquer l'affichage, pas la valeur.** `obscure(true)` change uniquement la chaîne
  **rendue** : `display()` renvoie un point par caractère, la vraie valeur reste dans
  `value`. Toute l'édition (insertion, sélection, IME, `text_value` pour le contexte de
  saisie) porte sur la valeur réelle. Comme le masque conserve le **nombre de
  caractères**, le caret, le hit-test et la sélection restent alignés index pour index —
  la géométrie est simplement shapée sur la chaîne masquée.

- **Le toggle « afficher » se compose côté app.** On n'ajoute pas de suffixe *interactif*
  au champ (qui exigerait un sous-hit-test à l'intérieur du widget). En architecture Elm,
  l'application possède le booléen `show`, rend `.obscure(!show)` et place à côté un
  bouton qui bascule ce booléen. Le champ reste pur et sans état caché.

- **Icônes = slots décoratifs, dessinés en place.** `prefix_icon`/`suffix_icon` prennent
  un `IconName` ; le champ peint son chemin vectoriel directement dans la boîte (comme le
  widget `Icon`), centré verticalement, en couleur discrète. Pas d'enfant, pas de widget
  imbriqué.

- **Les icônes rétrécissent la zone de contenu — partout.** `prefix_w`/`suffix_w`
  réservent `ICON_SIZE + ICON_PAD` de chaque côté. La géométrie de contenu (origine +
  largeur du texte) est recalculée avec ces insets **et dans `paint` et dans
  `cursor_at`** : un clic tombe donc au bon index même derrière un préfixe.

## Implémentation

- `crates/frus-widgets/src/textinput.rs` : champs `obscure`/`prefix`/`suffix` + builders ;
  `display()` (chaîne masquée) ; `layout()` shape l'affichage ; `prefix_w`/`suffix_w` ;
  `paint()` dessine les icônes et insère le contenu entre elles, rend `display()` ;
  `cursor_at()` applique les mêmes insets. Tests : masquage (valeur non fuitée, points
  dessinés, `text_value` intact) et préfixe (chemin dessiné + hit-test décalé).
- `crates/frus-test/tests/goldens.rs` : golden `password_field` (valeur masquée + icônes
  de préfixe et de suffixe).

## Vérification

- **Rendu à l'œil** : label, icône de préfixe, `•••••••` masqué, icône de suffixe, aide —
  figé en golden `password_field.png`.
- **Unitaires** : `cargo test -p frus-widgets textinput` vert (17 tests, dont 2 nouveaux) ;
  aucune régression du hit-test existant (sans icône, `left == PAD_X`, géométrie
  identique).
- **Suites** : `frus-widgets` + `frus-test` verts.

## Reste

- **Toggle de visibilité intégré** (suffixe interactif) : demanderait un sous-hit-test
  dans le widget — reporté ; l'app le compose aujourd'hui.
- **Icône colorable / taille réglable** par champ (aujourd'hui : discrète, `ICON_SIZE`).
- **Label flottant** animé (jalon suivant) et **validation groupée** (celui d'après).
