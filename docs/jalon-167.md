# Jalon 167 — Accessibilité : annonces de tri et de sélection

## Analyse

Le jalon 165 a doté frus d'une **région live** (annonces vocales), branchée au seul
réordonnancement de colonnes. Or deux autres gestes du tableau changent l'état sans le
dire à un utilisateur de lecteur d'écran : **trier** une colonne et **cocher** une (ou
toutes les) ligne(s). Il fallait les énoncer aussi — et, plus généralement, offrir un
**point d'accroche réutilisable** pour qu'un widget déclare ce qu'énoncer à son activation.

## Décisions techniques

- **Un point d'accroche générique : `Widget::announce()`.** Nouvelle méthode de trait
  (défaut `None`), retournant le texte à énoncer **quand le widget est activé** (clic souris
  **ou** Entrée/Espace). Elle décrit l'effet **résultant** — pas l'état courant — pour
  coller à ce que l'utilisateur veut entendre. Forwardée par `Box<dyn Widget>`, `Keyed`,
  `Responsive`, comme les autres méthodes.

- **Le shell lit `announce()` aux deux activations.** À la **validation d'un clic**
  (`pointer_up`, press == release) et à l'**activation clavier** (Entrée/Espace), le shell
  lit `announce()` du widget **avant** `dispatch` (qui reconstruit l'arbre) et le pousse via
  `set_announcement` (le mécanisme live du jalon 165).

- **Le tableau prédit l'effet.** L'en-tête triable énonce « Sorted by {label}
  {ascending|descending} » en **basculant** le sens courant (croissant par défaut — schéma
  Material usuel). La case à cocher énonce l'état **résultant** de sa bascule : « All rows
  selected/deselected » (case d'en-tête) ou « Row selected/deselected » (ligne).

## Implémentation

- `widget.rs` : `fn announce(&self) -> Option<String>` (défaut `None`) + forwarders
  (`Box`, `keyed.rs`, `responsive.rs`).
- `table.rs` : `Cell::announce` (tri résultant), `CheckCell::announce` (sélection résultante).
- `app.rs` : lecture de `announce()` et `set_announcement` aux chemins clic souris et
  Entrée/Espace.

## Vérification

- **Unitaire** : `sort_and_selection_are_announced` — en-tête non trié → « Sorted by Name
  ascending » ; déjà croissant → « descending » ; « tout cocher » partiel → « All rows
  selected » ; ligne cochée → « Row deselected », décochée → « Row selected ».
- `cargo test --workspace` **vert**.

## Reste

- **Sélection de ligne au clic** (hors case à cocher) : non annoncée — la cellule de donnée
  n'a ni l'identité de ligne ni l'état résultant. À câbler si l'app expose ces éléments.
- **Prédiction du tri** : suppose le cycle croissant/décroissant ; une app à cycle
  croissant/décroissant/aucun énoncerait un sens en avance d'un cran au 3ᵉ clic.
