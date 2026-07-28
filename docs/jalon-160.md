# Jalon 160 — Curseur de plage : infobulle de valeur & clavier

## Analyse

Le `RangeSlider` (jalons 156/157) était complet à la souris mais muet sur deux points du
« Reste » : **aucune valeur affichée** (on glissait sans repère chiffré) et **aucun accès
clavier** (les poignées n'étaient ni focusables ni pilotables aux flèches).

## Décisions techniques

- **Infobulle opt-in, toujours peinte.** `value_label(fmt)` : une bulle `primary` au-dessus
  de chaque poignée affiche `fmt(valeur)` (pourcentage, prix…). Choix d'un affichage
  **permanent** (et non seulement au survol) : l'état « actif » d'une poignée est peu fiable
  **pendant** le glissement (le survol se perd, l'interaction repasse à `Idle`). Peinte par
  le `RangeSlider` lui-même (qui connaît `low`/`high` et les positions), pas par les poignées.
  **Sans `value_label`, le rendu est inchangé** (hauteur = `H`, golden d'origine intact).

- **Réserve de hauteur.** Avec une infobulle, la hauteur passe à `TIP_H + TIP_GAP + H` ; la
  piste et les poignées vivent dans la bande **basse** `H`, les bulles dans la zone haute —
  dans les bornes du widget, donc jamais rognées.

- **Clavier générique.** Nouveau routage shell : une flèche **gauche/droite** est d'abord
  **proposée au widget focalisé** via `on_key` ; s'il la **consomme** (`Handled`), le focus
  ne navigue pas. Réutilisable par tout widget (pas seulement le curseur). Les poignées
  deviennent **focusables** et répondent aux flèches en déplaçant leur côté d'un **pas**
  (un palier si `divisions`, sinon 5 %), via la même logique bornée/accrochée que le glissé.
  Anneau de focus accentué sur la poignée focalisée.

## Implémentation

- `slider.rs` : `RangeThumb` factorise `moved(delta)`/`snap`/`key_step` (partagés glissé +
  clavier), devient **focusable** et gère `on_key` ; est dessinée dans la bande basse `H`.
  `RangeSlider` gagne `label` + `value_label`, `content_h` (réserve), peint piste/segment en
  bas et les **infobulles** en haut (`paint_tip`).
- `app.rs` (shell) : les flèches gauche/droite passent par `on_key` du focalisé avant la
  navigation géométrique du focus.
- `goldens.rs` : `range_slider_labels` (infobulles « 30% » / « 70% »).

## Vérification

- **Unitaire** : flèche droite/gauche déplace la poignée focalisée d'un pas
  (`divisions(10)` : 0.4 → 0.5 → 0.3), poignées **focusables** ; `value_label` **augmente**
  la hauteur (réserve). Glissé collant, paliers, bornage : inchangés.
- **Golden** `range_slider_labels` **inspecté** : bulles « 30% » / « 70% » au-dessus des
  poignées ; `range_slider` (sans étiquette) **inchangé pixel pour pixel**.
- `cargo test --workspace` **vert**.

## Reste

- **Révélation au survol / focus** de l'infobulle (aujourd'hui permanente) : demande un
  signal « poignée active » fiable pendant le glissement.
- **Clic sur la piste** pour rapprocher la poignée la plus proche.
- **Home/End** (bornes) et **PgUp/PgDn** (grand pas) au clavier.
