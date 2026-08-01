# Jalon 262 — Balayage overflow des écrans (tables défilables + textes enroulés + corps verticaux)

## Analyse

Après le board Kanban (jalons 258/260), un audit transverse des écrans de la démo a relevé la **même
classe de débordement** ailleurs (cible : ~393 px logiques de large). Corrigés ici selon le patron
Flutter : contenu large → **scroller** dédié ; texte long → **enroulé** ; corps haut → **scroll
vertical** (comme `settings_screen`).

## Corrections par écran

- **Data table** (`data_screen`) — *sévère* : table de colonnes fixes (~610 px) → **région défilable
  bornée** `Scroll { axis: Both, flex: 1 }` (colonnes en X, lignes en Y — un tableau scrollable, pas un
  pan de page). Hint et détail focalisé passés en `.wrap()`. Le **corps** est `flex(1)` pour que la
  région de table remplisse la hauteur (sinon elle retombe à sa taille de base, grand vide dessous).
- **Editable grid** (`grid_screen`) — *sévère* : table éditable (~644 px) → même région défilable
  bornée + corps `flex(1)` ; hint `.wrap()`.
- **Charts** (`charts_screen`) — *haut* : hint `.wrap()` ; corps (graphiques + compagnon ≈ 550-650 px)
  enveloppé dans un **scroll vertical** (`Scroll::new().width(width).flex(1.0)`).
- **Wizard** (`wizard_screen`) — *moyen* : largeur de champ **responsive**
  (`(width - 48).clamp(240, 360)`, plafonnée) au lieu de 360 px fixes ; texte de récapitulatif
  `.wrap()` ; corps en **scroll vertical** (utile clavier ouvert).
- **Home** (`todo_screen`) — *bas* : la vitrine d'icônes (~360 px) dépassait la carte (~305 px) →
  **scroll horizontal** de hauteur fixe (52 px, celle de la rangée).

## Décision : région défilable bornée vs pan de page

Un **tableau** (Data/Grid) est une grille 2D : une région défilable en X **et** Y, **confinée** au
tableau (le reste de l'écran est fixe), est idiomatique (Flutter `DataTable` dans un scroll). C'est
distinct du **pan de page** 2D refusé pour le board Kanban (jalon 260), où c'est toute la page qui
glissait en diagonale.

## Implémentation

- `frus-demo/src/lib.rs` : `data_screen`, `grid_screen`, `charts_screen`, `wizard_screen` (+
  `wizard_input` prend `field_width`), `todo_screen`.

## Vérification

- **Desktop** : compile ; démo (lib) 36.
- **Appareil** (Huawei STK-L21), confirmé par capture :
  - **Data table** : la table remplit la hauteur (5 lignes + pagination « 1–5 of 12 »), **défile**
    (barre horizontale → Score/Level accessibles), et le hint s'affiche sur **2 lignes** ; détail et
    résumé fixes dessous. Cas le plus dur (pagination) — validé.
  - **Home** : la vitrine d'icônes défile **horizontalement** (barre sous la rangée), plus de
    débordement de carte.
  - **Editable grid** emprunte le **même** patron que Data table (région `Scroll{Both, flex}` + corps
    `flex(1)` + hint `.wrap()`) → comportement identique. `charts`/`wizard` utilisent le patron
    vertical standard (`settings_screen`).
- Écrans déjà sûrs (audit) laissés intacts : `settings_screen`, `journal_screen`, `about_section`,
  `drawer_menu`, et le corps de `todo_screen` (déjà en `Scaffold` défilant).

## Reste

- Défilement **vertical par colonne** du Kanban (patron Flutter complet).
- Inertie verticale du glisser ; helper « écran défilable » si le patron se répète encore.
