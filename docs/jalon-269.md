# Jalon 269 — `compute_scroll` **remplit l'axe contraint** (fin du conteneur remplisseur)

## Objectif

Au jalon 266, faire remplir aux colonnes Kanban la hauteur du board imposait à l'app un contournement :
envelopper le `Scroll` horizontal dans un `Flex` `flex(1)` (un simple `Container` à hauteur `Auto`
**s'effondrait**). La cause : le **contenu** d'un défilable ne remplissait pas le viewport sur son axe
**transverse** (contraint) — il se calait sur son contenu, privant tout enfant `flex(1)`/`Percent` de
base définie. Ce jalon corrige la **racine** : `compute_scroll` remplit désormais l'axe contraint,
façon `ListView` de Flutter (contrainte transverse serrée). Les apps n'ont plus besoin du remplisseur.

## Le correctif (`frus-layout/src/tree.rs`)

Dans `compute_scroll`, avant le calcul : si le défilement est **mono-axe** (un axe libre, l'autre
contraint) et que la dimension **racine** sur l'axe contraint est `Auto`, on la fixe à la taille du
viewport (`Length`). Le contenu prend donc la taille transverse du viewport ; l'axe **libre** (celui du
défilement) garde sa taille naturelle (`MaxContent`).

Garde-fous de portée :

- **Mono-axe seulement** : `fill_w = !free_x && free_y` (défilement vertical → remplit la largeur) ;
  `fill_h = !free_y && free_x` (horizontal → remplit la hauteur). La **mise en page définie** (deux
  axes contraints, `Constraints::definite`) et le **défilement 2D** (deux axes libres) ne sont **pas**
  touchés — pas de régression des écrans/fenêtre/modales ni des tables défilables en X **et** Y.
- **`Auto` seulement** : une dimension **explicite** (`Length`/`Percent`) de la racine du contenu est
  **respectée**.

## Simplification côté app

- **`frus-demo`** (`board_screen`) : le board revient à un **simple** `Container::new().padding(24).
  child(board)` dans le `Scroll` horizontal — la structure d'avant le jalon 266, qui s'effondrait, et
  qui **fonctionne** désormais. Le contournement `Flex` `flex(1)` est retiré.
- `Kanban::scrollable_columns()` garde son `height: Percent(1.0)` (il remplit la zone de contenu du
  `Container`, lui-même rempli par `compute_scroll`).

## Vérification

- **Desktop** : `frus-layout` 4, `frus-widgets` 396, `frus-shell` 27, `frus-demo` 36, goldens **77** —
  tous verts (aucune régression : la mise en page définie et le défilement 2D sont exclus du
  remplissage). Le garde-fou `scrollable_columns_fill_the_board_height_then_scroll` passe désormais
  avec un **`Container`** (le cas même qui s'effondrait au jalon 266), preuve du correctif.
- **Appareil** : APK `frus-demo` reconstruit — à confirmer que les colonnes remplissent toujours la
  hauteur et défilent (structure app simplifiée, résultat identique).

## Reste

- RAS. Le remplissage transverse par défaut rapproche `Scroll` du `ListView`/`SingleChildScrollView`
  de Flutter (contrainte transverse serrée, axe de défilement libre).
