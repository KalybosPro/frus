# Jalon 70 — Focus : anneau clavier-seul + navigation aux flèches (géométrique)

Ouverture du **§6** par son premier item (« arbre de focus + routage des touches,
prérequis de tout »), via ses deux morceaux les plus rentables et testables.

## `FocusHighlightMode` : l'anneau ne flashe plus au clic

Le brief : *« ne peindre l'anneau de focus que si la dernière interaction était
clavier »*. Nouveau bit `Runtime::focus_visible` :

- **pointeur appuyé** → `false` (le focus reste actif — un champ garde son
  curseur —, seul l'anneau générique s'efface) ;
- **toute touche pressée** → `true` (redessin si le bit bascule).

`draw_focus_ring` est gaté dessus ; les widgets qui dessinent leur propre focus
(`TextInput` : bordure animée) ne changent pas — c'est une affordance d'édition,
pas un anneau de navigation.

## Navigation du focus **aux flèches** (politique géométrique)

`Ui::focus_directional(current, FocusDirection)` : parmi les focusables, choisit
le plus proche **dans un cône** autour de la direction (pas un simple demi-plan —
un candidat quasi aligné transversalement mais à peine « devant », dû à des
largeurs légèrement différentes, n'est pas une cible directionnelle : le bug
exact que le test a attrapé). Score = avance + 3 × écart transversal.

Côté shell : les flèches naviguent depuis tout focusable — **sauf** gauche/droite
dans un champ texte (elles y déplacent le curseur ; haut/bas naviguent même
depuis un champ mono-ligne). Tab/Shift+Tab (ordre d'arbre) inchangé.

## Validation

- **247 tests**, tout vert :
  - anneau présent au clavier, **absent** après un focus au pointeur, jamais pour
    un widget à focus propre ;
  - grille 2×2 : droite/bas corrects, **diagonale contrôlée** (depuis b, bas → d
    aligné, pas c), rien à gauche du bord — le cas dégradé des largeurs inégales
    est épinglé.
- Build sans avertissement ; démo sans panique.

## Suite (§6)

- Montée des touches **feuille→racine** à résultat 3 états (handled/ignored/
  skip) — le payoff : Échap ferme le dialogue depuis n'importe où.
- Modèle clavier régularisé (physical + logical + character), scrolling en 4
  pièces, split `padding`/`viewInsets`, scopes de focus (piéger dans une modale).
