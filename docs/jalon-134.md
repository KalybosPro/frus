# Jalon 134 — Label flottant animé (façon Material)

## Analyse

Depuis le jalon 132, le label était **statique**, toujours au-dessus du champ. Material
(et Flutter par défaut, `floatingLabelBehavior: auto`) fait mieux : au repos, le label
occupe la boîte comme un indice ; dès qu'on focalise ou qu'on saisit, il **flotte** vers
le haut en se réduisant. Un seul élément joue label *et* indice, et la transition guide
l'œil.

## Décisions techniques

- **Réutiliser l'animation de focus, ne pas en créer.** La bordure animait déjà sur
  `status.focus_progress` (interpolé 0→1 par le runtime). Le flottement s'y adosse :
  aucune nouvelle plomberie d'animation.

- **Deux pilotes distincts, position et couleur.**
  - **Position / taille** suivent le *flottement* `t = champ rempli ? 1 : focus_progress`.
    Rempli, le label reste flotté même sans focus (le contenu occupe la boîte) ; vide, il
    suit le focus. Toutes les transitions réelles sont fluides, car `focus_progress` vaut
    déjà 1 pendant qu'on édite.
  - **Couleur** suit la *focalisation* (`focus_progress`) seule : un champ rempli mais
    non focalisé garde un label **discret** (pas encore accentué) — accentué uniquement au
    focus. (En erreur, la couleur d'erreur prime à tout instant.)

- **La hauteur ne bouge pas.** `style()` réserve toujours la bande du label au-dessus de
  la boîte ; seul le **dessin** du label interpole entre sa position de repos (dans la
  boîte, à la taille du texte) et sa position flottée (au-dessus, réduite). La boîte, elle,
  reste fixe — pas de saut de mise en page.

- **L'indice cède la place au label au repos.** Quand un label est présent, l'indice
  (`placeholder`) ne se **révèle en fondu** que lorsque le label a flotté (`α = opacité ×
  focus_progress`) : sinon les deux se chevaucheraient dans la boîte. Sans label, l'indice
  s'affiche comme avant.

## Implémentation

- `crates/frus-widgets/src/textinput.rs` : `paint()` — le label interpole repos↔flotté
  (position, taille, couleur) selon `float_t` / `fp` ; l'indice fond avec `fp` en présence
  d'un label. Test `floating_label_rests_in_box_then_floats_up` (grand/bas au repos →
  petit/haut focalisé).
- `crates/frus-test/tests/goldens/decorated_form.png` : régénéré — le champ Password
  (vide, non focalisé) montre désormais son label **au repos dans la boîte** (le golden
  `password_field`, lui, est inchangé : rempli ⇒ label déjà flotté, discret).

## Vérification

- **Rendu à l'œil** : au repos, « Password » occupe la boîte ; « Email » (rempli) reste
  flotté au-dessus. Figé dans le golden `decorated_form` régénéré.
- **Unitaire** : le label est plus grand et plus bas au repos, plus petit et plus haut une
  fois focalisé.
- **Suites** : `frus-widgets` (238) + `frus-test` verts, `password_field` inchangé.

## Reste

- **Encoche du label** (le label flotté « coupe » la bordure, façon `OutlineInputBorder`
  de Material) — raffinement visuel.
- `floatingLabelBehavior: always/never` explicite, si un design veut forcer l'état.
- **Validation groupée** (jalon suivant).
