# Jalon 1 — Moteur de rendu 2D minimal (primitives)

Transforme le renderer du Jalon 0 (un quad codé en dur) en une **API de
primitives** : on décrit une [`Scene`] de rectangles colorés, le GPU la dessine.

## Ce qui est livré

- **API de dessin** dans `frus-gpu` : `Color`, `Rect`, `Scene::fill_rect`,
  `Renderer::render(&Scene)`.
- **Système de coordonnées** en pixels logiques, origine haut-gauche, Y vers le
  bas (comme CSS/Flutter). Conversion vers NDC faite dans le shader via un
  uniform `viewport`.
- **Rendu instancié** : un quad unité répété pour chaque rectangle, données
  d'instance `{rect, color}` dans un buffer qui grandit au besoin.
- **Alpha-blending** activé.
- **Test de rendu headless** : rend un rectangle rouge dans une texture
  offscreen et vérifie le pixel central — preuve automatique du rendu, sans
  fenêtre.

## Architecture

```
Scene (CPU: Vec<Instance{rect, color}>)
        │ queue.write_buffer
        ▼
 instance_buffer ─┐
 unit_quad (6 v)  ├─► Painter (pipeline) ─► shader:
 viewport uniform ┘        pos_px = rect.xy + quad * rect.wh
                            clip   = pixel_to_ndc(pos_px, viewport)
                            ▼
                         N rectangles à l'écran
```

Découpage des modules `frus-gpu` :

| Module | Rôle |
|---|---|
| `color` | `Color` RGBA |
| `geometry` | `Rect` (pixels logiques) |
| `scene` | `Scene` + `Instance` (données GPU) |
| `painter` | pipeline + buffers, **indépendant de toute surface** (donc testable headless) |
| `renderer` | lie une surface (fenêtre) au `Painter`, présente les frames |

Le `Painter` étant indépendant de la surface, le même chemin de rendu sert à la
fois pour la fenêtre et pour le test offscreen.

## Décisions

- **Rendu instancié** plutôt que tessellation : optimal pour des rectangles,
  simple. La tessellation (formes complexes, courbes) viendra plus tard.
- **Coordonnées pixels** dès maintenant : prérequis du futur moteur de layout.
- **Uniform buffer** pour le viewport (portable) plutôt que push constants (non
  garanties en downlevel/Web).
- **Buffer d'instances à capacité croissante** (`next_power_of_two`) : évite les
  réallocations quand la scène est stable.

## Lancer / tester

```sh
# Fenêtre de démo (3 rectangles) :
bash scripts/wsl-run.sh

# Tests (dont le rendu offscreen) :
#   dans WSL, à la racine :
cargo test
```

## Limites connues (à traiter plus tard)

- Pas encore de coins arrondis, bordures, ni z-order explicite (ordre = ordre
  d'insertion).
- Couleurs transmises telles quelles (gestion sRGB/linéaire remise à un jalon
  colorimétrie).
- Un seul type de primitive (`fill_rect`).
