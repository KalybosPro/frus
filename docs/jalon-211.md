# Jalon 211 — Charts : infobulle de sous-région au survol

## Analyse

Le jalon 208 a plombé la position du pointeur (`Status::hover_cursor`) et le halo du suffixe. La même
infrastructure permet le geste attendu d'un graphe : **survoler** pour lire la valeur exacte sous le
pointeur. C'était l'objectif final annoncé — réutiliser `hover_cursor` pour une infobulle.

## Décisions techniques

- **Réutilise `cursor_icon` (jalon 205) comme déclencheur.** `LineChart::cursor_icon` renvoie
  `Some(Cursor::Default)` quand le pointeur est dans la **zone de tracé** — sans changer la forme du
  curseur (un graphe n'est pas cliquable), mais en amenant le shell à poser `hover_cursor`. Hors
  zone : `None`. Aucune machinerie nouvelle.

- **Infobulle multi-séries.** Au survol, `paint` trouve la **catégorie la plus proche** en x, trace
  un guide vertical, accentue le marqueur de chaque série à cette catégorie, et dessine une boîte
  listant la catégorie puis, par série, sa pastille + sa valeur. En série unique, la ligne se réduit
  à la valeur.

- **Boîte auto-placée.** Dimensionnée au plus long libellé, posée à droite du guide et repliée à
  gauche si elle déborderait, ancrée en haut de la zone — jamais hors cadre.

- **Sans coût au repos.** `hover_cursor` reste `None` tant que le pointeur n'est pas sur la zone de
  tracé : pas de repaint, les goldens (rendus sans survol) sont inchangés.

## Implémentation

- `frus-widgets/src/chart.rs` : `LineChart::cursor_icon` (suivi sur la zone de tracé) ; bloc
  infobulle en fin de `paint` (guide + marqueurs accentués + boîte) ; constante `TOOLTIP_SIZE`.

## Vérification

- `hovering_the_plot_shows_a_tooltip_guide` : un guide vertical apparaît quand `hover_cursor` est
  sur la zone, aucun sans survol ; `cursor_icon` répond `Some(Default)` dans la zone, `None`
  au-dessus. (L'infobulle n'existant qu'au survol, elle n'est pas *goldenable* via `render_widget` ;
  couverte par ce test unitaire.)

## Reste

- Même infobulle pour la **BarChart** (barre sous le pointeur), suivre le point le plus proche en
  distance 2D (pas seulement en x), et une **transition** d'apparition/disparition du survol.
