# Jalon 175 — Retour du focus à la fermeture d'un overlay

## Analyse

Au clavier, ouvrir un menu/une modale déplace le focus **dans** l'overlay (piégé depuis les
jalons 172/174). Mais à la **fermeture**, le widget focalisé (un item de menu) **disparaît**
de l'arbre : le focus devenait orphelin, et la navigation repartait du **début** de la page.
Le motif attendu (« roving focus ») : revenir au **déclencheur** — le bouton qui a ouvert
l'overlay.

## Décisions techniques

- **Historique de focus, côté shell.** Plutôt qu'un couplage overlay ↔ ancre, le shell tient
  un petit **historique** des focus successifs. À chaque changement, l'ancien focus (encore
  présent) est empilé comme **déclencheur candidat**. Si, après reconstruction, le focus
  courant a **disparu** des focusables, on **dépile** jusqu'au premier encore présent et on
  l'y ramène. Cela gère naturellement l'**imbrication** (menu dans une modale) : l'historique
  fait remonter d'un cran à la fois.

- **Détecté par disparition, pas par événement.** L'app ne signale pas « overlay fermé » ; le
  shell le **déduit** en comparant le focus courant aux focusables de la frame fraîchement
  construite (`Ui::focusable_ids`). Robuste et général : tout focus qui s'évapore retombe sur
  un ancêtre de focus présent.

- **Logique pure et testable.** Le cœur est une fonction pure `resolve_focus(current,
  present, &mut history, &mut prev)` — testable sans fenêtre ; le shell ne fait que lui
  fournir l'ensemble présent et appliquer le résultat (redessin si le focus a bougé).

## Implémentation

- `ui.rs` : `Ui::focusable_ids()` (identités de tous les focusables de la frame).
- `app.rs` : champs `focus_history` / `prev_focus` ; `reconcile_focus()` appelée après
  construction de l'`Ui` (avant l'annonce AccessKit, focus à jour) ; fonction pure
  `resolve_focus` (+ borne `FOCUS_HISTORY_MAX`).

## Vérification

- **Unitaire** : `focus_returns_to_trigger_when_overlay_closes` — ancre → item (ancre
  empilée) → item disparu → **retour à l'ancre**, historique consommé.
  `focus_falls_to_none_when_no_trigger_remains` — sans déclencheur présent, le focus retombe
  à `None`.
- `cargo test --workspace` **vert** (25 tests shell).

## Reste

- **Restauration explicite au déclencheur exact** (id de l'ancre mémorisé par l'overlay)
  plutôt que par historique : plus direct si un cas tordu échappait à l'heuristique.
- **Anneau de focus visible** au retour : `focus_visible` pourrait être forcé pour rendre le
  saut perceptible (comme les demandes `Command::focus`).
