# Jalon 78 — Inspecteur runtime (§13, palier 1)

## Analyse

Suite du chantier DX (§13) : « exposez le dump diagnostique (§2) en overlay
(arbre + rects + ids). Un dev qui *voit* pourquoi son identité casse au
réordonnancement reste. » C'est le Widget Inspector de Flutter, palier 1 :
voir les boîtes, désigner un widget, corréler avec un dump texte.

## Architecture

- **`Widget::debug_name()`** — l'équivalent du `runtimeType` Flutter : nom du
  type concret sans chemin ni génériques (`Container<Msg>` → `Container`).
  Méthode par défaut du trait : `std::any::type_name::<Self>()` accepte
  `?Sized`, et chaque implémentation reçoit sa **copie monomorphisée** du
  corps par défaut — zéro impl à écrire dans les ~60 widgets. Les wrappers
  transparents (`Box`, `Keyed`, `Responsive`) délèguent au contenu.
- **Collecte pendant `build_ui`** : le `Builder` gagne un puits optionnel
  (`inspector: Option<Vec<InspectorNode>>`) + un compteur de profondeur
  encadrant chaque `walk`/`render_item` — les nœuds `(id, rect peint, nom,
  profondeur)` sortent dans l'ordre de peinture, overlays compris (re-racines
  à profondeur 0). `build_ui_inspected` expose le couple `(Ui, nœuds)` ;
  `build_ui` inchangé (puits `None`, coût nul).
- **`inspector.rs`** : `node_at` (le plus profond sous le point),
  `dump_tree` (texte indenté : `Nom  x,y  l×h  #id`), `paint_overlay`
  (contours teintés par profondeur ; widget désigné = voile primaire + fiche
  nom/taille/position/id près du widget, bornée à la fenêtre, posée sur
  `inverse_surface`). Ids abrégés à 32 bits (fiche **et** dump, corrélables).
- **Shell** : **F12** bascule (builds debug uniquement, `cfg!`), le dump part
  sur stderr à l'activation ; la phase paint appelle `build_ui_inspected` et
  peint le calque sur une **copie** de la scène (la scène retenue n'est pas
  polluée) ; le mouvement du curseur force le redraw quand l'inspecteur est
  actif (le surlignage suit, même au-dessus de widgets inertes).

## Décisions

- Pas de champ `nodes` retenu dans le shell : `build_ui` tourne déjà à chaque
  frame peinte, la collecte vit et meurt avec la frame.
- L'id long (64 bits) faisait déborder la fiche (repli à la largeur de la
  fenêtre — attrapé par le **golden**) : abrégé à 8 hex.
- Desktop d'abord (F12) ; le geste d'activation tactile viendra avec le
  chantier Android.

## Tests (269 → 275)

- `collects_names_rects_and_depths` (noms concrets, Keyed transparent,
  profondeurs, chemin normal inchangé), `node_at_picks_the_deepest`,
  `dump_tree_indents_by_depth`, `overlay_paints_outlines_and_hover_card`,
  `debug_names_are_short_and_delegated` (frus-widgets).
- `inspector_overlay_matches_golden` (frus-test) : l'overlay complet rendu et
  épinglé en PNG.

## Reste du §13

Hot-reload préservant l'état (`subsecond`), template `cargo new`, et pour
l'inspecteur : sélection au clic figée, affichage de l'état retenu
(hover/focus/scroll), geste d'activation tactile.
