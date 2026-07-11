# Jalon 30 — Robustesse fenêtre

Comble des angles morts de plateforme et laisse l'app **déclarer sa fenêtre**.

## Ce qui change

- **Garde taille nulle** : si la fenêtre est minimisée (`inner_size == 0`), on
  **saute** le rendu (évite les erreurs GPU et le travail inutile) et on remet
  `last_frame = None` pour ne pas provoquer un saut de `dt` à la restauration.
- **Occlusion** : sur `WindowEvent::Occluded(true)` le rendu est **suspendu** ;
  sur `false` on redemande une frame. Économise le GPU quand la fenêtre est cachée.
- **`ScaleFactorChanged`** : met à jour l'échelle **et reconfigure la surface** à
  la taille physique courante (plus seulement l'échelle).
- **Taille minimale** : la fenêtre est créée avec `min_inner_size` = 360×280 px
  logiques (évite une UI absurde).
- **DX fenêtre** : nouveau `Application::window_size() -> Option<(f32, f32)>`
  (taille **logique** initiale déclarée par l'app).

## Trait

```rust
fn window_size(&self) -> Option<(f32, f32)> { None }   // taille initiale logique
```

La démo déclare `Some((900.0, 680.0))`.

## Tests — honnêteté

Ce sont des **gardes d'événements winit**, non unit-testables sans fenêtre réelle.
Validation = **compilation + démo sans crash** + non-régression (chrono, 43 tests
inchangés). Pas de nouveau test unitaire ici, contrairement aux autres jalons —
c'est de la plomberie de plateforme (assumé).

## Limites (v1)

- Pas de gestion fine multi-écran / déplacement inter-moniteurs.
- `min_inner_size` fixe (non configurable par l'app).
