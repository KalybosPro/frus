# Jalon 10 — Curseur, navigation, sélection et presse-papier

Rend les champs de saisie pleinement éditables et introduit le premier **état de
widget retenu au runtime, clé par identité**.

## Décision d'architecture — un `Runtime` d'état de widgets

L'état d'interaction/édition est regroupé dans un [`Runtime`] passé à `build_ui` :

```rust
struct Runtime {
    input: InputState,                 // survol / pression / focus
    scroll: ScrollState,               // offsets de défilement
    edits: HashMap<WidgetId, Edit>,    // NOUVEAU : curseur / sélection par champ
}
struct Edit { cursor: usize, anchor: Option<usize> }  // indices caractères
```

La **valeur** d'un champ reste contrôlée (état applicatif) ; **curseur/sélection**
sont le premier état de widget retenu **par identité** (`WidgetId`, Jalon 6) —
la vraie brique de reconciliation.

## Ce qui est livré

- **`Key`** enrichi : `Left/Right/Home/End{shift}`, `Delete`, en plus de
  `Text/Backspace/Enter`.
- **`Widget`** : `on_edit(&mut Edit, &Key)` (édition), `cursor_at(local_x)`
  (placement au clic), `selected_text(&Edit)` (copie).
- **`Status`** porte `cursor`/`selection` → le champ dessine curseur + surbrillance.
- **`TextInput`** : insertion au curseur, navigation, sélection Shift+flèche,
  suppression (Backspace/Delete), placement du curseur **au clic**.
- **Presse-papier** (via `arboard`, couche shell) : **Ctrl+C/X/V**, **Ctrl+A**
  (tout sélectionner). Le collage réutilise `Key::Text`.
- **`find_widget(tree, id)`** : retrouve un widget par identité pour lui router
  clavier / requêtes.

## Boucle runtime (shell)

```
ModifiersChanged → suit Shift / Ctrl
MouseDown        → focus + curseur placé au point cliqué (cursor_at)
KeyPressed       → Ctrl+C/X/V/A (presse-papier) sinon on_edit(&mut edit, key)
                   → met à jour Runtime.edits (+ valeur via Msg) → redraw
Redraw           → build_ui(&tree, size, &runtime)
```

## Tests

- Édition : insertion au curseur ; Shift+flèche sélectionne puis Backspace
  supprime ; Home/End bornent le curseur.
- `selected_text` renvoie la plage sélectionnée.
- `find_widget` + `on_edit` produisent le message d'édition attendu.

## Simplifications (v1)

- Mono-ligne ; indices **caractère** (pas grapheme/emoji composites) ; pas de
  drag-sélection à la souris ; presse-papier best-effort (échec silencieux si
  indisponible, p. ex. environnement sans presse-papier).
