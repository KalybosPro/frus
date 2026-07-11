# Jalon 33 — Nouveaux widgets : Collapsible, Menu, Chip

Trois widgets de plus (disclosure / actions / étiquettes).

## Widgets

- **`Collapsible::new(titre, open, on_toggle).content(w)`** — section **repliable**
  contrôlée. Composite `[en-tête, contenu?]` : le contenu n'est réalisé que si
  ouvert → son apparition/disparition profite **gratuitement** des fondus de
  montage/démontage. En-tête cliquable (titre + chevron ▸/▾), focusable.
- **`Menu::new(ancre, open, on_dismiss).item(label, msg)`** — menu d'actions
  **flottant** (via `Portal`, placement `Below`). Fermeture au **clic extérieur**.
  Items cliquables focusables.
- **`Chip::new(label).on_remove(msg)`** — étiquette compacte (tag / filtre) avec
  croix de suppression optionnelle (cliquable, focusable).

## Généralisation utile

La **fermeture au clic extérieur** (`overlay_dismiss`) ne concernait que les
modales `Center` ; elle s'applique désormais à **tout overlay** (dont les menus
`Below`) : un hit plein écran, sous le contenu, émet le message de fermeture. Le
voile sombre reste, lui, réservé aux modales.

## Démo (intégration)

- **Menu** « ⋯ » dans l'en-tête (actions : Sauvegarder, Effacer les terminées).
- **Chip** du filtre actif (hors « Toutes »), supprimable → revient à « Toutes ».
- **Collapsible** « Options avancées » dans l'onglet « À propos » des Réglages,
  contenant des `Chip` (« beta », « expérimental »).

## Tests

- `Collapsible` : `[en-tête]` fermé, `[en-tête, contenu]` ouvert ; l'en-tête émet
  la bascule.
- `Menu` : fermé → pas d'overlay ; ouvert → overlay + `overlay_dismiss` ; un clic
  loin de l'ancre émet la fermeture.
- `Chip` : libellé seul, ou libellé + croix cliquable qui émet la suppression.
- 62 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Menu` : largeur fixe, pas de sous-menus ni de séparateurs d'items.
- `Collapsible` : pas d'animation de hauteur (le contenu fond, il ne « glisse » pas).
