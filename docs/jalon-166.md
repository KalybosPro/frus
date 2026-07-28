# Jalon 166 — Tableau : hauteur de rangée adaptative

## Analyse

Chaque cellule imposait une hauteur **fixe** (`ROW_H = 34`). Depuis les cellules-widgets
(jalon 164), une cellule peut contenir un contenu plus **haut** (grand avatar, puce
volumineuse, bouton, à terme du texte multi-ligne) — qui était alors **rogné** à 34 px.
Il fallait des rangées qui **grandissent avec leur contenu**, tout en gardant une hauteur
de **confort** minimale pour les rangées courtes.

## Décisions techniques

- **Une contrainte de minimum dans la mise en page.** Plutôt qu'un bricolage local au
  tableau, on ajoute la primitive manquante et générale : `Style.min_width` /
  `Style.min_height` (traduites vers `min_size` de taffy), façon `ConstrainedBox` de
  Flutter. Une boîte peut désormais grandir avec son contenu **sans jamais se tasser** sous
  un plancher.

- **Cellule = hauteur `Auto`, plancher `ROW_H`.** La cellule passe de `height: Length(ROW_H)`
  à `height: Auto` + `min_height: ROW_H`. Comme la rangée aligne ses cellules en `Stretch`
  (défaut), **toutes suivent la plus haute** : le contenu haut d'une seule cellule étire
  toute la rangée, sans rognage, et les rangées courtes gardent leurs 34 px.

- **Peinture centrée sur la hauteur réelle.** Le texte, le triangle de tri et la case à
  cocher se centraient sur la constante `ROW_H` ; ils utilisent désormais `bounds.height`
  (la hauteur **effective** de la cellule) — centrage correct quelle que soit la rangée.

## Implémentation

- `frus-layout/style.rs` : champs `min_width` / `min_height` (défaut `Auto`), pris en
  compte dans `to_taffy` (`min_size`), `layout_hash` et `Default`.
- `frus-widgets/flex.rs` : littéral `Style` complété (`..Default::default()`).
- `frus-widgets/table.rs` : `cell_style` (hauteur `Auto` + `min_height: ROW_H`) ; `Cell` et
  `CheckCell` centrent sur `bounds.height`.
- `goldens.rs` : `table_adaptive_rows` (grand avatar 48 px vs rangée texte).

## Vérification

- **Unitaire** : `widget_row_grows_to_tall_content` — un widget de 60 px dans une cellule
  est peint à **sa pleine hauteur** (la rangée l'a suivi), bien au-delà de `ROW_H`.
- **Golden** `table_adaptive_rows` **inspecté** : la rangée au grand avatar grandit
  (puce « admin » centrée dedans), la rangée texte « Bo/editor » garde sa hauteur — aucun
  rognage. Les goldens texte / avatars 26 px **inchangés** (aucune régression).
- `cargo test --workspace` **vert**.

## Reste

- **Redimensionnement + rangées hautes** : le calque de poignées se cale sur `n × ROW_H`
  (exact pour un tableau texte — le seul cas où toutes les colonnes sont fixes) ; une
  rangée-widget plus haute le sous-estimerait. Combinaison de niche, à traiter si besoin
  (hauteur mesurée après mise en page).
- **Tri de colonnes-widgets** : la clé de tri reste fournie par l'application (le tableau
  ne compare pas des widgets) — déjà possible, à documenter côté guide.
