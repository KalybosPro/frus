# Jalon 260 — Défilement Kanban façon Flutter : axe horizontal intentionnel (fin du pan 2D)

## Analyse

Le jalon 258 avait corrigé le débordement du board en le rendant défilable en **2D** (`Axis::Both`).
Remarque utilisateur, juste : **façon Flutter**, une page ne défile pas librement en diagonale — le
défilement est **intentionnel, par axe**, via un scroller configuré pour cet axe. Le patron Flutter
d'un board (type Trello) : un **scroller horizontal** pour la **rangée de colonnes**, et un **scroller
vertical par colonne** pour ses cartes — deux scrollers distincts, pas un pan 2D.

## Décisions techniques

- **Board = scroller horizontal.** `Scroll { axis: Horizontal, width: viewport, flex: 1 }` : la rangée
  de colonnes défile **gauche/droite** uniquement ; verticalement, le contenu est **borné au viewport**
  (les colonnes s'alignent en haut). Plus de composante verticale au geste → plus de pan diagonal.
- **Défilement vertical par colonne : différé.** Le vrai patron Flutter (chaque colonne = liste
  verticale indépendante) demande de donner aux colonnes une **hauteur définie** (sinon le `Scroll`
  interne s'effondre — cf. le piège de dimensionnement du `Scroll`). Les colonnes de la démo **tiennent**
  verticalement aujourd'hui ; on garde donc le scroller horizontal seul et on note le vertical-par-colonne
  comme suite (changement de layout du widget `Kanban` + regénération des goldens).

## Implémentation

- `frus-demo/src/lib.rs` : `board_screen` — `Axis::Both` → `Axis::Horizontal`.

## Vérification

- **Desktop** : compile ; démo (lib) 36.
- **Appareil** (Huawei STK-L21) : **confirmé par capture** (relance propre) — barre de défilement
  **horizontale**, colonnes **alignées en haut** (aucun pan vertical), défiler révèle « To do / Doing /
  Done », hint sur 2 lignes. *(Un premier cliché montrait un rémanent de l'écran précédent : simple
  artefact de transition dû à des taps enchaînés trop vite ; disparaît à la relance propre — pas un
  bug.)*
- Les goldens `kanban`/`kanban_rich` rendent le **widget** directement (inchangé) → non affectés.

## Notes

- Cohabitation glisser/défiler inchangée : glisser une **carte** réordonne (drag armé avant le repli de
  défilement, jalon 254) ; glisser une **zone vide** fait défiler horizontalement.

## Reste

- **Vertical par colonne** (patron Flutter complet) : hauteur définie des colonnes + `Scroll` vertical
  interne, testé en remplissant une colonne de cartes ; regénérer les goldens Kanban.
- Balayage overflow des autres écrans ; polish DnD (réagencement même-colonne, inertie verticale, ombre
  `Card`/`Toast`).
