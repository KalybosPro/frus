# Jalon 76 — `from_seed` : thème généré depuis une couleur graine (HCT)

## Analyse

Le `ColorScheme` (jalon 68) était écrit à la main. Material 3 génère le sien
depuis **une seule couleur graine** via l'espace **HCT** (Hue-Chroma-Tone) :
teinte/chroma perceptifs de CAM16 + ton L\* de CIELAB. Le ton porte le
contraste — deux tons éloignés de 40+ garantissent la lisibilité — donc un
schéma « par tons » a des paires `X`/`on_X` lisibles **par construction**.

## Architecture

- **`frus-core/hct.rs`** (pur, zéro dépendance) : port de
  `material-color-utilities` (Google).
  - Analyse [`Hct::from_color`] : sRGB → XYZ (D65) → CAM16 (conditions de
    vision standard) pour teinte/chroma, L\* pour le ton.
  - Synthèse [`Hct::solve`] : itération de Newton sur la clarté `J` (5 pas,
    `findResultByJ`) ; hors gamut, **dichotomie sur le chroma** (précision
    0,4 — le solveur historique de Google) au lieu de la bissection analytique
    de la frontière du gamut (~150 lignes évitées pour ±2/255 d'écart max).
  - [`TonalPalette`] : une teinte/chroma déclinée sur l'échelle des tons.
- **`frus-widgets`** : `ColorScheme::from_seed(seed, dark)` — 5 palettes
  (primaire = chroma de la graine plancher 48 ; secondaire 16 ; neutre 4 ;
  neutre-variante 8 ; erreur teinte 25 chroma 84), chaque rôle = un ton M3.
  `Theme::from_seed` en dérive focus/sélection depuis la primaire.

## Décisions

- **Vérité terrain** : les constantes et le comportement sont épinglés contre
  le port Python `materialyoucolor` (#4285F4 → H 265.979, C 62.269, T 56.550 ;
  les gris gardent un chroma résiduel ≈ 1,9 sous adaptation partielle — ce
  n'est pas un bug). Une constante mal recopiée du solveur (`m[2][2]`) a été
  détectée par ce croisement — d'où les tests à valeurs exactes.
- Écart M3 assumé : `surface` décollée du `background` (tons 12/6 sombre,
  100/98 clair) — nos cartes posent une surface sur le fond, la spec 2023 les
  confond.
- La palette **tertiaire** (teinte +60°, chroma 24) attendra un rôle
  consommateur (pas de champ tertiaire dans le schéma → pas de code mort).

## Tests (256 → 265)

- `google_blue_analyzes_to_known_hct`, `solve_matches_reference_implementation`
  (valeurs exactes du port Python, ± 1/255 en gamut, ± 3 hors gamut),
  round-trips, monotonie de la palette en luminance, entrées dégénérées.
- `from_seed_generates_contrasting_pairs` : **toutes** les paires `X`/`on_X`
  tiennent l'AA (≥ 4,5:1) pour 3 graines × 2 modes — y compris une graine
  grise (chroma quasi nul).
- `from_seed_light_and_dark_share_the_hue` : les deux modes déclinent la même
  teinte, fonds respectivement sombre/clair.

## Démo

Action « Seed: … » dans le menu de l'AppBar : cycle schéma main → Blue
(#4285F4) → Purple (#9C27B0) → Orange (#E8710A), avec le même fondu que la
bascule clair/sombre (le thème généré s'interpole rôle à rôle comme les
autres).
