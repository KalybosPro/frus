# Jalon 37 — Nouveaux widgets : Breadcrumb, Pagination, Skeleton

Trois widgets, chacun mettant en valeur un aspect différent du framework.

## Widgets

- **`Breadcrumb::new(on_select).crumb("Accueil").crumb("Réglages")`** — fil
  d'Ariane : segments cliquables séparés par « › » ; le **dernier** est la page
  courante (mise en avant, non cliquable). Cliquer le i-ᵉ émet `on_select(i)`.
- **`Pagination::new(courante, total, on_select)`** — sélecteur de page (pages
  **1-indexées**) : ‹ préc. · **fenêtre** de pages autour de la courante · suiv. ›.
  Préc./suiv. **désactivés** aux bornes. Cliquer une page émet `on_select(p)`.
- **`Skeleton::new().width(w).height(h)`** — placeholder de chargement dont
  l'intensité **pulse dans le temps** (shimmer). Réutilise l'horloge continue
  (`Status::time` + `continuous()`) : le framework redessine tout seul.

## Démo

- **`Breadcrumb`** « Accueil › Réglages » en tête de l'écran Réglages (cliquer
  « Accueil » dépile).
- **`Pagination`** (page contrôlée) et deux **`Skeleton`** animés dans l'onglet
  « À propos ».

## Tests

- `Breadcrumb` : `N` segments + `N−1` séparateurs ; liens cliquables, courant non
  cliquable.
- `Pagination` : fenêtre correcte (courante ±2) ; préc./suiv. bornés (désactivés
  en première/dernière page).
- `Skeleton` : `continuous() == true` ; l'opacité peinte **dépend du temps**.
- 77 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Pagination` : fenêtre simple (pas d'ellipses « 1 … 5 … 20 »).
- `Skeleton` : pulsation d'opacité (pas de dégradé glissant, qui exigerait un
  shader dédié).
- `Breadcrumb` : pas de troncature/`…` pour les chemins très longs.
