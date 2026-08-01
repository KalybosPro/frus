# Jalon 255 — Peinture du glisser-déposer portée sur le thème / constantes nommées

## Analyse

La revue du jalon 254 a relevé, dans la peinture du glisser-déposer du shell, des **littéraux** en
contradiction avec la règle « customizable like Flutter » : couleur d'ombre du fantôme codée en dur
(`Color::BLACK.fade(0.28)`), et une poignée de **nombres magiques** de géométrie (décalage 4, flou 12,
soulèvement −2, épaisseur/rayon d'insertion). À côté, `Button` tire déjà son ombre du thème
(`theme.scheme.shadow.with_alpha(…)`) — c'est la convention établie.

## Décisions techniques

- **Couleur d'ombre du thème.** Le fantôme utilise `theme.scheme.shadow` (surchargeable via le thème),
  comme `Button`, au lieu de `Color::BLACK`. `scheme.shadow` **étant** noir dans les thèmes clair et
  sombre fournis, le rendu est **strictement identique** — c'est un **dé-codage-en-dur**, pas un
  changement visuel.
- **Géométrie en constantes nommées.** Décalage/flou/opacité d'ombre, opacité/épaisseur du bord,
  soulèvement horizontal et épaisseur de la ligne d'insertion vivent dans un petit module
  `drag_preview` documenté — comme `Button`/`Card` gardent leur géométrie d'ombre en local. Le rayon de
  la ligne d'insertion dérive désormais du thème, borné à la demi-épaisseur
  (`theme.radius.min(line.height * 0.5)` = 1.5 aux valeurs courantes → identique).
- **Portée volontairement limitée au DnD.** `Card`/`Toast` portent le même littéral d'ombre
  (`rgba(0,0,0,0.3)`) ; laissé pour une passe de consolidation dédiée (voir Reste) afin de garder ce
  jalon focalisé.

## Implémentation

- `frus-shell/src/app.rs` : module `drag_preview` (constantes de géométrie) ; `draw_ghost_card` prend
  la couleur d'ombre du thème et les constantes nommées ; la ligne d'insertion prend son épaisseur
  (`INSERT_THICKNESS`) et un rayon dérivé du thème ; le soulèvement horizontal passe par `LIFT_Y`.

## Vérification

- **Shell 27** (dont `ghost_card_shape`) ; **goldens 77 inchangés** (dé-codage pixel-identique).
- La peinture DnD est un état runtime (glisser), non inspecté au GPU ici ; le changement est une
  substitution valeur-pour-valeur (ombre thème = noir, rayon borné = 1.5), sans écart visuel.

## Notes

- Convention retenue, cohérente avec `Button` : la **couleur** vient du thème, la **géométrie** reste
  en constantes locales nommées. Un vrai « spec d'élévation » thémé (couleur + décalage + flou, façon
  Material elevation) reste possible mais toucherait `lerp` et tous les dessinateurs d'ombre — hors
  périmètre ici.

## Reste

- Unifier l'ombre de `Card`/`Toast` (mêmes `rgba(0,0,0,0.3)` codés en dur) sur `theme.scheme.shadow`,
  voire un helper d'élévation partagé.
- Consolidation `ui.rs` (boucles de parcours) et unification des deux `reflow_*`.
- Couverture réagencement même-colonne ; inertie/ressort vertical.
