# Jalon 65 — `RichText::wrap()` : le paragraphe riche replié

Clôture de l'histoire « paragraphe » : la mécanique de mesure sous contraintes du
jalon 64 (`MeasureFn` taffy, `measure_key` anti-cache-obsolète), appliquée au
texte **riche** (jalon 62).

## Ce qui change

- **`Primitive::RichText` porte `max_width: Option<f32>`** (+ `Scene::
  rich_text_wrapped`) : le rendu GPU replie les runs à la **même largeur que la
  mise en page** ; l'échelle DPI s'applique à la largeur de repli.
- **`frus_text::measure_runs_wrapped(runs, max_width)`** — la mesure riche sous
  contrainte de largeur (`measure_runs` délègue, non contraint).
- **`RichText::wrap()`** : dimensions libres, `measure()` = closure possédée sur
  les runs aplatis, `paint()` = `rich_text_wrapped(bounds.width)`.
- **`TextSpan::measure_hash`** (frus-core) : l'empreinte de mesure de l'arbre —
  textes, tailles, graisses, italiques — **sans aplatir** l'arbre à chaque frame,
  et **sans les couleurs** : recolorer un span ne doit pas invalider la mise en
  page (le test l'épingle explicitement).

## Validation

- `wrapped_rich_text_measures_and_keys_by_content` : repli borné en largeur et
  plus haut qu'en libre ; la clé de mesure **suit le contenu** mais **ignore la
  couleur** ; sans `.wrap()`, ni mesure ni clé (contrat des hooks).
- **237 tests** au total, tout vert ; build sans avertissement ; démo sans
  panique. La ligne d'accroche riche de l'écran About se replie désormais à la
  largeur de sa carte.

## Suite (§5 restants)

Décorations de texte (souligné/barré), `letter_spacing`/`line_height`,
consolidation `ColorScheme` (+ `from_seed` HCT), `content_padding` → taffy,
rayons par coin (shader SDF), `Alignment`, RTL.
