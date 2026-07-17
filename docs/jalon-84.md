# Jalon 84 — RTL : direction de lecture et miroir de mise en page (§14, ouverture)

## Analyse

Le §14 (i18n/l10n/RTL) commence par la **direction**. Objectif : afficher
correctement une interface droite-à-gauche (arabe, hébreu) — les rangées,
l'alignement et les marges directionnelles se retournent, le texte reste lisible
dans sa boîte (le bidi *interne* d'un paragraphe est l'affaire de cosmic-text).

## Architecture

- **`frus-core`** : `TextDirection { Ltr, Rtl }` et `InsetsDirectional
  { start, end, top, bottom }` avec `.resolve(dir) -> Insets` (en RTL,
  `start` → droite). Portés dans le socle zéro-dep.
- **Propagation** : `Theme.direction` (contexte ambiant threadé jusqu'au paint,
  en attendant un `Env` §2), `Theme::rtl()`. `Theme::lerp` garde la direction
  de la cible (attribut discret, pas de fondu).
- **Miroir de mise en page** (le cœur) : taffy 0.7 n'a pas de `direction: rtl`.
  Plutôt que de réécrire chaque widget, le pilote **retourne les rectangles**
  de *chaque racine de layout* autour de sa largeur, quand la direction est
  RTL :

  ```
  r.x  ->  root.x + (root.width - (r.x - root.x) - r.width)
  ```

  Taffy calcule en LTR (canonique, mis en cache), le miroir s'applique après
  récupération dans `Builder::cached_rects`. Résultat : les rangées s'inversent,
  l'alignement et le padding se retournent, hit-test et clips restent cohérents
  (mêmes rectangles) — **sans toucher aux ~60 widgets**. Le chemin LTR est
  inchangé bit-à-bit (`mirror` court-circuité).

## Décisions

- Miroir **par racine de layout** (fenêtre, écran, contenu défilant, item de
  liste) autour du 1ᵉʳ rect (la racine) : compose correctement à travers les
  translations imbriquées.
- Le texte n'est pas retourné glyphe par glyphe : sa boîte se déplace du bon
  côté et il s'y dessine normalement ; le **bidi intra-paragraphe** (chiffres,
  mots latins dans un texte arabe) est délégué à cosmic-text.
- `InsetsDirectional` est fourni pour les marges qui doivent suivre la
  direction ; les widgets l'adopteront progressivement.

## Tests (283 → 287)

- `directional_insets_flip_start_end` (core).
- `rtl_mirrors_row_horizontally` (widgets, hit-test) : un bouton fixe passe de
  gauche (LTR) à droite (RTL), le flexible occupe l'autre bord.
- `rtl_mirrors_the_row` (frus-test, golden + pixels) : la rangée
  [rouge][vert][bleu] devient [bleu][vert][rouge] en RTL (rouge à droite) —
  preuve visuelle indépendante de la police.
- Les 21 suites LTR restent vertes (chemin inchangé).

## Démo

Action « RTL »/« LTR » dans le menu de l'AppBar : bascule toute l'application
en miroir (barre haute, cartes, listes, navigation).

## Limites (suite du §14)

- Le **placement des overlays** (tiroir Left/Right, ancre des menus) n'est pas
  encore retourné — un tiroir « Left » reste à gauche en RTL.
- Le **geste retour** reste sur le bord gauche (devrait être à droite en RTL).
- **Couverture de police** : le rendu de l'arabe dépend de la fonte ; la fonte
  embarquée (DejaVu) a une couverture arabe limitée. Sur Android, la fonte
  système prend le relais.
- **Localisation** (Fluent) et **accessibilité** (AccessKit) : chantiers
  distincts, à venir.
