# Jalon 245 — Démo : confirmation avant suppression groupée

## Analyse

Le `Delete` groupé (jalon 243) supprime **immédiatement** les lignes cochées — irréversible et sans
filet. La démo dispose déjà d'un motif de **confirmation modale** (effacement des tâches terminées :
`Portal` + `Card` centrée, fermable au clic extérieur). Ce jalon applique le même motif au `Delete` du
tableau de données, pour boucler proprement le domaine.

## Décisions techniques

- **Réutilise le motif existant.** `Portal::new(écran).overlay(carte, Placement::Center)
  .dismiss(Msg::DataCancelDelete)` — identique à la confirmation d'effacement, pour une UX cohérente.

- **Le bouton n'agit plus directement.** Dans la barre d'actions, `Delete` émet désormais
  `Msg::DataAskDelete` (ouvre la modale) au lieu de `Msg::DataDeleteChecked`. La modale porte les deux
  issues : `Cancel` (`DataCancelDelete`) et `Delete` (`DataDeleteChecked`, la suppression réelle).

- **Navigation bloquée pendant la modale.** `can_go_back` inclut `!data_confirm_delete`, comme les
  autres modales — le geste/bouton retour ne navigue pas tant que la confirmation est ouverte.

## Implémentation

- `frus-demo/src/lib.rs` : état `data_confirm_delete` ; `Msg::{DataAskDelete, DataCancelDelete}`
  (+ `DataDeleteChecked` remet le drapeau à zéro) ; `data_confirm_content(count)` (carte « Delete
  selected rows? » + Cancel/Delete) ; `data_screen` fait émettre `DataAskDelete` au bouton et enrobe
  l'écran d'un `Portal` quand la modale est ouverte ; `can_go_back` mis à jour.

## Vérification

- **Démo** `data_table_screen_…` étendu : `DataAskDelete` ouvre la modale (rien n'est supprimé) ;
  `DataCancelDelete` la ferme sans supprimer ; `DataAskDelete` puis `DataDeleteChecked` supprime la
  ligne cochée et referme la modale.
- Démo 34 ; shell compile (widgets/goldens inchangés).

## Reste

- Un nouveau domaine de widgets : `Tree` view (arbre extensible, sélection) ou `Kanban`
  (colonnes + cartes, glisser-déposer).
