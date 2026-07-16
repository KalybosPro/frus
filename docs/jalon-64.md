# Jalon 64 — Mesure sous contraintes (closures taffy) + paragraphe à retour à la ligne

Le dernier manque du protocole de layout (§1 du brief) : « les tailles
intrinsèques routées vers la **closure de mesure de taffy** — pour le texte et le
contenu peint sur mesure ». Jusqu'ici, un `Text` mesurait sa taille naturelle et
la **figeait** dans son style : un texte long débordait ou était découpé, jamais
replié à la largeur du parent.

## Feuilles mesurées (frus-layout)

- **`MeasureFn`** = `Box<dyn Fn(Option<f32>, Option<f32>) -> Size>` : reçoit les
  largeur/hauteur maximales (`None` = libre), renvoie la taille du contenu.
- **`Layout::measured_leaf(style, data, measure)`** — la closure est retenue par
  nœud (`HashMap<NodeId, MeasureFn>`), **sans toucher au type de contexte** de
  l'arbre.
- Les deux chemins de calcul passent par **`compute_layout_with_measure`** ; la
  traduction de contraintes donne les **intrinsèques gratuitement** :
  `min-content` → largeur `Some(0)` (le mot le plus long), `max-content` → `None`
  (taille naturelle).

## Le paragraphe : `Text::wrap()`

- `style()` → dimensions libres ; **`measure()`** → closure possédée (contenu
  cloné) sur `frus_text::measure_wrapped` (repli cosmic-text sous largeur
  contrainte) ; `paint()` → **`Scene::text_wrapped`**.
- **`Primitive::Text` porte `max_width: Option<f32>`** : le rendu GPU se replie à
  la **même largeur que la mise en page** (avant, glyphon repliait à la largeur de
  surface — jamais atteinte). `scaled` met la largeur de repli à l'échelle DPI.
- Nouveaux hooks `Widget::measure` / `Widget::measure_key`, délégués par
  `Box<dyn Widget>`, `Keyed`, `Responsive`. Le texte **sans** `.wrap()` est
  strictement inchangé (dimensions figées, pas de closure).

## Le piège du cache de relayout — corrigé

Le cache (jalon 55) n'empreint que **style + structure**. Or le contenu d'une
feuille mesurée influe sur la géométrie **sans passer par le style** : deux
paragraphes différents, mêmes styles, auraient partagé une empreinte — et le cache
aurait resservi une **vieille mise en page**. D'où **`measure_key()`** (empreinte
du contenu : texte + taille + graisse + italique), mêlée à l'empreinte de
relayout. Contrat documenté : `measure()` et `measure_key()` sont `Some` ensemble.

Le test `wrapped_text_wraps_in_layout_and_invalidates_the_cache` épingle
précisément ce scénario : même arbre, même runtime (cache chaud), contenu
différent → le suiveur cliquable **bouge** (recalcul), et le paragraphe replie ses
lignes dans la colonne (le suiveur est repoussé).

## Validation

- `frus-layout` **4 tests** (+1 : feuille mesurée repliée à la largeur offerte, 3
  lignes attendues) ; `frus-text` **10** (+1 : repli borné en largeur, hauteur qui
  grandit) ; `frus-widgets` **140** (+2 : mesure/clé du paragraphe + le test de
  bout en bout layout + cache). **236 tests** au total, tout vert.
- Démo : l'écran About gagne un paragraphe replié à la largeur de la carte.
  Build sans avertissement ; démo sans panique.

## Suite

- `RichText` à retour à la ligne (même mécanique, mesure sur runs).
- La suite §5 côté couleurs : consolidation `ColorScheme`, `content_padding` →
  taffy (les feuilles mesurées ouvrent la voie aux mesures avec padding).
