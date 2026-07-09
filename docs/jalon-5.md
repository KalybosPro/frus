# Jalon 5 — Texte

Ajoute le rendu de texte : mesure (pour le layout) et rasterisation GPU (via un
atlas de glyphes). L'UI peut enfin afficher des libellés qui réagissent à l'état.

## Ce qui est livré

- **Nouveau crate `frus-text`** : mesure d'une ligne de texte via
  [`cosmic-text`](https://docs.rs/cosmic-text) (`measure(text, px) -> Size`).
  `FontSystem` initialisé paresseusement et partagé (Mutex).
- **`frus-core`** : nouvelle primitive `Primitive::Text { position, text, size, color }`
  + `Scene::text(...)`.
- **`frus-widgets`** : widget `Text` (se dimensionne par mesure, pousse une
  primitive de texte à la peinture).
- **`frus-gpu`** : rendu du texte via [`glyphon`](https://docs.rs/glyphon)
  (cosmic-text + atlas + pipeline wgpu). Le `Painter` (rectangles) et le
  `TextPainter` (texte) dessinent dans le **même render pass** ; le renderer
  sépare `prepare` (téléversements) et `draw` (enregistrement dans le pass).
- **Démo** : un bouton avec libellé « + Ajouter un carré » et un compteur
  « Carrés : N » qui se met à jour à chaque clic.

## Architecture

```
frus-core   : Primitive::Text (donnée pure)
frus-text   : measure(text, px) -> Size          [cosmic-text, FontSystem global]
frus-widgets: widget Text → style()=measure ; paint()=Scene::text
frus-gpu    : TextPainter (glyphon) rend les Primitive::Text dans le render pass
```

## Décisions & simplifications (v1)

- **Réutilisation** : `glyphon` (rendu) + `cosmic-text` (mesure) plutôt qu'un
  pipeline texte maison — résultat rapide et robuste. Un pipeline maison
  (SwashCache → atlas → shader) reste envisageable plus tard pour un contrôle total.
- **Une ligne**, police système par défaut, alignement gauche.
- **Deux `FontSystem`** distincts (mesure vs rendu) : inefficacité connue, à
  unifier via un contexte de police partagé.
- `prepare`/`draw` séparés dans le renderer pour que rectangles et texte
  partagent un seul render pass.

## Tests

- `frus-text` : `measure` > 0 pour un texte non vide, largeur nulle si vide.
- `frus-widgets` : le widget `Text` émet la bonne `Primitive::Text`.
- `frus-gpu` : **rendu offscreen** — « Hello » blanc sur fond noir produit des
  pixels non-noirs (preuve automatique de la rasterisation, sans fenêtre).
- `frus-shell` : `view` produit le bon nombre de primitives (fond bouton +
  libellés + carrés).

## Prérequis (WSL)

Une police système est nécessaire : `apt-get install -y fonts-dejavu-core fontconfig`.

## Limites (prochains jalons)

- Pas de retour à la ligne / alignement / styles riches (gras, italique).
- Deux `FontSystem` à unifier.
- États visuels survol/pressé/focus et clavier : toujours en attente de la
  reconciliation.
