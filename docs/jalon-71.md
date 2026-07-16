# Jalon 71 — Touches feuille→racine (3 états) : Échap ferme partout

Deuxième item du §6 : le **routage des touches** en montée, avec le résultat à
trois états du brief — et son payoff immédiat : **Échap ferme la modale, le menu,
le tiroir ou la feuille du dessus, depuis n'importe où** (aucune touche Échap
n'existait jusqu'ici).

## L'infrastructure

- **`KeyResponse<Msg> { Ignored, Handled(Option<Msg>), Skip }`** — `Ignored`
  continue de remonter, `Handled` consomme (message éventuel émis), `Skip`
  arrête la montée **sans** repli.
- **`Widget::on_key(&Key) -> KeyResponse`** (hook, délégué par `Box`/`Keyed`/
  `Responsive`) — le focalisé reçoit d'abord, puis chaque ancêtre.
- **`find_path(root, id) -> Vec<&dyn Widget>`** — le chemin racine→cible (mêmes
  identités `child_id` que tous les parcours), parcouru **en sens inverse** pour
  la montée.
- **`Key::Escape`** ajouté (jamais routé vers l'édition : un champ texte
  l'ignore, il remonte).

## Le routage d'Échap (shell)

1. **Montée** le long du chemin de focus : le premier `Handled`/`Skip` arrête.
   `Portal` consomme Échap (`Handled(on_dismiss)`) — le cas « focus dans le
   dialogue ».
2. **Repli** si tout le chemin a ignoré (ou sans focus) : fermeture de l'overlay
   **le plus au-dessus** — `Ui::top_dismiss()`, mémorisé pendant le rendu des
   overlays (le dernier rendu est le plus haut ; les portails imbriqués suivent).

Leçon attrapée par le test : sous une modale ouverte, le voile plein-écran
intercepte le hit-test — tout ce qui est derrière est inatteignable au pointeur.
Les **deux** chemins sont donc nécessaires : la montée depuis le focus *intérieur*
au dialogue, et le repli overlay-du-dessus pour tous les autres cas.

## Payoff démo (sans changement de code démo)

Tous les overlays existants déclarent déjà leur fermeture (`.dismiss(...)`) :
la modale de confirmation, les menus, le tiroir et la feuille répondent à Échap
gratuitement.

## Validation

- **248 tests**, tout vert — le nouveau test épingle : la fermeture du dessus
  (`top_dismiss`), le chemin racine→contenu traversant le portail, la
  consommation d'Échap en montée par le portail, le chemin vide pour une cible
  inconnue, et l'absence de fermeture sans overlay.
- Build sans avertissement ; démo sans panique.

## Suite (§6)

Modèle clavier régularisé (physical + logical + character), scrolling en 4 pièces
(`Position/Controller/Physics/Activity`), split `padding`/`viewInsets`, scopes de
focus (piéger Tab dans une modale).
