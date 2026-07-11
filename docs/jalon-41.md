# Jalon 41 — Colorimétrie sRGB / linéaire

Corrige une dette notée dans `color.rs` : les couleurs étaient envoyées **telles
quelles** (valeurs sRGB) à une surface **sRGB**. Le GPU les traite comme
linéaires et ré-encode linéaire→sRGB à l'écriture → **double encodage** → couleurs
**délavées** (trop claires) à l'écran.

## Correctif

Une cible sRGB ré-encode linéaire→sRGB en sortie ; il faut donc lui envoyer du
**linéaire** pour restituer la couleur voulue.

- **`frus-core`** : `Color::to_linear()` / `to_srgb()` (conversion par composante,
  alpha inchangé), avec la vraie courbe sRGB (seuil 0.04045, exposant 2.4).
- **Quads** (`quad.wgsl`) : le fragment convertit la couleur finale sRGB→linéaire
  avant de l'écrire (`srgb_to_linear`).
- **Texte** (`glyphon`) : les couleurs sont converties en linéaire avant d'être
  passées à glyphon (même raison ; l'alpha reste tel quel).

Les couleurs de la scène restent **authoring-friendly** (sRGB, comme un sélecteur
de couleur) ; la conversion se fait au tout dernier moment, à la frontière GPU.

## Tests

- `frus-core` : points fixes (0→0, 1→1), milieu (`0.5` sRGB → `~0.214` linéaire),
  aller-retour `to_linear→to_srgb`, alpha préservé.
- Les rendus offscreen (tests GPU) utilisent des couleurs **pures** (0/1),
  invariantes par la conversion → toujours verts.

## À valider à l'œil (hors WSL)

Le rendu logiciel WSL ne me permet pas de **juger les couleurs**. Sur un vrai
écran, les couleurs devraient être **plus riches / saturées** (plus délavées), et
le texte lisible (ni trop clair, ni trop sombre). Hypothèse : glyphon n'applique
pas lui-même la conversion — **à confirmer visuellement** ; si le texte paraît trop
sombre, retirer la conversion côté texte.

## Limites (v1)

- Les **dégradés** sont interpolés en espace sRGB puis convertis (mélange
  légèrement différent d'un mélange en linéaire) — acceptable en v1.
- Pas de gestion d'espaces colorimétriques larges (P3, etc.).
