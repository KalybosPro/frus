# Jalon 56 — Phases de frame : build conditionnel (build → paint)

Deuxième item moteur du §1 de `docs/idees-flutter.md` : **séparer la frame en
passes indépendamment invalidées**, chacune ne s'exécutant que si son bit « dirty »
est posé. Le jalon 55 a fourni la moitié « layout » (cache de relayout) ; celui-ci
ajoute la séparation **build → paint** au niveau du shell.

## L'observation clé

Dans le modèle Elm de Frus, la `view` est une **fonction pure de
`(état de l'app, thème, taille)`**. Elle ne lit **jamais** le `Runtime` — survol,
focus, offsets de scroll, curseurs, progressions d'animation d'interaction vivent
dans le shell, hors de la vue. Donc :

> Une frame d'animation d'interaction (un survol qui monte, un scroll à ressort, un
> curseur qui clignote) **ne change pas la sortie de `view`** — elle n'a besoin que
> de **repeindre** l'arbre déjà construit.

Jusqu'ici, chaque frame reconstruisait pourtant tout l'arbre (`app.view()` +
détection montages/sorties) avant de peindre.

## Le bit `build_dirty`

`App` gagne un drapeau `build_dirty`. La phase **build** (`app.view()` + montages +
capture des sorties) ne s'exécute que si l'état a pu changer :

```
need_build = build_dirty || app_animating || (aucun arbre retenu)
```

- `build_dirty` est posé **exactement** aux six seuls points qui mutent l'état de
  l'app : `dispatch` (tout `Msg`), `on_resize`, `on_insets`, et les trois hooks du
  geste retour (`back_gesture` ×2, `back_gesture_end`) ; plus la (re)création de
  surface.
- `app_animating` (retour de `app.tick`) couvre les animations *propres à l'app*
  (fondu de thème, transition d'écran, détente de geste) qui, elles, modifient bien
  l'état lu par la vue à chaque frame.

Sinon, l'**arbre retenu** (`self.tree`, déjà conservé pour le routage clavier) est
réutilisé tel quel. La phase **paint** — avance des animations du `Runtime` puis
`build_ui` (dont la mise en page passe par le cache de relayout du jalon 55) —
s'exécute, elle, à chaque frame animée.

## Pourquoi c'est correct (et sûr)

La direction du risque est asymétrique : **construire quand ce n'est pas nécessaire
est inoffensif** (juste un peu plus lent, comme avant) ; **sauter quand il le
fallait** serait un bug (UI figée). Comme l'état de l'app n'est mutable **que** par
`update`/`tick`/`on_resize`/`on_insets`/`back_gesture*` — toutes marquées
`build_dirty` (ou couvertes par `app_animating`) — et que `view`/`theme` prennent
`&self`, l'arbre retenu ne peut jamais devenir obsolète sans qu'un rebuild soit
programmé. Le survol/scroll/focus/curseur ne changent que le `Runtime`, jamais la
vue.

## Résultat

- Survol, scroll à inertie, curseur clignotant, fondu d'apparition/disparition,
  spinner : **peinture seule**, sans reconstruire ni ré-allouer l'arbre de widgets.
- Combiné au jalon 55, une telle frame ne fait plus **ni** `view()` **ni** taffy —
  seulement le parcours de peinture. C'est la discipline « un survol ne touche que
  paint » du brief.

## Validation

- Toute la suite verte, comportement inchangé : `frus-widgets` 129, `frus-core` 37,
  `frus-demo` 15, `frus-shell` 7, layout 3, gpu 4, text 2.
- `cargo build --workspace` sans avertissement ; démo lancée sans panique ni conflit
  d'emprunt. (La boucle de rendu n'est pas observable sous WSLg-root — rendu logiciel
  llvmpipe — ; la correction repose sur l'argument de pureté ci-dessus et les tests.)

## Suite possible (§1 / §12)

- Un vrai **système de listes « dirty » par nœud** (pas seulement par frame) : ne
  repeindre que les sous-arbres touchés (régions de dommage + scissor GPU, §12).
- Arbre taffy **persistant** réconcilié par identité (au-delà du cache de résultat du
  jalon 55) pour un relayout incrémental intra-racine.
