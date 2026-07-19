# Jalon 87 — Écriture arabe (bidi) : rendu du script + correctif RTL hors-écran

## Objectif

Le J84 a posé le **miroir de mise en page** RTL (rangées/marges/overlays
retournés). Restait le plus visible : **afficher réellement de l'arabe**. Ce
jalon embarque une face arabe, route les runs par script, et corrige un bug de
placement qui rendait tout texte RTL **invisible** sur l'appareil.

## Ce qui est fait

### Face arabe embarquée
`NotoNaskhArabic-Regular.ttf` + `-Bold.ttf` (famille « Noto Naskh Arabic »)
chargées dans `frus_text::new_font_system` aux côtés de DejaVu. Embarquées (et
non résolues via le système) pour un rendu **déterministe partout**, en
particulier Android où aucune liste de repli plateforme n'est peuplée.

### Routage par script (`family_for`)
DejaVu ne couvre pas l'arabe et **cosmic-text ne fait pas de repli
cross-famille sur Android**. On choisit donc la famille **à la source** :
`family_for(text)` renvoie la famille Naskh si le texte contient un caractère
des blocs arabes (0600–06FF, 0750–077F, 08A0–08FF, FB50–FDFF, FE70–FEFF),
sinon la sans-serif. Appliqué **de façon identique** en mesure (`frus-text`) et
au rendu (`frus-gpu`), texte simple **et** riche.

### Correctif : texte RTL rendu hors écran (cause du « blanc »)
Symptôme : sur l'appareil, le layout RTL se retournait bien et les chaînes
**latines** s'affichaient, mais **tout l'arabe restait vide** (titre, filtres,
libellé de menu « العربية »). Diagnostic sur l'appareil : le shaping produisait
de **vrais glyphes** (7 glyphes, 0 `.notdef`) — donc police et shaping OK.

Cause réelle : pour un texte **non-paragraphe** (`max_width == None`), le
renderer bornait le buffer à la **largeur de la surface**
(`unwrap_or(width)`). Or cosmic-text **aligne un run RTL à droite** de la
largeur du buffer : les glyphes atterrissaient à x ≈ largeur_surface, puis
décalés de `position.x` → **hors écran à droite**. Le latin (aligné à gauche,
x ≈ 0) n'était pas affecté — d'où « latin visible, arabe blanc ».

Correctif (frus-gpu `text.rs`, arms `Text` et `RichText`) : passer
directement `*max_width` à `set_size` — **non contraint** (`None`) pour un
texte libre, jamais borné à la surface. Un vrai paragraphe garde sa largeur de
mise en page (l'alignement RTL à droite **dans la boîte** est alors correct,
conforme à Flutter).

### Démo : locale arabe
`i18n/ar.ftl` (titre/filtres + pluriels CLDR arabes zéro/un/deux/…),
`LANGS` passe à trois entrées (English / Français / العربية), et le choix
« arabe » active automatiquement le thème RTL (`lang_is_rtl` → `Theme::rtl`).

## Tests (frus-text + frus-gpu)

- `arabic_shapes_with_embedded_only_font_system` : reproduit le cas Android
  (db embarquée seule, **aucun** repli système) → l'arabe façonne de vrais
  glyphes (`glyph_id != 0`) via `Family::Name`. Isole la résolution de police
  de tout repli plateforme.
- `rtl_right_aligns_to_buffer_width` : **prouve la cause** — buffer large ⇒
  premier glyphe RTL à droite (x > 500) ; non contraint ⇒ à gauche (x < 50).
- `renders_arabic_to_non_background_pixels` (frus-gpu, readback offscreen) :
  l'arabe rasterise bien des pixels.
- `arabic_falls_back_to_the_embedded_naskh_face` (mesure) conservé.

## Validé sur l'appareil (Huawei STK-L21)

Locale العربية : titre **« مهامي »**, filtres **« الكل / النشطة / المكتملة »**,
libellé de menu **« العربية »** — tous rendus, formes de jonction correctes,
ordre RTL, sous mise en page miroir (hamburger à droite, nav inversée). ✔

## Reste

- Sélection de police par script généralisable (hébreu, etc.) si besoin.
- Formatage dates/nombres par locale (repris de J86).
