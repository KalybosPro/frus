# Jalon 208 — Surbrillance de sous-région au survol

## Analyse

Le jalon 205 change le **curseur** (main) au survol d'une sous-région cliquable, mais la sous-région
elle-même ne réagissait pas. Le retour visuel attendu (halo sous l'icône survolée) demande que le
widget connaisse la position **locale** du pointeur au moment de peindre — ce que `Status` ne portait
pas encore.

## Décisions techniques

- **La position dans `Status`, pilotée par le même signal que le curseur.** Nouveau champ
  `Status::hover_cursor: Option<Point>` (position **absolue** du pointeur), rempli **uniquement** pour
  le widget survolé. Chaque `paint` connaît déjà ses `bounds` : il ramène la position en local
  lui-même — pas besoin de propager le rectangle jusqu'à `full_status`.

- **Réutilise `cursor_icon` (jalon 205), pas de nouvelle machinerie.** Le shell pose `hover_cursor`
  exactement quand `cursor_icon` répond `Some` (le pointeur est sur une sous-région interactive), et
  le remet à `None` sinon. Résultat : le halo apparaît là et seulement là où la main apparaît, et le
  coût (repaint au mouvement) est **limité** à ces sous-régions.

- **Le hash de statut inclut `hover_cursor`.** Le suivi de dommage (`hash_status`) repeint donc quand
  la position change — le halo suit / se retire. Hors sous-région interactive, `hover_cursor` reste
  `None` : aucun repaint supplémentaire, la frugalité existante est préservée.

- **`InputState.hover_cursor`** relaie la position depuis le shell ; `status_for` la restreint au
  widget survolé.

## Implémentation

- `frus-widgets/src/interaction.rs` : champ `hover_cursor` sur `Status` et `InputState` ; `status_for`
  le restreint au survolé.
- `frus-widgets/src/ui.rs` : `hash_status` hache `hover_cursor` (quantisé).
- `frus-shell/src/app.rs` : `update_cursor_icon` pose `hover_cursor` d'après `cursor_icon` et repeint
  au changement.
- `frus-widgets/src/textinput.rs` : `paint` dessine un halo arrondi discret derrière le suffixe
  **cliquable** quand `hover_cursor` y tombe.

## Vérification

- `hovering_active_suffix_paints_a_halo` : le survol du suffixe peint un rectangle `~28x28` (le
  halo) ; dans le corps ou sans survol, aucun. Purement visuel — clic et curseur inchangés.

## Reste

- Généraliser le halo aux boutons/chips génériques, et réutiliser `hover_cursor` pour les
  **infobulles** de sous-région (valeur d'une barre / d'un point de graphe sous le pointeur).
