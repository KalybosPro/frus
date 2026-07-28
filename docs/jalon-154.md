# Jalon 154 — Autocomplétion : liste de suggestions défilante

## Analyse

La liste flottante de l'`Autocomplete` (jalons 150–152) s'**étirait sans fin** : une
requête à nombreuses correspondances produisait une liste haute comme l'écran, débordant
sous l'ancre. Il fallait **borner** la hauteur visible et laisser le reste **défiler**,
comme le menu d'un `DropdownButton` ou l'`Autocomplete` Material (fenêtre ~5–6 options).

## Décisions techniques

- **Seuil `max_visible`, sinon liste nue.** `Scroll` a un viewport à **hauteur fixe** (pas
  de « max-height »). Plutôt que d'imposer un défilement systématique (barre inutile sur 2
  suggestions), le widget n'emballe la liste dans un `Scroll` **que** si le nombre de
  suggestions **dépasse** `max_visible` ; en deçà, il pousse la liste `Flex` directe
  (comportement d'origine, aucune régression). Viewport = `n·ROW_H + (n−1)·écart`.

- **Réutilise `Scroll` tel quel.** Aucun changement du conteneur défilable : molette /
  tactile / barre fonctionnent déjà dans l'overlay (le `Scroll` s'enregistre à la marche
  de construction, y compris sous le portail flottant). Les suggestions restent
  **focusables** ; le focus clavier révèle la suggestion visée dans le viewport.

- **Contrôlé et opt-in.** `max_visible` par défaut `None` (illimité). L'application choisit
  la fenêtre ; l'état de défilement est retenu au runtime par identité, comme tout `Scroll`.

## Implémentation

- `autocomplete.rs` : champ `max_visible: Option<usize>` + `.max_visible(n)` ; `rebuild`
  emballe la liste dans `Scroll::new().width(w).height(viewport)` au-delà du seuil, sinon
  pousse la liste directe ; constante `ROW_GAP`.
- `goldens.rs` : golden `autocomplete_scroll` (6 suggestions, `max_visible(3)`).

## Vérification

- **Unitaire** : au-delà du seuil, l'overlay est un `Scroll` dont la hauteur = 2 lignes
  (`max_visible(2)`, 4 suggestions) et qui contient bien les **4** suggestions ; en deçà
  (`max_visible(5)`, 2 suggestions), l'overlay reste la **liste nue** (2 enfants, 1ʳᵉ =
  `Pick("a1")`). Tests J150–152 inchangés.
- **Golden** `autocomplete_scroll` **inspecté** : viewport de 3 lignes (Alabama / Alaska /
  Arizona), **barre de défilement** à droite (≈ moitié → 6 éléments), « a » mis en avant.
- `cargo test --workspace` **vert**.

## Reste

- **Auto-défilement sur la suggestion active** : révéler `active` dans le viewport quand
  l'app la fait avancer au clavier (aujourd'hui seul le **focus** réel est révélé).
- **Hauteur adaptative** : borner par pixels (`max_height`) plutôt que par nombre de
  lignes, utile si les suggestions deviennent des widgets de hauteur variable.
