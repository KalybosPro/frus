# Jalon 57 — `BoxDecoration` : le modèle de décoration de boîte (§5)

Les fondations moteur (Bloc A du brief : phases, cache de relayout, physique
d'animation) étant posées, ce jalon ouvre le **système de design** (§5) par sa
« clé de voûte peignable » : un **modèle de décoration réutilisable**, là où chaque
widget réinventait jusqu'ici son fond/dégradé/bordure/ombre à la main.

## Ce qui manquait

`Container::paint` composait sa décoration en ligne (résolution de couleur,
`scene.shadow`, `scene.gradient_rect`/`draw_rect`), sans type partageable. Aucun
`BoxDecoration`, aucune primitive de peinture nommée. Tout widget voulant une boîte
décorée dupliquait cette logique.

## Les types cœur (dans `frus-core`, purs, `Copy`)

Nouveau module `decoration.rs` :

- **`Border { width, color }`** — bordure uniforme (`is_visible`).
- **`LinearGradient { end, direction }`** — dégradé du fond vers `end`, ancré en
  espace `[0,1]²`.
- **`BoxShadow { color, offset, blur, spread }`** — ombre douce, avec `bounds(rect)`
  (l'enveloppe décalée/floutée/élargie).
- **`BoxDecoration { color?, gradient?, border?, radius, shadow? }`** — la boîte
  décorée complète, avec :
  - **`paint_into(scene, rect, opacity)`** : abaisse la décoration en primitives de
    `Scene` dans l'**ordre fixe** ombre → fond (uni ou dégradé) → bordure ;
    `opacity` module toutes les couleurs (fondu d'apparition).
  - **`content_padding()`** : la marge à réserver pour la bordure — destinée à
    alimenter taffy pour qu'un fond bordé ne mange pas son contenu.

Également, des helpers `Color` réclamés par le brief : **`with_alpha`**,
**`from_argb_u32`** (`0xAARRGGBB`), **`compute_luminance`** (WCAG, sur canaux
linéarisés — base d'un calcul de contraste).

## Intégration : `Container` adopte `BoxDecoration`

`Container::paint` **compose** désormais un `BoxDecoration` (couleur résolue par
l'état survol/pressé, dégradé, bordure, ombre) et le peint via `paint_into`. La
logique de peinture en ligne disparaît — remplacée par le modèle partagé. Le rendu
est **strictement identique** : les 129 tests de `frus-widgets` (dont ceux qui
inspectent les primitives produites) et les 15 de la démo passent inchangés.

## Validation

- `frus-core` : **46 tests** (+9 : ordre de peinture fixe, `content_padding`,
  bordure seule, fondu d'opacité, bornes d'ombre ; `with_alpha`/`from_argb_u32`/
  luminance WCAG).
- `frus-widgets` **129**, `frus-demo` **15**, reste vert — sortie bit-à-bit
  identique après refactor de `Container`.
- `cargo build --workspace` sans avertissement.

## Suite (§5)

- **`content_padding` → taffy** : câbler la réserve de bordure dans le style pour
  que les widgets bordés dimensionnent correctement (aujourd'hui la bordure est
  purement peinte).
- **Rayons par coin** (`BorderRadius` 4 coins) — nécessite une évolution du shader
  SDF (rayon unique aujourd'hui).
- **`Alignment`**, `EdgeInsetsDirectional::resolve(dir)` (RTL), `Gradient`
  radial/sweep, `TextStyle`/`TextSpan`, puis le **thème structuré** (rôles M3 +
  échelle typographique), state-layer bakée.
