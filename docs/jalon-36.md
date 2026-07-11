# Jalon 36 — Nouveaux widgets : Table, SegmentedControl, Toast

Trois widgets, dont un bâti sur la grille du jalon précédent.

## Widgets

- **`Table::new(colonnes).header(&["Nom","Note"]).row(&["Ada","5"])`** — tableau de
  données **texte**, bâti sur `Grid` (colonnes égales). En-tête stylé (fond léger,
  texte discret) + cellules ; délègue son layout au `Grid`. Cellules riches →
  utiliser `Grid` directement.
- **`SegmentedControl::new(sel, on_select).segment("Jour").segment("Semaine")`** —
  sélecteur segmenté **contrôlé** (boutons connectés, actif en avant). Cliquer le
  i-ᵉ émet `on_select(i)`.
- **`Toast::new("Enregistré").success()`** — notification transitoire (carte +
  barre d'accent selon la variante Info/Succès/Erreur). Le *widget* est passif ;
  le *système* (minuterie, empilement) est du ressort de l'app.

## Démo — vitrine du modèle `update → Command`

- **`SegmentedControl`** remplace les trois boutons de filtre de la liste.
- **`Toast`** « Sauvegardé » s'affiche en bas-centre au clic sur **Sauvegarder**
  (couche `Stack`), puis **s'auto-ferme après 2 s** via un `Command` minuté
  (`Command::perform(|| { sleep(2s); DismissToast })`) — démonstration d'un effet
  temporisé.
- **`Table`** de métriques (Widgets / Jalons) dans l'onglet « À propos ».

## Tests

- `Table` : `colonnes × (1 en-tête + N lignes)` cellules ; textes peints.
- `SegmentedControl` : N segments ; clic i-ᵉ → `on_select(i)`.
- `Toast` : carte + barre d'accent (couleur de variante) + texte.
- 72 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Table` : cellules **texte** ; pas de tri, sélection de ligne, ni largeurs de
  colonnes variables (héritées des limites de `Grid`).
- `Toast` : pas de file/empilement intégré (géré par l'app) ni d'animation
  d'entrée/sortie dédiée (le fondu de montage/démontage s'applique).
