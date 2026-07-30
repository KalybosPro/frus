# Jalon 217 — Charts : halo pulsant animé au survol

## Analyse

Le survol met déjà en avant le point visé (jalon 211) via un marqueur accentué **statique**. Un
retour **animé** (halo qui pulse) attire mieux l'œil. C'est le premier usage d'animation **continue**
dans le domaine graphes — la brique `continuous()` + `Status::time` (celle du `Spinner`).

## Décisions techniques

- **Opt-in via `.animated(bool)`.** Désactivé par défaut. Quand actif, `continuous()` renvoie `true`
  (repaint continu) et le point survolé émet un **halo** : un cercle qui grandit (`PULSE_GROW`) et
  s'estompe sur un cycle (`PULSE_SPEED`), dérivé de `Status::time`. Le halo est peint **sous** le
  marqueur plein.

- **Réutilise l'infra existante.** Aucune nouvelle plomberie runtime : `continuous()` pilote déjà le
  repaint continu (prouvé par `Spinner`), `Status::time` fournit le temps écoulé. Le halo ne s'affiche
  qu'au survol (dans le bloc infobulle) et hors mode empilé (où la hauteur individuelle n'a pas de
  sens).

- **Coût maîtrisé.** Le repaint continu n'est demandé **que** si `.animated` est posé — un graphique
  fixe reste à coût nul.

## Implémentation

- `frus-widgets/src/chart.rs` : champ `animated` + `.animated(bool)` sur `LineChart` ;
  `continuous()` renvoie `animated` ; halo pulsant dans le bloc infobulle ; constantes
  `PULSE_SPEED` / `PULSE_GROW`.

## Vérification

- `animated_pulse_adds_a_halo_and_requests_continuous_repaint` : `continuous()` suit `animated` ; au
  survol, le graphique animé dessine **un cercle de plus** (le halo) que le graphique fixe.
  (Animation continue au survol : non *goldenable* via `render_widget` ; couverte par ce test.)

## Note d'exécution

Le binaire de test fraîchement compilé a été **bloqué par Smart App Control** (os error 4551) au
lancement natif — le gotcha connu de cette machine. Tests exécutés via **WSL** (ELF Linux, hors SAC),
la logique du chart étant pure (pas de GPU requis).

## Reste

- **Pulse sur la BarChart** (contour de la barre survolée) et surtout le **pulse « à l'arrivée » sur
  une cellule de grille** (jalon 214) : un one-shot déclenché par le focus, qui demande une primitive
  d'animation transitoire au runtime (non encore disponible) plutôt qu'un `continuous()` permanent.
