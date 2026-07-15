# Jalon 53 — Physique unifiée (`trait Simulation`)

Point de départ des **fondations moteur** proposées par `docs/idees-flutter.md`
(§4). Jusqu'ici, l'animation était **fragmentée** : une courbe en ressort codée à
la main (`spring_ease`), un intégrateur d'Euler ad hoc (`spring_step`), une
physique de défilement séparée (`scroll_axis`), et des ressorts par-widget dans
`runtime.rs`. Chaque mouvement réinventait ses maths ; rien ne se généralisait.

Ce jalon introduit la brique que Flutter a prouvée et que le brief qualifie
d'« idée la plus portable » : **une simulation est une fonction pure
`temps → valeur`**. Fling, momentum de scroll et feuilles glissantes peuvent
désormais partager un seul chemin.

## La couche `frus_core::animation` (pure, zéro-dépendance)

Placée dans `frus-core` (socle commun, sans rendu ni plateforme), donc utilisable
par les widgets **comme** par le shell.

- **`trait Simulation { x(t), dx(t), is_done(t), tolerance() }`** — le contrat
  commun, sans état mutable ni emprunt vers le haut.
- **`SpringSimulation`** — ressort amorti en **forme close**, les trois régimes
  choisis par le discriminant `c² − 4mk` : critique, suramorti, sous-amorti
  (formules portées de `physics/spring_simulation.dart`). `SpringDescription`
  décrit le ressort par `(masse, raideur, amortissement)` ou par un **ratio
  d'amortissement** (`with_damping_ratio`, `1` = critique). Le régime critique est
  détecté avec une tolérance relative pour que `ratio = 1` n'oscille pas à cause
  des arrondis flottants.
- **`FrictionSimulation`** — décélération de momentum en forme close, avec
  `through()` (calibre le `drag` pour passer par deux points) et `final_x()`.
- **`ClampedSimulation`** — épingle la **position** dans `[min, max]` tout en
  laissant la **vitesse** continuer de rapporter (pour le scroll).
- **`Curve`** — façonnage `[0,1] → [0,1]` : `Linear`, `Cubic` (Bézier par
  recherche binaire, avec les presets `ease/ease_in/ease_out/ease_in_out`),
  `Interval` (déverrouille les animations **étagées** gratuitement), `Flipped`, et
  `CriticalSpring` (la réponse indicielle qui remplace `spring_ease`).
- **`Tween<T>` + `trait Lerp`** — un pilote `[0,1]` anime n'importe quelle valeur
  typée (`f32`, `Color`, `Point`, `Size`).
- **`AnimationController`** — le **pilote** : valeur bornée + `Status`
  (`Dismissed/Forward/Reverse/Completed`) + `Box<dyn Simulation>` + `tick(dt)`.
  Tout ce qu'il fait — `forward`/`reverse`/`animate_to` (interpolation d'une
  courbe sur une durée) ou `fling` (ressort physique) — est exprimé comme une
  simulation. **Une seule boucle de tick pour tout.** C'est l'objet que le shell
  instanciera par identité (`child_id`) au jalon suivant ; la vue lira `value()`
  au paint.

## Intégration au chemin vif (parité prouvée)

`runtime::spring_ease` **délègue** désormais à `Curve::critical_spring()` (même
`omega = 8`, mêmes opérations flottantes) : la couche partagée devient la source
de vérité unique de cette courbe, **sans changer un pixel**. Les tests qui
épinglent la sensation (`bottomsheet`, `drawer`) passent inchangés — la parité
numérique est ainsi vérifiée automatiquement.

## Validation

- `frus-core` : **36 tests** (dont 23 nouveaux pour la couche animation) — les
  ressorts (3 régimes), la friction, le clamp, les courbes, les tweens et le
  contrôleur sont couverts, la cohérence `dx`↔`x` vérifiée par différence finie.
- `frus-widgets` : **122 tests** verts (parité `spring_ease`).
- `cargo build --workspace` sans avertissement ; démo lancée 8 s sans régression
  (chrono/rendu continus).

## Suite (fondations moteur restantes, §1)

- **Cache de frontière de relayout** au-dessus de taffy `(contraintes, taille,
  dirty)`.
- **Phases de frame + listes dirty séparées** (`build → layout → paint →
  composite`).
- **Câbler `AnimationController` dans le shell** : registre clé par `child_id`,
  piloté par `Command` (`animate` / `fling`), valeurs lues au paint — puis migrer
  le scroll/nav des intégrateurs d'Euler ad hoc vers `SimulationController` (change
  la sensation → à re-verrouiller par goldens).
