# Jalon 68 — `ColorScheme` : les rôles consolidés (source de vérité unique)

Le gros morceau couleurs du §5. Depuis le jalon 58, les rôles s'accumulaient **à
plat** sur `Theme` ; ce jalon les regroupe en une vraie **`ColorScheme`** (façon
Material 3) sans casser un seul des ~130 accès existants.

## L'architecture : schéma source, champs plats dérivés

- **`ColorScheme`** : ~23 rôles écrits à la main clair/sombre — la famille
  `primary`/`secondary` (+ conteneurs et `on_*`), les surfaces (`background`,
  `surface`, `surface_variant`, **`surface_container[_high]`** pour l'élévation,
  **`inverse_surface`** pour les toasts), les contours (`outline[_variant]`),
  `error`, et **`scrim`**/**`shadow`** (l'alpha appliqué à l'usage). `lerp` rôle à
  rôle.
- **`Theme.scheme`** devient la **source de vérité** ; les champs plats
  historiques (`background`, `surface`, `primary`, `muted = on_surface_variant`,
  `border = outline`, …) sont des **vues dérivées** via `Theme::from_scheme` —
  valeurs strictement identiques, l'API des widgets ne bouge pas. Le `lerp` du
  thème interpole le schéma puis re-dérive les plats : la cohérence tient **même
  en plein fondu** (épinglée par `flat_fields_mirror_the_scheme`, testée aussi à
  `t = 0.37`).
- `focus`/`selection` restent des accents d'interaction propres à frus (hors
  rôles M3), passés à `from_scheme`.
- `Theme::from_scheme` est public : une app peut fournir son **propre schéma**
  complet (règle « personnalisable comme Flutter ») ; `from_seed` (HCT) viendra
  s'y brancher.

## Adoptions (les nouveaux rôles vivent immédiatement)

- **`scrim`** : les deux voiles codés en dur (`rgba(0,0,0, 0.5·p)` des
  modales/tiroirs, `0.22·couverture` de l'écran arrière en navigation) passent
  par `scheme.scrim.with_alpha(…)` — rendu identique (scrim noir), désormais
  thémable.
- **`shadow`** : l'ombre de `Button` (noir 35 % codé en dur) passe par
  `scheme.shadow.with_alpha(0.35)` — idem.
- **`surface_container_high`** : les lignes de `Menu` (panneau **flottant**)
  reposent sur la surface élevée au lieu de la surface de base — la tonalité
  d'élévation Material, subtile.
- `secondary*` / `inverse_surface` : présents dans le schéma (complétude de la
  palette), adoption suggérée (puces → `secondary_container`, toasts →
  `inverse_surface`) au fil de l'eau.

## Validation

- **241 tests**, tout vert — les tests thème existants passent (valeurs plates
  préservées à l'identique), + l'invariant plats↔schéma (3 thèmes dont un fondu
  partiel). Build sans avertissement ; démo sans panique.

## Suite (§5 restants)

`from_seed` (HCT) branché sur `from_scheme` ; décorations de texte,
`letter_spacing`/`line_height` ; `Alignment` ; RTL (§14) ; adoption progressive de
`secondary*`/`inverse_surface`.
