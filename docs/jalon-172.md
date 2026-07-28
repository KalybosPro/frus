# Jalon 172 — Tableau : menu de colonne au clavier

## Analyse

Après le widget d'action d'en-tête (jalon 170), la suite naturelle était le **menu de
colonne** : un bouton d'en-tête qui ouvre un menu d'actions (trier ↑/↓, masquer…),
atteignable au **Tab** et piloté flèches/Entrée. La question : fallait-il un nouveau
mécanisme dans le tableau ?

## Décisions techniques

- **Rien à construire : la composition suffit.** Un [`Menu`](crate::Menu) (ou un
  [`Dropdown`](crate::Dropdown)) déposé en `header_action` **est** un menu de colonne. Vérifié
  de bout en bout :
  - **Overlay imbriqué rendu.** Le walk de mise en page détecte `overlay()` sur **n'importe
    quel** nœud visité (méthode forwardée par `Box<dyn Widget>`) ; le menu flottant d'un
    `Menu` **enfant d'une cellule d'en-tête** est donc bien collecté et rendu par-dessus la
    grille, exactement comme au niveau racine.
  - **Clavier.** Le bouton d'action est déjà dans l'ordre de focus (jalon 170) ; les items du
    `Menu` sont `focusable` → navigables ; Entrée/Espace les active (chemin shell du jalon 167).
  - **Fermeture.** `overlay_dismiss` remonte jusqu'à `Ui::top_dismiss` (Échap / clic extérieur).

- **Pas de faux jalon.** Plutôt qu'un mécanisme redondant, ce jalon **verrouille** la
  composition par un test de non-régression et la **documente** comme recette sur
  `header_action`. L'application pilote l'état ouvert/fermé du menu (cohérent avec
  l'architecture : le tableau ne détient pas d'état transitoire).

## Implémentation

- `table.rs` : note de doc « Menu de colonne » sur `header_action` (recette + garanties).
- `goldens.rs` : `table_column_menu` (bouton « … » d'en-tête ouvrant un menu flottant).

## Vérification

- **Unitaire** : `header_action_menu_opens_as_column_menu` — un `Menu` ouvert en action
  d'en-tête voit son overlay **collecté même imbriqué** (`top_dismiss` = message de
  fermeture) et ses items **peints** au-dessus de la grille.
- **Golden** `table_column_menu` **inspecté** : bouton « … » en en-tête, menu flottant
  « Sort ascending / descending / Hide column » par-dessus les données.
- `cargo test --workspace` **vert**.

## Reste

- **Piège de focus (modale)** du menu ouvert : le scope de focus modal est géré au niveau des
  overlays ; à confirmer pour un menu de colonne si l'app veut piéger le Tab dans le menu.
- **Item par défaut / rôle ARIA** du menu : porté par `Menu`/`Dropdown`, pas par le tableau.
