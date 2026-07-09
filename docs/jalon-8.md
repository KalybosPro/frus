# Jalon 8 — Saisie de texte + focus clavier

Ajoute un champ de saisie éditable, le focus clavier et les événements clavier.

## Position d'architecture

On avait annoncé « la vraie reconciliation ». En pratique, avec le modèle
**contrôlé** (à la iced), elle n'est pas nécessaire ici :

- la **valeur** du champ vit dans l'état applicatif (`State`) → préservée entre
  rebuilds sans machinerie ;
- le **focus** est de l'état runtime, **clé par `WidgetId`** (l'identité du
  Jalon 6), comme le survol/pression.

On livre donc un champ éditable complet **sans moteur de diff**. Un arbre d'état
persistant (vraie reconciliation) ne servira qu'à de l'état non contrôlé et
riche (navigation curseur/sélection, scroll interne, animations) : différé.

## Ce qui est livré

- **`Key`** (`Text`/`Backspace`/`Enter`) et **`Status { interaction, focused }`**
  transmis à `Widget::paint`.
- **`InputState.focused`** : widget qui a le focus clavier.
- **`Widget`** : méthodes par défaut `on_key(&Key) -> Option<Msg>` et
  `focusable() -> bool`.
- **`TextInput`** : champ contrôlé, focalisable, avec fond/bordure de focus et
  curseur ; `append` + `backspace`.
- **`Ui::focus_hit(point)`** et **`dispatch_key(arbre, focused, key)`**.
- **Runtime** (shell) : focus posé au clic, arbre conservé pour router les
  touches, événements clavier winit traduits en `Key`.

## Boucle runtime

```
MouseDown  → pressed = ui.hit(cursor) ; focused = ui.focus_hit(cursor) ; redraw
KeyPressed → si focused { msg = dispatch_key(tree, focused, key) ; update ; redraw }
Redraw     → tree = view(state) ; ui = build_ui(&tree, size, &input) ; render ;
             conserver ui (hit-test) + tree (routage clavier)
```

## Démo

Un champ « Nom : [____] » éditable au clavier (focus au clic, anneau de focus,
curseur) et une salutation « Bonjour {nom} ! » qui se met à jour à la frappe —
le champ étant contrôlé, sa valeur vient de l'état et y retourne via `Msg`.

## Tests

- `TextInput::on_key` : `Text("c")` puis `Backspace` produisent la bonne valeur.
- `Ui::focus_hit` + `dispatch_key` : une touche routée vers le champ focalisé
  produit le message d'édition attendu.
- `focusable()` : le champ est focalisable ; statut focus indépendant du survol.

## Limites (prochains jalons)

- Pas de **navigation curseur** (flèches), de sélection, ni de copier/coller.
- Pas de moteur de diff (inutile tant que l'état reste contrôlé).
- Toujours pas de scroll/clipping.
