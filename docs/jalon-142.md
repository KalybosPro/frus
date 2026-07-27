# Jalon 142 — Colonne cible mémorisée + Page préc./suiv.

## Analyse

Deux limites de la navigation clavier verticale (jalon 141) restaient à lever :

1. **Colonne « qui saute ».** À chaque Haut/Bas on repartait de la colonne *courante*.
   En traversant une ligne plus courte, le caret se collait à la fin de cette ligne,
   puis redescendait depuis cette fin — la colonne d'origine était perdue. Les éditeurs
   mémorisent au contraire une **colonne cible** (« magic column ») : on la garde tant
   qu'on ne fait que monter/descendre.
2. **Pas de saut de page.** PgUp/PgDn ne faisaient rien dans un champ multi-lignes.

## Décisions techniques

- **Colonne cible portée par le shell.** Un champ `TextInput` n'a pas d'état retenu ;
  c'est le shell qui garde `goal_x: Option<f32>`. `Widget::caret_vertical` prend
  désormais cette colonne en entrée (`goal_x`) et **rend la colonne à re-mémoriser**
  pour le saut suivant — `None` = repartir de la colonne courante. Ainsi la colonne
  d'origine survit à une ligne courte : la layout repliée `hit_test` à la même `x`,
  qui, bornée à la fin d'une ligne courte, y pose le caret, mais la colonne rendue
  reste l'originale — le saut d'après la retrouve.

- **Oubli explicite.** La colonne cible est **effacée** dès qu'un autre déplacement
  survient : frappe, effacement, Gauche/Droite, Début/Fin (tout passe par `apply_key`,
  qui remet `goal_x = None`) et clic souris posant le caret. Seuls Haut/Bas/PgUp/PgDn
  la préservent.

- **Ligne vs. page dans une seule méthode.** `caret_vertical(width, cursor, down,
  page, goal_x)` unifie les deux : `page=false` avance d'**une ligne** (hauteur du
  caret) et rend `None` aux bornes (le shell **navigue le focus**, comme au jalon 141) ;
  `page=true` avance d'**une page** (hauteur visible du champ, ≥ 1 ligne) et **borne au
  champ** — aux extrémités le curseur se cale au début / à la fin et rend `Some` (on ne
  quitte jamais le champ par PgUp/PgDn).

- **Facteur commun côté shell.** Le bloc flèches et le nouveau bloc PgUp/PgDn appellent
  le même helper `App::move_caret_vertical(id, down, page)` : géométrie du champ
  (`widget_rect`), appel `caret_vertical`, sélection au Shift, mémorisation de la
  colonne, `reveal_caret`. Un seul chemin, deux entrées.

## Implémentation

- `widget.rs` (+ relais `Box`/`Keyed`/`Responsive`) : signature `caret_vertical`
  étendue (`page`, `goal_x` → `Option<(usize, f32)>`).
- `textinput.rs` : impl unifiée ligne/page avec colonne cible ; bornage au champ en mode
  page.
- `app.rs` : champ `goal_x` ; helper `move_caret_vertical` ; bloc PgUp/PgDn ; oubli de
  `goal_x` dans `apply_key` et au clic-caret.

## Vérification

- **Unitaire** : la colonne cible franchit une ligne courte (`"hello\nhi\nworld"` :
  col. 5 → "hi" bornée → retombe loin dans "world") ; PgUp/PgDn se bornent au champ et
  rendent `Some` aux extrémités ; les cas du jalon 141 (ligne simple, bornes → `None`,
  mono-ligne) restent verts avec la nouvelle signature.
- **Non-régression** : `cargo test --workspace` vert, aucun golden déplacé.

## Reste

- **Ctrl+Début/Fin** (début / fin du champ) et **Ctrl+Flèches** (saut de mot).
- La colonne cible est en **pixels** ; un futur passage en colonne « caractère » serait
  plus proche des éditeurs à chasse fixe, mais sans intérêt en police proportionnelle.
