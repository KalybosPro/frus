# Jalon 249 — Kanban : cartes riches + ajout/suppression

## Analyse

Les cartes du Kanban (jalons 247/248) n'étaient que du **texte**. Un vrai tableau porte des cartes
**riches** (libellé, étiquettes, boutons d'action) et permet d'**ajouter** / **supprimer** des cartes.
Ce jalon ajoute le contenu widget par carte et l'affordance d'ajout, en gardant le modèle contrôlé.

## Décisions techniques

- **Cartes widgets.** Une carte peut héberger un **contenu widget** (fabrique rappelée à la
  reconstruction, comme les cellules-widget du `Table`) au lieu d'un libellé. La carte reste la tuile
  (fond, bord, source/cible de dépôt) ; son contenu se peint par-dessus. Nouveau builder
  `column_widgets(title, factories)`, à côté de `column(title, texts)`.

- **Ajout par colonne.** `on_add(f)` pose un bouton **« + Add card »** au bas de chaque colonne ;
  `on_add(col)` au clic (l'app ajoute la carte).

- **Suppression = contenu riche.** La suppression n'est pas une API du widget : l'**application** met
  un bouton **×** dans le contenu de la carte, émettant son propre message de suppression. Le widget
  n'impose rien — il rend le contenu fourni.

- **Contrôlé.** L'app tient les cartes (`data`/état) et applique ajout/suppression ; le widget rend.

## Implémentation

- `frus-widgets/src/kanban.rs` : `Card` gagne un champ `content` (widget riche ou libellé) ; enum
  interne `ColCards` (texte ou fabriques) ; builders `column_widgets` et `on_add` ; le bas de colonne
  porte la zone de dépôt puis, si demandé, un `Button` « + Add card ». Test
  `rich_cards_host_content_and_add_button_is_present` (la carte riche garde son index de
  réordonnancement et route un Move ; les clics exposent le × de chaque carte et le + Add de la colonne).
- `frus-demo/src/lib.rs` : `Msg::{KanbanAdd, KanbanDelete}` + reducers (ajout au bas, suppression de la
  carte visée) ; helper `rich_card(label, col, pos)` (libellé + × danger) ; `board_screen` passe en
  `column_widgets(...).on_add(Msg::KanbanAdd)`.

## Vérification

- **Widgets** : la carte riche héberge son contenu, reste réordonnable, et le bouton d'ajout est présent.
- **Golden** `kanban_rich` : cartes libellé + × et bouton « + Add card » par colonne — inspecté visuellement.
- **Démo** `kanban_move_relocates_a_card` étendu : `KanbanAdd` ajoute « New card » au bas ; `KanbanDelete`
  retire la carte visée.
- Widgets 386 ; goldens 77 ; démo 36 ; shell compile.

## Limite connue (glisser interactif)

Le **déplacement** de carte est correct et testé au niveau logique (`on_move`, jalons 247/248), mais son
**engagement à la souris** ne se déclenche pas encore in-app : le shell repère les cibles de
réordonnancement via le registre des widgets **cliquables** (`ui.hit`), or les cartes et zones de dépôt
n'ont pas d'action de clic. Les `Table` en-têtes fonctionnent car ils sont cliquables (tri). Rendre les
cartes réellement glissables demande un **registre des réordonnables** distinct du clic (côté `ui`/shell)
— jalon dédié à venir. Les boutons **+ Add card** et **×** (cliquables) fonctionnent, eux, dès maintenant.

## Reste

- Registre des **réordonnables** (indépendant du clic) pour engager le glisser des cartes/zones.
- Étiquettes/couleurs de carte, compteur de cartes par colonne.
