# Jalon 186 — DatePicker : calendrier double (longues plages)

## Analyse

Le mode plage (jalon 184) tient dans **un** mois. Or une plage franchit souvent la frontière —
« du 28 juillet au 3 août » — et l'utilisateur doit alors naviguer d'un mois à l'autre à
l'aveugle, sans voir les deux bornes ensemble. Le date-range picker de Material affiche **deux
mois côte à côte** ; il manquait à frus.

## Décisions techniques

- **Composition de deux `DatePicker::range`.** `range_dual(year, month, …)` construit le mois
  demandé **et le suivant** (avec bascule d'année décembre → janvier), chacun un
  `DatePicker::range` partageant la **même** plage `[start, end]`, puis les pose dans une
  `Flex::row`. Aucune logique de plage dupliquée : la bande **traverse** naturellement la
  frontière car `range_mark` compare des dates **complètes** `(année, mois, jour)` (jalon 184).

- **Désambiguïsation du mois cliqué.** En mode simple, `on_select` rend le **jour** (le mois est
  celui affiché). En double, `on_select` rend la **date complète** `(année, mois, jour)` : chaque
  mois enveloppe son `on_select(day)` interne en `on_select((son_année, son_mois, day))`. Le
  rappel partagé (`on_select`, `on_nav`) est mis en `Rc` pour alimenter les deux mois ; `on_nav`
  décale la **paire**.

- **Un drapeau `dual` pour la largeur.** `DatePicker` gagne un champ `dual` : `style()` renvoie
  `2 × largeur_mois + écart` en double, la largeur d'un mois sinon. Le reste (grille, cases,
  peinture) est **inchangé** — le mode double n'est qu'un agencement de deux calendriers simples.

## Implémentation

- `datepicker.rs` : `DatePicker::range_dual` (mois + suivant, `Rc` partagé, `Flex::row`) ; champ
  `dual` (+ largeur dans `style`) ; `assemble` initialise `dual: false`.
- `goldens.rs` : `date_range_dual` (juillet + août 2026, plage 28/07 → 03/08).

## Vérification

- **Unitaire** : `range_dual_shows_two_consecutive_months` — un seul enfant (la rangée), deux
  calendriers ; bascule décembre 2026 → janvier 2027 ; cliquer le 3 janvier du mois de droite
  rapporte `(2027, 1, 3)` (date complète, à l'index attendu). Tests des jalons 184 **verts**.
- **Golden** `date_range_dual` **inspecté** : juillet (28 début + 29–31 en bande), août (1–2 en
  bande, 3 fin) — la plage se poursuit à travers la frontière de mois.
- `cargo test -p frus-widgets datepicker::` **vert**.

## Reste

- **Navigation partagée** : chaque mois porte ses propres flèches ‹ › (quatre au total) ; une
  barre de navigation unique au-dessus de la paire serait plus sobre (extension d'agencement).
- **Aperçu au survol** (bande provisoire jusqu'au jour survolé pendant la saisie) — l'état de
  survol existe déjà, à câbler applicativement.
