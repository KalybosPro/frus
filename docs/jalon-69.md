# Jalon 69 — Gestes, paliers 0+1 : entrée normalisée + appui long

Ouverture du **Bloc B** du brief (§3, « le plus gros manque structurel côté
entrée »), par ses deux premiers paliers — en livrant au passage une capacité
nouvelle : **`on_long_press`**.

## Palier 0 — l'entrée pointeur normalisée

Les quatre sources winit (souris pressée/relâchée, curseur, tactile 4 phases)
convergent vers **une** entrée : `PointerEvent { kind: Down/Move/Up/Cancel,
position (px logiques), touch }` → `App::pointer()`. Le **`Cancel` est
première-classe** (le brief y insiste : app en arrière-plan, geste volé →
abandonner sans callback de succès) — il réinitialise glissement, pression et
reconnaisseur. Non-jetable : c'est le socle sur lequel l'arène (palier 2) se
branchera. *(Le chemin de hit-test complet `Vec<HitEntry>` et le `PointerRouter`
multi-pointeurs sont différés avec l'arène — pour ne pas livrer d'API morte.)*

## Palier 1 — le reconnaisseur tap-ou-appui-long (vocabulaire d'arène)

`PressRecognizer` (frus-shell, machine **pure** — les instants sont passés en
paramètres, donc testable au tick près) :

- L'**appui long accepte avidement** au franchissement du délai (500 ms
  immobile) : le message est émis immédiatement et le relâchement suivant est
  **avalé** (l'appui long évince le tap) — exactement la sémantique d'arène.
- Le **tap accepte passivement** : relâchement avant l'échéance → le chemin de
  clic existant, intact.
- Mouvement au-delà du **slop** (8 px) → l'appui long est rejeté, le geste
  redevient glissement/défilement. `Cancel` → abandon.
- **Réveil précis** : `ControlFlow::WaitUntil(échéance)` arme la boucle winit
  pile au bon moment (`new_events(ResumeTimeReached)` fait tirer le
  reconnaisseur) — zéro frame de polling, cohérent avec la discipline
  « 0 CPU au repos ».

Un appui capturé par une barre/poignée/sélection ne candidate pas ; un
défilement tactile **pas encore en mouvement** reste candidat (le slop tranche).

## L'API : `on_long_press`

- `Widget::on_long_press()` (hook, délégué par `Box`/`Keyed`/`Responsive`) ;
  builder `Container::on_long_press(msg)`.
- `Ui::long_press_at(point)` : cible la plus au-dessus (collectée comme les
  hits, bornée au visible).
- **Démo** : appui long sur une ligne de tâche = suppression (le motif mobile),
  en plus du bouton ×.

## Validation

- **245 tests**, tout vert — dont 5 tests du reconnaisseur (tire une seule fois à
  l'échéance ; tap avant l'échéance non avalé ; slop rejette ; cible non
  intéressée inerte ; cancel abandonne) et la collecte du plus-au-dessus.
  Comportements existants intacts (clics, drags, geste retour, demo 15).
- Build sans avertissement ; démo sans panique.

## Suite (Bloc B)

- **Palier 2** : la vraie arène (`Arena::resolve/close/sweep` pures renvoyant les
  outcomes), `PointerRouter`, chemin de hit-test complet, multi-pointeurs — quand
  des régions imbriquées indépendamment défilables l'exigeront.
- Vélocité LSQ, scale/pinch : palier 3 (différé).
