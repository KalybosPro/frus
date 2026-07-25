# Jalon 120 — Tests au pixel du pipeline de transformation

## Analyse

Depuis J114 (rotation) puis J117 (matrice affine unifiée), le **rendu** des
transformations n'était vérifié que *par construction* : les tests de `frus-widgets`
prouvent que le bon `Affine` est émis, mais **rien ne prouvait que le GPU le rend
correctement**. Ce jalon referme ce trou en rendant réellement des calques
transformés hors écran et en **vérifiant les pixels**.

## Décisions techniques

- **Réutilise le harnais existant.** `frus-gpu::render_offscreen` (rendu headless +
  relecture des pixels) et `frus-test::render_scene` / `Snapshot::pixel` existent
  déjà, avec **skip propre** sans adaptateur GPU. Aucune infrastructure nouvelle.

- **Tests au pixel plutôt que goldens PNG.** On assène des assertions
  **géométriques** (« après +90°, la barre est verticale, plus horizontale ») sur des
  pixels **au cœur** des formes, loin des bords anticrénelés — robuste d'un GPU à
  l'autre, auto-documenté, sans image binaire à committer.

- **On teste le maillon manquant : le shader.** On construit directement un
  `Primitive::Layer` porteur d'un `LayerTransform` (affine) et on le rend. Cela
  exerce `composite.wgsl` — l'échantillonnage à `M⁻¹(p)` — de bout en bout. Combiné
  aux tests de `frus-widgets` (la bonne matrice est émise), toute la chaîne est
  couverte.

## Implémentation

- `crates/frus-test/tests/transforms.rs` : un helper `transformed_layer(inner, color,
  m)` (rectangle plein enveloppé dans un calque transformé) et quatre cas.

## Tests (exécutés sur le rasteriseur logiciel, **non ignorés**)

- `rotation_turns_a_horizontal_bar_vertical` : +90° autour du centre — la barre
  horizontale devient verticale ; l'ancien emplacement est au fond.
- `uniform_scale_enlarges_about_center` : ×2 — un point hors du carré d'origine mais
  dans son image est peint.
- `non_uniform_scale_widens_x_only` : `scale(3, 1)` — élargi en x, inchangé en y.
- `scale_then_rotate_composes` : ×2 **puis** +90° en une matrice — l'image composée
  (étroite et haute) est correcte.
- Workspace complet vert.

## Reste

- Étendre au **clipping** (ClipRRect/ClipOval) une fois construit — même approche au
  pixel.
- Goldens PNG pour des scènes riches (texte + décor) restent utiles pour l'anti-
  régression visuelle globale ; les tests au pixel ciblent la **géométrie**.
