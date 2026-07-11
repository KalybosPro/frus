# Jalon 43 — Layout adaptatif (navigation & maître-détail)

Deuxième étage de la responsivité : au-delà des primitives (jalon 42), des
**structures d'écran qui changent de forme** selon la [`SizeClass`].

## Lot A — `NavRail` + `BottomBar`

Les deux présentations d'une navigation principale à sélection unique, même API
`new(selected, on_select).item(icon, label)` (l'« icône » est un glyphe texte) :

- `BottomBar` — barre horizontale en bas (téléphone), items à largeur partagée.
- `NavRail` — rail vertical à gauche (tablette/bureau), items à largeur fixe.

Un leaf interne `NavItem` peint la pastille de sélection (fond `primary`), le
glyphe et le libellé, centrés ; survol et sélection thémés au paint.

## Lot B — `NavScaffold` (l'ossature adaptative)

```rust
NavScaffold::new(size_class, selected, on_select)
    .destination(icon, label)…
    .body(content)
```

Choisit **automatiquement** la présentation selon la classe : **BottomBar** en
Compact (corps au-dessus, barre en bas — colonne), **NavRail** en Medium/Expanded
(rail à gauche, corps à droite — rangée). Le `NavScaffold` **est** lui-même le
conteneur flex ; le corps est enveloppé dans un panneau `flex(1)` qui remplit le
reste. `body()` finalise (un seul bras construit la navigation, donc `on_select`
n'est déplacé qu'une fois).

## Lot C — `TwoPane` (maître-détail)

```rust
TwoPane::new(size_class).ratio(0.36).show_detail(flag).list(a).detail(b)
```

**Côte à côte** en Expanded (largeurs proportionnelles via `flex_grow` =
`ratio` / `1 - ratio`), **panneau unique** sinon (la liste, ou le détail si
`show_detail` — l'app le passe à `true` en « naviguant »). `detail()` finalise.

## Infrastructure

Nouvelle impl `Widget for Box<dyn Widget<Msg>>` (délègue tout) : permet de
composer un widget **déjà boxé** là où un `impl Widget` est attendu (p. ex.
`Flex::child`) — indispensable pour envelopper les panneaux du `TwoPane`.

## Démo

L'accueil passe sous un `NavScaffold` (destinations **Tasks / Stats / About**) :
rail à gauche en grand, barre en bas en étroit. La section **Stats** est un
`TwoPane` maître-détail (liste de métriques | panneau de détail), côte à côte en
grand, panneau unique avec retour en étroit.

## Tests

`NavRail`/`BottomBar` (émission d'index, sélection, flexibilité des items),
`NavScaffold` (colonne+barre en Compact, rangée+rail en Expanded), `TwoPane`
(deux panneaux proportionnels en Expanded, un seul sinon).

## Limites (v1)

- Pas de **drawer** (le 3ᵉ palier Material) : bar ↔ rail seulement.
- `TwoPane` bascule côte-à-côte uniquement en Expanded (pas de réglage du seuil).
- La navigation par destinations est indépendante de la pile de routes existante
  (geste retour / push-pop) — les deux coexistent dans la démo.
