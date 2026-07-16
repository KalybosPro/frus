# Jalon 73 — Fling tactile : le momentum de défilement (balistique)

Le manque le plus visible du défilement (§6) : relâcher le doigt après un
glissement rapide **arrêtait le contenu net** — aucun momentum tactile (la
molette avait son inertie à ressort, le doigt non). C'est la première rencontre
de la couche physique du jalon 53 avec le défilement.

## Le mécanisme (balistique en forme close, ressort existant)

1. **Vitesse du doigt** : pendant un `Drag::Scroll`, la vitesse en espace
   défilement (le contenu va à l'opposé du doigt) est lissée par moyenne
   exponentielle — l'élan du relâchement.
2. **Projection balistique** : `fling_destination(position, vitesse)` =
   la **position finale d'une `FrictionSimulation`** (forme close :
   `x∞ = x₀ + v/ln(1/drag)`, drag 0.135 — la constante usuelle : ~1000 px de
   course pour 2000 px/s). Sous 50 px/s, pas d'entraînement.
3. **Glissement** : la destination (bornée avec le dépassement élastique
   existant) devient la **cible du ressort de défilement**, amorcé par l'élan du
   doigt — décélération douce, **rebond aux bornes gratuit** (le rubber-band du
   jalon 23 fait le reste). Un fling au-delà du bord dépasse élastiquement puis
   revient — la sensation native.

Aucun nouvel état ni nouvelle boucle : `advance_scroll` est **intouché** (ses
tests épinglés passent tels quels) ; le fling ne fait que *semer* cible + vitesse.
Molette, barres et glissement précis inchangés.

## Validation

- **250 tests**, tout vert — `fling_projects_a_friction_final_position` épingle la
  forme close (≈ v/ln(1/drag), symétrie, seuil), et les tests de ressort/rebond
  existants sont inchangés.
- Build sans avertissement ; démo sans panique. Bénéficiaires immédiats : toutes
  les zones défilables au doigt — **Android** en tête (listes, Journal 5000
  lignes).

## Non couvert (assumé)

- Vitesse par moyenne exponentielle (le fit **LSQ** du brief = palier 3 des
  gestes, différé).
- La restructuration complète en 4 pièces (`Position/Controller/Physics/
  Activity`) viendra avec l'arène (palier 2) — ce jalon en livre la pièce
  `Physics::createBallisticSimulation`, en pratique.

## Suite (§6 restants)

Modèle clavier régularisé (physical + logical + character), split
`padding`/`viewInsets` (évitement du clavier Android).
