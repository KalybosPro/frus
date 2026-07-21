# Jalon 100 — Widgets nommés : `Opacity`, `AnimatedOpacity`, `AnimatedContainer`

## Analyse

J96→99 ont rendu `Container` capable d'animer opacité, couleur, taille et rayon,
mais via des méthodes empilées (`Container::new().animated_color(…).animated_size(…)`).
Flutter expose ces capacités sous des **widgets nommés** — `Opacity`,
`AnimatedOpacity`, `AnimatedContainer` — plus lisibles et découvrables. Ce jalon
les ajoute comme **sucre ergonomique**, sans dupliquer la moindre logique.

## Décisions techniques

- **Wrappers transparents sur un `Container` interne.** Chaque widget nommé
  contient un [`Container`] configuré et lui **délègue tout** (idiome de
  [`crate::Keyed`]). Le `Container` interne **est** le nœud animé ; l'enfant de
  l'utilisateur reste un nœud **séparé** — donc la valeur animée par nœud (scalaire
  d'opacité, couleur, taille, rayon) ne peut pas entrer en collision avec un enfant
  lui-même animé. Les identités (`child_id`) restent alignées sur la marche de
  peinture, donc animations et layout fonctionnent à l'identique.

- **Délégation par macro, en syntaxe qualifiée.** Un `macro_rules!
  forward_to_container!` génère l'impl `Widget` en déléguant exactement les
  méthodes que `Container` surcharge. Subtilité : `Container` a des méthodes
  **inhérentes** de même nom que le trait (`on_click`, `repaint_boundary`… =
  builders) ; on appelle donc le trait en `Widget::…(&self.inner)` pour lever
  l'ambiguïté. `debug_name` n'est **pas** délégué : l'inspecteur affiche le nom du
  widget nommé (`AnimatedContainer`, non `Container`).

- **`AnimatedContainer` : un builder à durée/courbe partagées.**
  `AnimatedContainer::new(duration, curve)` puis `.color()/.size()/.radius()/`
  `.opacity()/.padding()/.child()` — toutes les propriétés animées héritent de la
  même `(durée, courbe)`, cohérent avec le fait qu'une boîte n'a qu'un couple de
  timing. `Opacity::new(o, child)` et `AnimatedOpacity::new(o, dur, curve, child)`
  enveloppent directement un enfant.

## API

```rust
AnimatedContainer::new(0.3, Curve::ease_in_out())
    .color(theme.primary)
    .size(200.0, 100.0)
    .radius(12.0)
    .child(Text::new("hi"))

Opacity::new(0.5, child)
AnimatedOpacity::new(0.0, 0.2, Curve::ease_in(), child)
```

## Implémentation

- `frus-widgets` : nouveau `animated.rs` (`Opacity`, `AnimatedOpacity`,
  `AnimatedContainer` + macro de délégation) ; re-exports dans `lib.rs`. Aucune
  logique d'animation nouvelle — pur sucre sur les capacités de `Container`
  (J96→99).

## Tests

- `animated_container_declares_all_targets` : couleur/taille/rayon/opacité + durée
  + courbe correctement exposés via le trait (donc pris par les `advance_*`).
- `opacity_wraps_child_as_a_group` : groupe d'opacité fixe, enfant = **nœud
  séparé** (pas de collision).
- `animated_opacity_declares_a_group_target` : opacité animée + `debug_name`
  propre (`"AnimatedOpacity"`).
- Suite complète verte (widgets 191, +3).

## Bilan

`AnimatedContainer` de Flutter est désormais porté **de bout en bout** : les
capacités (opacité J96, couleur J97, taille J98, rayon J99) et l'**API nommée**
(J100), sur l'infrastructure de timeline courbée de J95.

## Reste

- Padding/marge animés (injection au layout comme la taille).
- `Tween` typés génériques ; animations **explicites** (contrôleur piloté).
- `alignment`/`decoration` statiques sur `AnimatedContainer` (parité Flutter).
