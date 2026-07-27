# Jalon 150 — Audit `Dropdown` / `Autocomplete` (niveau Flutter)

## Analyse

Les deux widgets fonctionnaient mais restaient en deçà de leurs équivalents Flutter :

- **`Dropdown`** : largeur **codée en dur** (240 px), option sélectionnée **non indiquée**
  (ni surlignage ni coche), en-tête et options **non focusables** (pas de clavier), et
  **aucun test**.
- **`Autocomplete`** : largeur codée en dur (260 px) — non « customizable ».

## Décisions techniques

- **`Dropdown` reconstruit son arbre.** Il stocke désormais son état (libellé, largeur,
  index sélectionné, ouverture, options) et régénère en-tête + menu (`rebuild`), ce qui
  ouvre les réglages `width(px)` et `selected(index)` sans casser l'API.

- **Option sélectionnée à la Flutter.** Dans le menu, l'option d'index `selected` est
  **surlignée** (fond teinté `primary`) et **cochée** (icône `Check` à droite) — comme le
  `DropdownButton`. Le chevron de l'en-tête devient un **triangle vectoriel** (plus de
  dépendance au caractère « ▾ »).

- **Clavier gratuit.** En-tête et options renvoient `focusable` quand elles portent un
  message : le shell les atteint au Tab, ouvre / choisit à Entrée, et les flèches
  parcourent les options empilées (navigation géométrique existante). Aucune logique
  nouvelle.

- **`Autocomplete` : largeur réglable.** Le champ étant reconstruit à chaque réglage, son
  rappel `on_input` devient **partagé** (`Rc`) pour être recapturé par le `TextInput`
  reconstruit ; `width(px)` s'applique au champ **et** aux suggestions. Les suggestions
  étaient déjà focusables (clavier OK).

## Implémentation

- `dropdown.rs` : `Row` gagne `width`/`selected`/`focusable` (surlignage + coche + chevron
  vectoriel) ; `Dropdown` stocke son état et `rebuild` ; `width`, `selected` ; **3 tests**
  (overlay ouvert/fermé, focusabilité, option surlignée+cochée) — le module n'en avait aucun.
- `autocomplete.rs` : rappels en `Rc`, champ reconstruit ; `width(px)` ; `Suggestion`
  gagne `width`.
- `goldens.rs` : golden `dropdown_menu` (menu ouvert, « Medium » surligné + coché).

## Vérification

- **Unitaire** : `Dropdown` ferme → pas d'overlay, ouvert → menu à 2 options dont la 2ᵉ
  émet `Select(1)` ; en-tête + options focusables ; option sélectionnée surlignée + cochée
  (chemin + rect teinté). `Autocomplete` : tests existants verts avec la largeur réglable.
- **Golden** `dropdown_menu` rendu et **inspecté** (en-tête + chevron, menu flottant,
  « Medium » vert coché). `cargo test --workspace` vert.

## Reste

- **`Autocomplete`** : suggestion **active** surlignée + descente clavier depuis le champ
  (flèche bas) façon Material ; **surbrillance du texte** correspondant.
- **`Dropdown`** : ouverture/fermeture au clavier gérées par l'app (message de bascule) —
  un raccourci Échap pour refermer serait un plus.
