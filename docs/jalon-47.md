# Jalon 47 — Tiroir droit & tiroir permanent

Deux compléments au `Drawer` : l'accostage au bord **droit** et un mode
**permanent** (accosté dans le flux, toujours visible).

## Tiroir droit — `Placement::Right`

Symétrique de `Left` : un panneau plein-hauteur collé au bord droit, avec voile
et glissement animé (le même mécanisme piloté par le runtime, jalon 46).

- `process_overlays` place le panneau `Right` en `x = largeur_fenêtre −
  progression · largeur_panneau` (le bord droit reste collé, le panneau entre par
  la droite), hauteur contrainte à la fenêtre.
- Le voile s'applique aussi (`Center | Left | Right`).
- API : `Drawer::new(open).right()`.

Le liseré du `DrawerPanel` passe sur le bord **intérieur** (gauche pour un tiroir
droit, droite pour un tiroir gauche).

## Tiroir permanent — `Drawer::permanent(bool)`

Quand `permanent` est vrai (typiquement au palier `Expanded`), le panneau n'est
plus un overlay modal : il est **accosté dans le flux**, toujours visible à côté
du corps, **sans voile ni animation**.

- Le `Drawer` devient une **rangée** : `[panneau, corps]` (gauche) ou
  `[corps, panneau]` (droite) ; le panneau garde sa largeur fixe, le corps prend
  le reste (`flex(1)`).
- `overlay()` renvoie `None` et `anim_target()` renvoie `None` (rien à animer).
- `open` / `on_dismiss` sont ignorés dans ce mode.

C'est le pendant « accosté » du rail : un tiroir qui, en grand écran, cesse
d'être escamotable.

## Démo

Le tiroir applicatif est désormais **accosté à droite** :

- **Compact / Medium** : modal, glissant, ouvert par le bouton « ☰ » ;
- **Expanded** : **permanent** — le « hamburger » disparaît, le panneau se dote
  à droite. On obtient une mise en page à **3 zones** : rail (`NavScaffold`) ·
  corps · panneau du tiroir.

## Tests

- `frus-widgets` : `right()` → `Placement::Right` ; permanent → pas d'overlay,
  pas d'`anim_target`, rangée à 2 enfants, **aucun voile**, panneau accosté
  plein-hauteur à gauche (x ≈ 0) ; permanent + `right()` → panneau collé au bord
  droit (x + largeur ≈ largeur fenêtre).

## Limites (v1)

- En permanent, pas de bascule « replier/déplier » (le panneau est figé) — c'est
  le rôle du mode modal aux paliers plus étroits.
