# Jalon 11 — Animations (transitions implicites)

Introduit l'horloge de frames, un état animé retenu par identité, et le redraw
continu tant qu'une animation tourne.

## Ce qui est livré

- **`Color::lerp`** (frus-core) : interpolation de couleurs.
- **`Runtime.anims: HashMap<WidgetId, f32>`** : une progression `0..1` par widget,
  avec `advance_hover(dt) -> bool` (glisse vers la cible — 1 si survolé, 0 sinon —
  et signale si une animation est encore en cours).
- **`Status.hover_progress`** : lu par `Container`, qui interpole
  `base.lerp(hover, ease(progress))` (easing **smoothstep**).
- **Boucle animée** (shell) : horloge `Instant`, `dt` clampé, et redraw redemandé
  tant que `advance_hover` renvoie vrai (cadencé par le present mode Fifo/vsync).

## Modèle

Transition **implicite**, façon CSS : quand l'état de survol change, la
progression du widget glisse dans le temps vers sa cible. L'état est retenu
**par identité** (`WidgetId`) dans `Runtime` — la même infrastructure que le
focus (J8), le scroll (J9) et le curseur (J10).

```
RedrawRequested :
  dt = clamp(now - last_frame)
  animating = runtime.advance_hover(dt)     // met à jour anims[id]
  ui = build_ui(&tree, size, &runtime)      // anims -> Status.hover_progress
  render(ui)
  si animating -> request_redraw            // continue la boucle
```

## Démo

La couleur de survol du bouton **transitionne en fondu** (~120 ms) au lieu de
basculer d'un coup ; le pressé reste instantané.

## Tests

- `Color::lerp` : mi-chemin exact ; bornes.
- `Runtime::advance_hover` : la progression monte vers 1 (survolé), se stabilise
  (plus d'animation), redescend à 0 puis l'entrée est nettoyée.
- `Container` : progression 0 → couleur de base ; 1 → couleur de survol.

## Limites (prochains jalons)

- Une seule progression par widget (survol). Pas encore d'animations
  d'apparition/disparition (montage/démontage), de courbes personnalisées, ni
  d'animations pilotées explicitement (contrôleur 0→1).
- Focus et pressé restent instantanés.
