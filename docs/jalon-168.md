# Jalon 168 — Tableau : en-têtes à icône (+ tri de colonnes-widgets)

## Analyse

Un en-tête de tableau ne savait afficher qu'un **libellé texte**. Les grilles réelles
coiffent souvent une colonne d'une **icône** (un pictogramme de catégorie, une étoile pour
une note…) — icône **puis** libellé. Par ailleurs, depuis les cellules-widgets (jalon 164),
le **tri d'une colonne-widget** méritait d'être **documenté** : le tableau ne compare pas
des widgets, l'application fournit la clé.

## Décisions techniques

- **Icône de tête, décorative, sans casser le tri.** L'en-tête reste une `Cell` (donc
  toujours **triable** et **réordonnable**) ; elle gagne un champ `icon: Option<IconName>`,
  peint **avant** le libellé. Le libellé — et l'indicateur de tri qui le suit — se décalent
  d'une largeur d'icône : icône + texte + (▲/▼), cohabitant proprement.

- **Une icône par colonne, à la demande.** `Table::header_icons(&[Option<IconName>])` :
  `None` laisse la colonne sans icône. L'icône est purement visuelle (aucune sémantique
  ajoutée : le libellé porte déjà l'annonce du lecteur d'écran).

- **Tri de colonnes-widgets : documenté.** Le tableau n'émet que la **colonne cliquée**
  (`on_sort`) ; c'est l'**application** qui ordonne ses données par le champ correspondant
  (le nom derrière un avatar, p.ex.), puis repasse les lignes triées — comme pour les
  colonnes texte. Documenté sur `widget_row`.

## Implémentation

- `table.rs` : constantes `ICON` / `ICON_GAP` ; champ `Cell.icon` peint avant le libellé
  (le texte et l'indicateur de tri décalés) ; champ `Table.header_icons`, builder
  `header_icons` ; note de doc sur le tri de colonnes-widgets (`widget_row`).
- `goldens.rs` : `table_header_icons` (icône Menu + « Name », icône Star + « Rating ▼ »).

## Vérification

- **Unitaire** : `header_icon_shifts_label_and_paints` — le libellé d'un en-tête à icône
  recule d'au moins une largeur d'icône ; une colonne sans icône n'est pas décalée.
- **Golden** `table_header_icons` **inspecté** : icônes de tête devant les libellés,
  indicateur de tri conservé, données alignées — aucune régression sur les goldens texte.
- `cargo test --workspace` **vert**.

## Reste

- **En-tête entièrement widget** (au-delà d'icône + libellé : bouton de filtre, menu) :
  demanderait un en-tête bâti sur une fabrique tout en conservant tri/réordonnancement —
  chantier plus lourd, non requis ici.
- **Icône à droite** (après le libellé) ou **cliquable indépendamment** du tri : possible
  extension si un cas concret l'exige.
