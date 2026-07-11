# Jalon 28 — Reconciliation par clé (identité stable)

L'identité des widgets était **positionnelle** (chemin racine → indices).
Supprimer un élément **au milieu** d'une liste décalait l'identité des suivants,
et leur état retenu (survol, focus, curseur, animations, fondu de sortie)
« sautait ». Les **clés** rendent l'identité stable.

## Mécanisme

- `WidgetId::keyed(hash)` : identité dérivée de `parent + clé` (constante et
  décalage distincts de `child(index)` pour éviter les collisions).
- `Widget::key(&self) -> Option<u64>` (défaut `None` → positionnel).
- **`Keyed<Msg>`** : un wrapper **transparent** — délègue *toutes* les méthodes du
  trait au widget interne, mais renvoie `key() = Some(...)`. `Keyed::new(clé, w)`
  accepte n'importe quel type `Hash`.
- **DX** : helper `keyed(clé, widget)` dans le DSL.

Le cœur de la cohérence : un unique `child_id(parent, index, child)` —
`child.key()` ? `parent.keyed(k)` : `parent.child(index)` — utilisé **partout où
l'on dérive une identité** : `build_ui` (branches normales **et** scroll / nav /
overlay), `collect_ids`, `find_widget`, `advance_values`. Si l'un divergeait,
l'état ne correspondrait plus ; d'où le test d'intégration.

## Démo

Les lignes de tâches sont enveloppées : `keyed(todo.id, todo_row(...))`. Supprimer
une tâche au milieu ne fait plus « sauter » l'état retenu des autres lignes.

## Tests

- `WidgetId::keyed` : stable (même clé = même id), ≠ `child(index)` de même valeur,
  ≠ autre clé, ≠ même clé sous un autre parent.
- `Keyed` : renvoie une clé et délègue enfants/style ; même clé → même hash.
- **Intégration** (`keyed_identity_survives_middle_removal`) : dans une liste de 3
  éléments colorés keyés, on lit l'`owner` (= id) de la primitive d'un élément ;
  après retrait de celui du milieu, l'`owner` de l'élément survivant est
  **identique** — alors que **sans clé** il change. Preuve directe.
- 41 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- Les clés visent les **enfants de liste** (frères) ; les enfants structurels
  uniques (contenu d'un `Scroll`, écran d'un `Navigator`) honorent aussi les clés
  via `child_id` mais restent en pratique positionnels.
- Clé hachée en `u64` : collisions théoriques (négligeables).
