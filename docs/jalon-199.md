# Jalon 199 — Charts : graphique à barres

## Analyse

Le domaine **graphes** était vierge : aucune façon de visualiser des données chiffrées (statistiques,
séries temporelles…). Première brique : un graphique à **barres**, simple et thémé.

## Décisions techniques

- **Vue auto-peinte, pilotée par les données.** `BarChart` prend une série `(libellé, valeur)` et
  la peint lui-même (barres, valeurs, libellés, ligne de base) — aucun enfant, non générique sur
  `Msg` (façon [`Icon`](../crates/frus-widgets/src/icon.rs)) : c'est une **vue**, pas un contrôle.
  Les barres sont mises à l'échelle de la **valeur maximale** (bornée à 1 pour une échelle stable
  même à valeurs nulles).

- **Thémé et personnalisable.** Barres en `primary` du thème (surchargé par `color`), valeurs en
  `on_surface`, libellés en `muted`, ligne de base en `border`. `height` règle la hauteur.

- **Remplit la largeur.** `width: Percent(1.0)` (le graphique occupe la largeur offerte) : le
  parent doit donc avoir une **largeur définie** (sinon la largeur `Percent` s'effondre à 0 — piège
  de mise en page rencontré et corrigé dans le golden en fixant la largeur du conteneur).

- **Formatage des valeurs** : entier si la valeur l'est, sinon une décimale.

## Implémentation

- `chart.rs` : `BarChart` (`new` / `color` / `height`) ; `impl<Msg> Widget` (auto-peint) ;
  helpers `max_value` / `format_value`.
- `lib.rs` : `mod chart` + `pub use chart::BarChart`.
- `goldens.rs` : `bar_chart` (série Mon–Fri).

## Vérification

- **Unitaire** : `value_formatting` (3.0 → « 3 », 2.5 → « 2.5 ») ; `empty_series_paints_nothing` ;
  `bars_scale_to_the_max_value` (une barre par valeur, la plus grande valeur → la plus haute
  barre, proportionnelle ; valeurs et libellés dessinés).
- **Golden** `bar_chart` **inspecté** : cinq barres proportionnelles (max = Thu 8), valeurs
  au-dessus, libellés dessous, ligne de base.
- `cargo test -p frus-widgets chart::` **vert**.

## Reste

- **Graphique en lignes** (polyligne) — demande un tracé de trait (segments) ; les rects suffisent
  aux barres, pas aux lignes.
- **Axe des ordonnées / grille** (graduations, valeurs de repère) et **barres empilées / groupées**.
- **Interaction** (survol d'une barre → infobulle de valeur) — via l'état de survol existant.
