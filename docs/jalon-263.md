# Jalon 263 — Défilement vertical par colonne : blocage layout + garde-fou réordonnables-dans-Scroll

## Objectif visé

Compléter le patron Trello : chaque **colonne** défile ses cartes **verticalement**, indépendamment
(scroll horizontal du board au jalon 260 + scroll vertical par colonne ici).

## Ce qui s'est passé

Tentative : envelopper la liste de cartes de chaque colonne dans un `Scroll { axis: Vertical,
flex: 1 }`, colonnes étirées à la hauteur du board. Résultat : le `Scroll` interne **s'effondre** — les
cartes disparaissent (leur rectangle visible est nul, donc elles ne sont plus ni peintes ni
**enregistrées comme réordonnables**).

**Cause (limite du layout frus).** Un `Scroll` en `flex(1)` n'obtient une hauteur exploitable que si la
**chaîne d'ancêtres** lui fournit une hauteur **définie**. Or ici : board `Row` (hauteur `Auto`) →
colonne (`Auto`/`Percent`) → `Scroll` `flex(1)`. `align: Stretch` et `Percent(1.0)` **ne suffisent
pas** : la hauteur étirée n'est pas traitée comme une base définie pour le flex interne (le viewport du
scroll est bien calculé — 196×248 dans le registre — mais son contenu est découpé à zéro). Il manque à
frus une primitive « **remplir la hauteur disponible puis défiler** » (façon `Expanded` + `ListView`)
que `Scroll`/`Flex` n'offrent pas encore de façon fiable. La tentative a donc été **remisée**.

## Découverte importante (et garde-fou)

En instrumentant, constat : **les réordonnables placés dans un `Scroll` interne n'étaient pas
enregistrés**. Cela a soulevé une crainte sérieuse — les jalons 258/260 **enveloppent le board (avec
ses cartes) dans un `Scroll` horizontal**, et je n'avais **re-testé le glisser au doigt qu'avant** cet
enveloppement. Vérification : un test dédié montre que **le board dans un `Scroll` horizontal enregistre
bien ses cartes réordonnables** (`>= 2`). Donc **258/260 n'ont pas cassé le glisser** — l'effondrement
était **spécifique** au `Scroll` **vertical en `flex(1)` sans hauteur d'ancêtre définie**, pas au fait
d'être dans un `Scroll`.

Le test `reorderables_inside_a_scroll_are_still_registered` est **conservé** comme garde-fou : il
protège le glisser du board-dans-scroll contre une régression future.

## Implémentation

- `frus-widgets/src/ui.rs` : test `reorderables_inside_a_scroll_are_still_registered` (board enveloppé
  dans un `Scroll` horizontal → cartes toujours réordonnables). La tentative sur `kanban.rs` a été
  **entièrement annulée** (structure de colonne inchangée).

## Vérification

- **Widgets 394** (dont le garde-fou) ; kanban 7. *(Doctests bloqués au **runtime** par SAC — os error
  4551, environnement, pas une régression : ils compilent.)*

## Reste

- **Défilement vertical par colonne** : rouvrir quand frus aura une primitive « fill-then-scroll »
  fiable (ou via une **hauteur de colonne explicite** passée par l'app en solution d'attente).
- Inertie verticale du glisser.
