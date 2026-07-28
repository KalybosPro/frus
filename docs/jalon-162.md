# Jalon 162 — Curseur de plage : survol, clic-piste & Début/Fin

## Analyse

Le `RangeSlider` (jalons 156/157/160) affichait ses infobulles **en permanence**, ne
réagissait **pas au clic sur la piste** (seules les poignées étaient interactives) et son
clavier se limitait aux flèches. Trois points du « Reste » à traiter.

## Décisions techniques

- **Infobulle révélée au survol / focus.** L'infobulle est désormais peinte par la
  **poignée** (et non le slider), et n'apparaît que si la poignée est **survolée** ou
  **focalisée** (`status.hover_progress > 0` ou `status.focused`), avec fondu. La hauteur
  reste réservée dès qu'un `value_label` est posé, mais l'affichage est contextuel — comme
  Material. Chaque poignée montre **sa** valeur.

- **Clic / glissement sur la piste.** Le `RangeSlider` (parent) redevient **glissable** :
  ses poignées étant peintes **au-dessus**, `draggable_at` renvoie la poignée quand on la
  vise, sinon la **piste** → `on_drag(fraction)` rapproche la **poignée la plus proche**
  (bornée, accrochée). On retrouve le clic-piste sans casser le glissement collant des
  poignées.

- **Début / Fin au clavier.** Nouveau routage shell : les touches **Début/Fin** sont
  proposées au widget focalisé via `on_key` avant l'édition (un champ texte les ignore ici).
  Une poignée focalisée y répond en filant à sa **borne** (0 / voisin, ou voisin / 1),
  réutilisant `moved(±grand)`.

## Implémentation

- `slider.rs` : `RangeThumb` gagne `label` + `value()` et peint la bulle **au survol/focus**
  (`paint_tip`) ; `on_key` gère aussi **Début/Fin**. `RangeSlider` transmet `label` aux
  poignées, ne peint plus les bulles, et devient **glissable** (`on_drag` → poignée la plus
  proche, avec `snap`).
- `app.rs` (shell) : Début/Fin routées vers `on_key` du focalisé avant l'action par défaut.
- `goldens.rs` : `range_slider_labels` **focalise** la poignée basse (révèle bulle + anneau).

## Vérification

- **Unitaire** : la piste est **glissable** et `on_drag` vise la poignée la plus proche
  (`0.25`→bas, `0.9`→haut) ; **Début/Fin** filent la poignée à sa borne (bas : 0 / 0.7 ;
  haut Fin : 1). Glissé collant, paliers, flèches, réserve de hauteur : inchangés.
- **Golden** `range_slider_labels` **inspecté** : poignée basse **focalisée** avec anneau et
  bulle « 30% » **révélée**, poignée haute **sans** bulle ; `range_slider` (sans étiquette)
  **inchangé**.
- `cargo test --workspace` **vert**.

## Reste

- **Infobulle pendant le glissement** : la révélation par survol/focus ne couvre pas encore
  le glissement pur (aucun signal fiable « poignée en cours de glissement » — le survol se
  perd) ; il faudrait exposer la poignée activement glissée depuis le shell.
- **PgUp/PgDn** (grand pas) — demanderait des variantes `Key` dédiées.
