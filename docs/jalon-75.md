# Jalon 75 — Décorations de texte (soulignement, barré, surlignement)

## Analyse

Le `TextStyle` (jalon 60) couvrait taille/graisse/italique/couleur mais aucune
**décoration** : impossible de barrer une tâche terminée ou de souligner un
lien. Flutter expose `TextDecoration` (underline / overline / lineThrough,
combinables), `decorationColor` ; c'est la dernière brique de la spec §5 côté
attributs de texte (restent `letter_spacing`/`line_height`, hors périmètre).

Contrainte clé : ni cosmic-text ni glyphon ne dessinent les décorations — un
moteur de texte met en forme des glyphes, les lignes sont l'affaire du
rasteriseur au-dessus (comme dans Flutter, où le paragraphe les peint lui-même).

## Architecture

- **`frus-core`** : `TextDecoration { underline, overline, strikethrough }`
  (Copy, combinable via `combine`, consts `UNDERLINE`/`STRIKETHROUGH`/…).
  `TextStyle` gagne `decoration` + `decoration_color: Option<Color>` (`None` =
  couleur du texte). `TextSpan` cascade les deux via ses `Overrides` partiels ;
  `TextRun` et `Primitive::Text` les transportent jusqu'au GPU.
- **`frus-gpu`** : les quads de décoration sont calculés **depuis les lignes
  mises en forme** (`buffer.layout_runs()`), donc exacts même avec repli
  (`max_width`) et runs mêlés. `TextPainter::prepare_frame` renvoie des
  `DecorationQuad { rect, color, clip }` ; le `Painter` des rectangles les
  dessine dans sa passe (sous les glyphes — même couleur : indiscernable).
  - Texte simple : une ligne par `layout_run`, de la première à la dernière
    avance de glyphe.
  - Texte riche : chaque span reçoit `Attrs::metadata(index du run)` ; les
    glyphes consécutifs d'un même run forment un segment décoré — la
    décoration est bien **par-span**, pas par-ligne.
  - Offsets depuis la ligne de base (`line_y`) en fraction de la taille du
    run : soulignement +0,12 em, barré −0,28 em (≈ mi-hauteur d'x),
    surlignement −0,90 em ; épaisseur `max(1, taille/14)`.

## Décisions

- **Pas d'effet sur la mesure** : les décorations sont exclues de
  `measure_hash`/`measure_key` (comme les couleurs) — recolorer ou barrer un
  paragraphe n'invalide pas le cache de relayout.
- `merge` : la décoration est un attribut *de type* (celle de `over` gagne en
  bloc), sa couleur *hérite* comme la couleur du texte. Dans un `TextSpan`,
  `decoration(NONE)` explicite annule l'héritage (`Some(NONE)` ≠ absent).
- L'ordre du Renderer s'inverse : le texte se prépare **d'abord** (il produit
  les quads), les rectangles ensuite. Aucun changement de passes.
- `decoration_style` Flutter (pointillés, ondulé) : non retenu — pas de
  consommateur, et demanderait un pipeline dédié.

## Implémentation

- `frus-core/text_style.rs` : type + builders (`.underline()`,
  `.strikethrough()`, `.decoration()`, `.decoration_color()`) sur `TextStyle`
  **et** `TextSpan` ; champs sur `TextRun`.
- `frus-core/scene.rs` : `Primitive::Text` gagne les deux champs ;
  `text_styled`/`text_wrapped` les prennent du style ; `push_faded` fond la
  couleur de décoration, `scaled` la transporte (l'épaisseur dérive de la
  taille déjà mise à l'échelle).
- `frus-gpu/text.rs` : `DecorationQuad`, `push_line_quads`, calcul par
  `layout_runs` (+ `metadata` par-span pour le riche) ; `painter.rs` accepte
  les quads en plus de la scène ; `renderer.rs` réordonne la préparation.
- Widgets : `Text::underline()/strikethrough()/decoration*()` ; `RichText`
  transporte les champs (fondu de sortie appliqué à la couleur de décoration).
- Démo : tâche terminée = libellé grisé **et barré** ; « portable » de la
  tagline riche souligné.

## Tests (253 → 256)

- `decorations_combine_and_cascade` (core) : combinaison, héritage dans les
  spans, annulation explicite, sémantique de `merge`.
- `underline_lights_more_pixels_than_plain_text` (gpu, readback) : le même
  texte souligné allume strictement plus de pixels — preuve de bout en bout
  (calcul des quads + passe des rectangles).
- `rich_text_strikethrough_is_per_run` (gpu, readback) : le barré du second
  run ajoute des pixels — preuve du chemin métadonnées → segments par run.

## Limites connues

- Les décorations se dessinent **sous** les glyphes (passe des rectangles) :
  avec une couleur de décoration différente, un barré passe derrière l'encre
  du glyphe là où Flutter le peint devant. Même couleur : indiscernable.
- Offsets approchés (fractions d'em) plutôt que lus dans les métriques de la
  police (`post.underlinePosition`) — suffisant pour DejaVu ; à raffiner si
  des polices exotiques arrivent.
