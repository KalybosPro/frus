# Jalon 95 — Animations implicites : courbe & durée par widget

## Analyse

frus animait déjà **implicitement** les valeurs de widgets (`Widget::anim_target`
→ `Runtime::values`/`advance_values`) : un widget déclare une **progression cible**
`[0,1]`, le runtime fait tendre la valeur retenue vers elle (snap au montage), et
chaque widget interpole ses propriétés par cette progression au paint (interrupteur,
tiroir, feuille…). Deux manques face à Flutter (`AnimatedContainer`/`AnimatedFoo`,
paramètres `duration`+`curve`) :

1. **Progression linéaire** : `advance_values` avançait la valeur à pas constant.
   Résultat robotique — départ/arrivée brusques.
2. **Durée figée** : une constante globale (`ANIM_DURATION = 0.12 s`), non réglable
   par widget.

Or `frus-core` fournit depuis longtemps une API [`Curve`] complète et testée
(Linear, Cubic/bézier, CriticalSpring, Interval, Flipped, `ease_*`) — **inutilisée**
côté widgets. Ce jalon la branche.

## Décisions techniques

- **Deux méthodes opt-in sur le trait** [`Widget`] : `anim_duration() -> f32`
  (défaut : la durée standard) et `anim_curve() -> Curve` (défaut : `Linear`). Les
  défauts **préservent exactement** le comportement antérieur — aucun widget
  existant ne change de feel sans le demander (fidèle à Flutter, dont le défaut est
  aussi linéaire). Forwardées par les wrappers transparents (`Box`, `Keyed`,
  `Responsive`).

- **Timeline courbée** ([`ValueAnim`]). Chaque valeur animée retient désormais
  `{ current, from, to, elapsed }`. À chaque frame : `t = (elapsed/duration)`
  borné, `current = lerp(from, to, curve(t))`. Un **changement de cible rebase**
  la timeline depuis la valeur courante (`from = current`, `elapsed = 0`) : la
  reprise est franche et continue même en plein vol (0→1→0), exactement le modèle
  des *implicit animations* de Flutter. Le montage adopte la cible sans transition.

- **`current` est la seule valeur lue au paint** (`Runtime::value`/`value_or`), la
  timeline restant interne. Un helper `set_value(id, v)` pose une valeur au repos
  (rendus/tests isolés).

## Démonstration

`Switch` adopte `Curve::ease_in_out()` : le pouce **accélère puis freine** au lieu
de glisser à vitesse constante — l'animation canonique d'une bascule. Tout autre
widget peut désormais régler sa courbe/durée d'une ligne.

## Tests

- `curve_shapes_the_value_timeline` : via un widget mock, à t=0.25 un *ease-in* est
  **en retard** sur la progression linéaire (0.25), un *ease-out* **en avance** ; le
  linéaire vaut exactement `t` ; toutes convergent vers la cible.
- `shorter_duration_animates_faster` : à `dt` égal, une durée plus courte est plus
  avancée (t=0.5 vs 0.125).
- Les tests existants (`value_snaps_on_mount_then_animates`, `anim_target_*` des
  tiroir/feuille) restent verts : endpoints et snap au montage inchangés ; les
  rendus à mi-progression passent par `set_value`.

## Reste

- Widgets `Animated*` clés en main (couleur/taille/padding interpolés d'un bloc via
  [`Tween`]), et un `AnimatedOpacity` s'appuyant sur les calques (J92-94).
- Courbes par **propriété** et fenêtres étagées (`Curve::Interval`) déjà possibles
  côté core, à exposer au niveau widget.
- Animations pilotées par cible **multiple** (aujourd'hui une progression scalaire
  par widget).
