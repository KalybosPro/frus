# Jalon 27 — DX / ergonomie (écrire une UI plus vite)

Priorité posée par la direction : **le développement doit être très facile et
rapide**. On attaque donc l'ergonomie d'écriture avant d'empiler des features.
Purement **additif** — les constructeurs restent disponibles.

## Ce qui est ajouté

Macros de layout et fonctions raccourcis (module `dsl`) :

```rust
// Avant
Flex::row()
    .align(Align::Center)
    .gap(12.0)
    .child(Text::new("Nom").size(18.0))
    .child(Flex::row().flex(1.0))
    .child(Button::new("Ajouter").on_press(Msg::Add))

// Après
row![
    text("Nom").size(18.0),
    spacer(),
    button("Ajouter", Msg::Add),
]
.align(Align::Center)
.gap(12.0)
```

- **`row![a, b, c]` / `column![a, b, c]`** → un `Flex` avec ces enfants, **restant
  chaînable** (`.gap()`, `.align()`, `.padding()`…). `row![]` = conteneur vide.
- **`text(s)`** = `Text::new(s)`.
- **`spacer()`** = un espaceur flexible (`Flex::…flex(1.0)`), qui pousse ses voisins.
- **`button(label, msg)`** = `Button::new(label).on_press(msg)` (chaînable :
  `.variant()`, `.size()`).

## Preuve : la démo refactorée

Toutes les vues de la todo (`todo_screen`, `settings_screen`, `todo_row`,
`confirm_content`) réécrites avec le DSL : arbre **plus plat, plus lisible**,
moins de bruit (`.child(...)` en cascade → listes `[...]`). Aucune régression
(mêmes tests, chrono et rendu identiques).

## Décision technique (alternatives)

- **Macros + helpers** plutôt qu'un **`Element` généralisé** (accepter
  `.child("chaîne")`). Même gain de lisibilité, **zéro rupture** du trait
  `Widget`. L'`Element` newtype (façon iced) imposerait de changer `children()`
  partout et un enfer de cohérence de traits ; gardé en évolution si le besoin
  se confirme.

## Tests

- `dsl` : `row!`/`column!` produisent le bon nombre d'enfants (dont imbrication
  et rangée vide) ; `button(label, msg)` émet bien le message.
- Non-régression : toute la suite (widgets, shell, demo) + démo à l'écran + chrono.

## Prochaine étape

Roadmap DX-first validée : **J28 reconciliation par clé** (correctness + DX des
listes), puis J29 nav clavier/a11y, J30 robustesse fenêtre, J31 widgets riches.
Le réflexe DX est conservé à chaque jalon.
