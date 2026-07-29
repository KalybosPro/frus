# Jalon 184 — DatePicker : sélection d'une plage de dates

## Analyse

`DatePicker` (calendrier mensuel maison) ne gérait qu'une **date unique** (`selected: Option<u32>`).
Réserver un séjour, filtrer un rapport « du…au… », planifier un congé : autant de besoins d'une
**plage** début→fin. Material propose un date-range picker qui met en avant les deux bornes et
**surligne l'intervalle**. Il manquait à frus.

## Décisions techniques

- **Un marqueur d'état par case, plus un booléen.** La case `Day` portait `selected: bool` ; elle
  porte désormais un `DayMark` : `Off`, `Selected` (mode simple, inchangé), `Start`, `End`,
  `Between`. Le rendu du mode simple (`Off`/`Selected`) est **identique** au jalon précédent —
  seul le mode plage ajoute des états.

- **Bornes en pastille, intérieur en bande.** `Start`/`End` gardent la **pastille pleine**
  (`primary`) des jours sélectionnés ; `Between` peint une **bande douce** (`primary` à 18 %,
  coins carrés pour que les jours voisins se touchent). Pour relier la pastille à la bande,
  chaque **borne** peint une **demi-bande** côté intérieur (`Start` → droite, `End` → gauche).
  La bande se coupe naturellement en fin de semaine (comme Material) — pas de logique de retour
  à la ligne.

- **Comparaison de dates par tuple.** Le marqueur d'un jour vient de `range_mark((y, m, d),
  start, end)` : les dates `(année, mois, jour)` se comparent **lexicographiquement**, donc
  `<` est l'ordre chronologique — l'intervalle traverse les **frontières de mois** sans code
  spécial. Fonction **pure** et testée à part.

- **Constructeurs factorisés.** `new` (simple) et `range` partagent `assemble(...)` qui bâtit
  en-tête, jours de semaine et grille ; seul le `mark_of(jour)` diffère. `range` reste
  **contrôlé** : `on_select(jour)` rapporte le jour cliqué du mois affiché, l'application décide
  s'il devient début ou fin (et gère une borne seule pendant la sélection : `end == None` →
  seul le début est marqué).

## Implémentation

- `datepicker.rs` : `enum DayMark` ; `Day.mark` (+ peinture bande/demi-bande/pastille) ;
  fonction pure `range_mark` ; `DatePicker::range` + `assemble` partagé (`new` s'y ramène).
- `goldens.rs` : `date_range` (juillet 2026, du 10 au 15).

## Vérification

- **Unitaire** : `range_marks_endpoints_and_interior` (bornes, intérieur, hors plage, traversée
  de mois, borne seule) ; `range_builds_grid_with_clickable_days` (grille complète, jours
  cliquables). Tests du calendrier simple (`date_math_is_correct`, `builds_header_weekdays_and_grid`)
  **verts**.
- **Golden** `date_range` **inspecté** : 10 et 15 en pastille pleine, 11–14 en bande douce,
  coupure en fin de semaine.
- `cargo test -p frus-widgets datepicker::` **vert**.

## Reste

- **Aperçu au survol** (bande provisoire jusqu'au jour survolé pendant la sélection) — l'état de
  survol existe déjà côté framework ; à câbler applicativement.
- **Double calendrier** (deux mois côte à côte) pour de longues plages — composition de deux
  `DatePicker`.
- **Bornes hors du mois affiché** : déjà correctes (les jours du mois se comparent aux dates
  complètes) ; un affichage « … » indiquant la continuité serait un plus.
