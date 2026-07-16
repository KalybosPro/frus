# Jalon 59 — Généralisation des state-layers Material

Le jalon 58 a introduit `Theme::state_layer` (la règle d'états bakée) et l'a adoptée
dans `Button`. Ce jalon **propage** cette règle aux widgets qui réinventaient
chacun leur survol avec un pourcentage arbitraire (`surface.lerp(on_surface, 0.05..
0.08 · hover)`), sans réponse au focus ni à la pression.

## Widgets migrés

Six surfaces de survol passent à `theme.state_layer(theme.surface, theme.on_surface,
&status)` :

- **menu** (ligne d'action) — était 7 %
- **dropdown** (en-tête) — était 6 %
- **collapsible** (en-tête) — était 5 %
- **autocomplete** (champ) — était 7 %
- **datepicker** (cellule de jour non sélectionnée) — était 8 %
- **tree** (ligne cliquable, sous sa garde de survol) — était 5 %

Bénéfice : un **survol unifié à 8 %**, et surtout l'ajout **gratuit** des réponses
**focus (10 %)** et **pression (12 %)** partout, là où ces widgets n'y réagissaient
pas. Au repos (`hover_progress = 0`, pas de focus/pression), `state_layer` renvoie la
couleur de base **inchangée** — l'apparence au repos est identique, donc aucune
régression.

## Laissés tels quels (sémantique différente)

- **breadcrumb**, **chip** : interpolent une **couleur de texte** (`muted →
  on_surface`) au survol, pas un fond — ce n'est pas une state-layer.
- **switch** : couleur de piste selon la **valeur** (position), pas l'interaction.
- **navrail** : pastille de survol/sélection dessinée en `fill_rect` teinté — une
  migration ultérieure possible, mais structurée autrement.

## Validation

- `frus-widgets` **130 tests**, `frus-demo` **15**, verts — l'apparence au repos est
  inchangée (state-layer neutre à l'état de repos), donc les tests de structure/rendu
  passent sans modification.
- `cargo build --workspace` sans avertissement.

## Suite

- **navrail** et autres survols structurés (pastilles) à unifier au fil de l'eau.
- Système de **typographie** (`TextStyle`/`TextSpan`/`TextTheme`) — l'autre moitié
  d'un défaut premium (jalon suivant).
