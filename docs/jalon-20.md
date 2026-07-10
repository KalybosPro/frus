# Jalon 20 — App exemple réelle : liste de tâches (todo)

Première **vraie application** écrite avec frus, pour éprouver l'API bout-en-bout
plutôt que d'aligner des démos de fonctionnalités. L'écran d'accueil **devient**
l'app todo ; l'écran Réglages reste atteignable (bouton + geste retour) pour ne
rien perdre de la couverture nav / geste / overlay / thème.

## Fonctionnalités

- **Ajouter** une tâche : champ de saisie + bouton, **ou touche Entrée**.
- **Cocher / décocher** (barre le libellé en le grisant), **supprimer** (`×`).
- **Filtrer** : Toutes / Actives / Terminées (le filtre actif est mis en avant).
- **Compteurs** vivants (« N active(s) · M terminée(s) »), **état vide**.
- **Effacer les terminées** avec **modale de confirmation** (voile cliquable).
- **Thème** clair/sombre (fondu) et **liste défilante**.
- Ajouts/suppressions **en fondu** (montage/démontage, gratuit depuis J13/J14).

Modèle Elm classique : `State { todos, draft, filter, next_id, … }`, `Msg`,
`update`, `view`. Les tâches portent un `id` stable ; les messages `ToggleTodo(id)`
/ `DeleteTodo(id)` ciblent par identité (pas par position).

## Ce que l'app réelle a révélé (le but de l'exercice)

1. **Manque d'API — `TextInput` ignorait `Entrée`.** Une todo-list sans
   « Entrée pour ajouter », ce n'est pas crédible. → Ajout de
   `TextInput::on_submit(msg)` : `Key::Enter` émet le message **sans** modifier la
   valeur. Câblage shell gratuit (l'Entrée passait déjà par `on_edit`).
2. **Couplage framework/app.** L'application (`State`/`Msg`/`update`/`view`) vit
   encore **dans** `frus-shell`, pas comme consommateur externe. Écrire une vraie
   app rend ce couplage concret : la prochaine étape d'archi naturelle est une
   **API d'hébergement** (`run(app)` générique) pour que l'app soit un crate à
   part. Reporté à un jalon dédié.
3. **Identité positionnelle.** Supprimer une tâche au milieu décale l'identité
   positionnelle des suivantes (l'état retenu — survol/anim — « saute »).
   Acceptable ici ; motive une future **identité par clé**.

## API ajoutée

```rust
TextInput::new(value).on_input(Msg::Draft).on_submit(Msg::Add) // Entrée = submit
```

## Tests

- `add_todo_from_draft_and_trims_blanks` : ajoute, vide le champ, ignore les blancs.
- `toggle_delete_and_clear_done` : coche, compte, supprime par id, efface les terminées.
- `view_builds_a_non_empty_scene` : fumée de rendu.
- `TextInput` : `enter_submits_without_changing_value`, `enter_without_submit_is_noop`.
- Total : **30 tests frus-widgets** + tests shell.

## Limites (v1)

- App encore hébergée dans le shell (voir point 2) — pas de `run(app)` public.
- Pas d'édition en place d'une tâche, pas de persistance (en mémoire).
- Libellés longs non tronqués (texte mono-ligne mesuré).
