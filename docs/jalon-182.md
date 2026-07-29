# Jalon 182 — Formulaire multi-étapes : indicateur `Steps`

## Analyse

Un formulaire long se découpe en **étapes** (assistant / wizard) : compte, profil, revue…
L'utilisateur a besoin de savoir **où il en est** — quelles étapes sont faites, laquelle est en
cours, lesquelles restent. C'est le rôle du `Stepper` de Material : une rangée de marqueurs
numérotés reliés, chacun terminé / courant / à venir. Il manquait à frus.

Le nom `Stepper` est **déjà pris** par le sélecteur numérique −/valeur/+ (jalon antérieur) ; le
nouvel indicateur s'appelle donc **`Steps`**.

## Décisions techniques

- **Widget purement visuel, auto-peint.** `Steps` n'a **pas d'enfants** : il peint lui-même les
  marqueurs, connecteurs, numéros/coches et libellés dans ses `bounds` (marqueurs répartis d'un
  bord à l'autre, `center_x`). Pas d'état interne — l'étape courante est un simple `usize`
  fourni par l'application. Simple, déterministe, testable au pixel.

- **Trois états lisibles.** *Terminé* (`i < current`) : rond **accent** + **coche** (icône
  `Check` 16 px). *Courant* (`i == current`) : rond accent + **numéro** clair. *À venir* : rond
  **surface bordé** + numéro atténué. Les connecteurs **franchis** (avant l'étape courante)
  prennent l'accent, les autres la couleur de bord. Libellé sous chaque marqueur, atténué hors
  étape courante. C'est exactement la grammaire visuelle du `Stepper` de Material.

- **Navigation & validation = applicatives.** `Steps` n'orchestre rien : l'application tient
  l'étape courante, câble des boutons Précédent/Suivant, et valide **par étape** avec un
  [`Form`](../crates/frus-widgets/src/form.rs) (jalons 180–181) — récapitulatif final via
  `ErrorSummary`. Le widget reste une **vue** de la progression, pas une machine à états.

- **Personnalisable.** `current(i)` (borné au dernier index), `color(c)` surcharge l'accent
  (marqueurs terminés/courant + connecteurs franchis) ; sinon `primary` du thème.

## Implémentation

- `steps.rs` : `Steps { labels, current, color }` ; builders `new` / `current` / `color` ;
  `impl<Msg> Widget<Msg>` (non générique, comme `Icon`) — `style` pleine largeur × hauteur fixe,
  `paint` connecteurs puis marqueurs (coche `fill_path` ou numéro `text`) puis libellés.
- `lib.rs` : `mod steps;` + `pub use steps::Steps;`.
- `goldens.rs` : `form_wizard` (indicateur 2/3 + contenu d'étape + barre Précédent/Suivant).

## Vérification

- **Unitaire** : `current_is_clamped_to_last` (index débordant → dernier ; liste vide → 0) ;
  `markers_reflect_progress` (2 étapes terminées → 2 coches et pas de numéros « 1 »/« 2 » ;
  courante → « 3 » ; à venir → « 4 » ; tous les libellés dessinés).
- **Golden** `form_wizard` **inspecté** : « Account » cochée (accent), connecteur franchi vert
  vers « Profile » courante (« 2 »), connecteur gris vers « Review » à venir (« 3 » bordé),
  libellés dessous, puis titre d'étape, champ et boutons Back/Next.
- `cargo test -p frus-widgets steps::` **vert**.

## Reste

- **Marqueurs cliquables** (`on_tap(|usize| Msg)`) pour sauter à une étape déjà visitée :
  nécessiterait des marqueurs enfants (hit-test par marqueur) — extension.
- **Orientation verticale** (étapes empilées avec contenu déroulant sous l'étape courante,
  autre forme du `Stepper` de Material) — extension.
