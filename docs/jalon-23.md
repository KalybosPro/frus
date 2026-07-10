# Jalon 23 — Défilement avec inertie (ressort + rebond)

Le défilement passe de **discret** (chaque cran de molette saute) à **lissé** :
la molette pousse une *cible*, l'offset courant la rejoint à ressort, avec
**rebond élastique** aux bords.

## Choix (honnêteté CTO)

L'« inertie de flick » tactile a besoin d'une **vélocité de doigt** ; ici l'entrée
est une **molette** (crans discrets). Le bon modèle est donc un **défilement à
ressort vers une cible** + **rubber-band** aux bords — et non une friction libre.
Avantages : réutilise `spring_step` (même langage de mouvement que nav/geste), et
c'est **déterministe donc testable** (les constantes de friction, elles, se
règleraient au ressenti — impossible en rendu logiciel sans entrée injectée).

## Mécanisme

`Runtime` tient, par zone défilable :

- `scroll` — offset **courant** (rendu),
- `scroll_target` — offset **visé**,
- `scroll_velocity` — vitesse du ressort.

Chaque frame (`Runtime::advance_scroll`, piloté par le framework) :

1. La **cible** est ramenée vers `clamp(cible, 0, max)` (rappel élastique) — un
   dépassement au-delà des bornes revient donc en douceur (rebond).
2. L'**offset courant** ressort vers la cible via `spring_step` (K=200, C=28).
3. Au repos (seuils px), on nettoie l'état d'animation (l'offset reste).

Entrées :

- **Molette** : pousse la cible, avec dépassement autorisé `SCROLL_OVER = 48 px`.
- **Barre de défilement (glissement)** : reste **directe** (cible synchronisée,
  vitesse coupée) — un drag doit être précis, pas élastique.

Les bornes `max` viennent de la dernière `Ui::scrollable_maxes()` (stables d'une
frame à l'autre → pas de latence : l'offset avancé est rendu la même frame).

## Tests

- `scroll_springs_to_target_and_settles` : l'offset rejoint la cible puis se fige
  (état d'animation nettoyé).
- `scroll_overshoot_rubber_bands_back_to_max` : une cible au-delà de `max` revient
  exactement à `max`.
- Total : **35 tests frus-widgets** + frus-demo + doctest.

## Limites (v1)

- Ressenti non réglé finement (pas de test interactif ici) ; constantes
  conservatrices.
- Pas d'inertie tactile vraie (entrée molette uniquement) — le jour où une entrée
  tactile/trackpad avec vélocité arrive, on amorcera `scroll_velocity` directement.
