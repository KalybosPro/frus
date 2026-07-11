# Jalon 38 — Nouveaux widgets : Tree, ColorPicker, Timeline

Trois widgets comblant des manques structurants (hiérarchie, choix de couleur,
chronologie).

## Widgets

- **`Tree::new(on_toggle).node(id, profondeur, "src", expandable, ouvert)`** —
  arbre hiérarchique **contrôlé**. L'application tient la structure et l'état
  d'expansion, et ne passe que les **lignes visibles**, à plat. Chaque ligne est
  indentée selon sa profondeur, avec un chevron ▸/▾ pour les nœuds à enfants ;
  cliquer un nœud pliable émet `on_toggle(id)`. Bon découpage (le widget rend,
  l'app possède l'arbre — comme `Toast`).
- **`ColorPicker::new(sélection, colonnes, on_pick).swatch(couleur)`** — palette de
  pastilles bâtie sur `Grid`. La pastille sélectionnée porte un anneau ; cliquer
  émet `on_pick(couleur)`.
- **`Timeline::new().event("Titre", "détail")`** — chronologie verticale : chaque
  événement est un point relié par une ligne continue, avec titre + détail.

## Démo (onglet « À propos », section « Options avancées »)

- **`Tree`** : un explorateur de fichiers pliable (`State.expanded: HashSet<u64>`,
  `Msg::ToggleNode`). L'app aplatit les nœuds visibles selon l'état.
- **`ColorPicker`** : une palette de 6 couleurs (`State.picked`, `Msg::PickColor`).
- **`Timeline`** : les jalons récents.

## Tests

- `Tree` : nœuds pliables cliquables (`on_toggle(id)`), feuilles non cliquables.
- `ColorPicker` : pastilles ; la sélection ajoute un anneau (bordure focus).
- `Timeline` : événements ; points + textes peints.
- 80 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Tree` : pas de sélection de ligne ni de lignes de guidage ; l'app gère
  entièrement la hiérarchie et l'aplatissement.
- `Timeline` : ligne continue simple (pas de branches / statuts colorés par point).
- `ColorPicker` : palette fournie par l'appelant (pas de sélecteur TSL continu).
