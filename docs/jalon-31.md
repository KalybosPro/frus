# Jalon 31 — Liste virtualisée (`List`)

Dernier de la roadmap (widgets riches). Un `Scroll` met en page et peint **tous**
ses enfants à chaque frame ; pour de grandes listes (milliers de lignes) c'est
O(N). La `List` virtualisée ne construit/pose/peint que la **fenêtre visible** :
coût par frame ∝ éléments visibles, pas au total.

## API

```rust
List::new(count, item_height, |index| ligne(index))
    .width(w).height(h)
```

Aussi simple qu'une boucle, mais scalable à des milliers d'éléments.

## Mécanisme

- La `List` est une **zone défilable** : elle réutilise toute la machinerie
  scroll (offset runtime, molette, inertie, barre). Hauteur de contenu =
  `count × item_height` → borne de défilement.
- Plage visible = `[offset/h , (offset+viewport)/h]` ; seuls ces ~N éléments sont
  **construits à la demande** (closure `index → widget`), posés à
  `index×h − offset`, clippés au viewport. Identité par **index** (`id.child(i)`).
- Hook trait `virtual_list(&self) -> Option<VirtualList<'_, Msg>>` (count, hauteur,
  &fabrique) ; `build_ui` le traite comme une branche spéciale (comme `Scroll`).

## Décisions & limites (assumées)

- **Hauteur d'élément fixe** (pas de mesure variable) — hauteurs variables reportées.
- **Rendu via `render_item`** (et non le `walk` principal) : `walk` porte une
  lifetime `'a` pour différer les overlays ; un élément **construit à la volée** ne
  peut la satisfaire. Conséquence : un élément est un **sous-arbre simple** — pas
  d'overlay/scroll/navigator imbriqué, **pas d'état retenu par élément** ni de
  focus clavier (on ne retient pas l'état d'un élément hors écran). Clic/survol des
  éléments **visibles** : OK. C'est le compromis correct d'une virtualisation.
- Refactor DX interne : `full_status` / `draw_focus_ring` factorisés et partagés
  entre le rendu principal et `render_item`.

## Démo

Nouvel écran **Journal** (bouton « Journal → ») : `List::new(5000, 44.0, …)` —
5000 lignes fluides, seules ~une douzaine construites par frame.

## Tests

- `only_visible_items_are_built` : un compteur prouve que sur 5000 éléments,
  seuls ~5–8 sont construits (viewport 200 / item 40).
- `scroll_max_covers_full_content` : `max_y = count×h − viewport` (100×40−200 = 3800).
- `builds_a_scene` : rendu non vide.
- 46 tests frus-widgets ; démo + chrono non régressés.
