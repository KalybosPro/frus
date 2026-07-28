# Jalon 157 — Curseur de plage : poignée collante & pas discret

## Analyse

Le `RangeSlider` (jalon 156) était un **widget feuille** utilisant `on_drag(fraction)` :
la fraction déplaçait la poignée la **plus proche**. Deux limites notées au « Reste » :

- **Pas collant.** Une fois une poignée saisie, franchir l'autre lui faisait « passer la
  main » (le geste changeait de poignée) — Material garde la poignée saisie **sélectionnée**.
- **Course continue** seulement (pas de pas discret).

Le nœud : `on_drag(fraction)` est **sans mémoire** de la poignée saisie ; un widget feuille
ne peut pas savoir laquelle a été prise à l'appui.

## Décisions techniques

- **Deux poignées glissables distinctes.** `RangeSlider` devient **composite** : il peint
  la piste + le segment actif, et ses enfants sont **deux `RangeThumb`** posés le long de
  la piste par des cales (`Spacer`). Chaque poignée est un **widget glissable** à part :
  saisir une poignée glisse **cette** poignée — la stickiness est **structurelle**, sans
  état de glissement supplémentaire. Le croisement est exclu par bornage (`low` borné par
  `high` et vice-versa).

- **Delta, pas fraction.** Chaque poignée utilise `on_drag_delta(dx)` (jalon 151) : `dx`
  converti en fraction via la largeur de piste, **accumulé** sur la valeur du côté saisi.
  Contrairement à `on_drag(fraction)` — qui saturerait sur la petite boîte d'une poignée —
  le delta est insensible à la taille du widget. API inchangée : `on_change(low, high)`
  (le widget calcule l'absolu depuis son état courant).

- **Pas discret.** `divisions(n)` accroche la valeur glissée à `k/n` (arrondi), appliqué
  après bornage.

## Implémentation

- `slider.rs` : `Spacer` (cale inerte) ; `Side` (bas/haut) ; `RangeThumb` (glissable,
  `on_drag_delta` → `on_change(low, high)` borné + accroché) ; `RangeSlider` composite
  (`divisions`, `rebuild` posant les poignées, peint piste + segment, enfants = poignées).

## Vérification

- **Unitaire** : chaque poignée déplace **son** côté (+22 px = +0.1 : bas 0.2→0.3, haut
  inchangé ; −22 px : haut 0.8→0.7) ; **collant** — la poignée basse poussée à fond
  s'arrête au haut (0.8, 0.8) sans le pousser ; delta nul → aucun message ; `divisions(10)`
  accroche +0.125 → **0.1** ; `new(0.9, 0.1)` réordonne en `(0.1, 0.9)`.
- **Golden** `range_slider` **inchangé** (rendu pixel-identique : le composite peint la
  même piste + segment + deux poignées).
- `cargo test --workspace` **vert**.

## Reste

- **Infobulle de valeur** au survol / pendant le glissement (bulle au-dessus de la poignée)
  — demande un overlay conditionné par l'état de survol (aujourd'hui `overlay()` est
  structurel, pas piloté par le `Status`).
- **Clic sur la piste** pour rapprocher la poignée la plus proche (perdu en passant au
  composite : seules les poignées sont interactives).
- **Grossissement de la poignée** au survol / focus, et **navigation clavier** (flèches).
