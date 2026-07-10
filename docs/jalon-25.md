# Jalon 25 — DPI / facteur d'échelle (HiDPI)

Le rendu tenait compte des seuls **pixels physiques** : sur un écran HiDPI
(scale 2.0), l'UI était deux fois trop petite. Désormais **le monde UI est en
pixels logiques** ; l'échelle ne s'applique qu'aux **frontières**.

## Principe

```
scale = window.scale_factor()

Entrées (curseur, molette, geste) : physique → logique   (÷ scale)
Layout / view / build_ui           : en LOGIQUE           (taille physique ÷ scale)
Sortie (rendu)                     : logique → physique   (ui.scene().scaled(scale))
```

`frus-gpu` reste **totalement ignorant du DPI** : il reçoit une scène déjà en
pixels physiques et dessine comme avant. La surface et le viewport GPU restent en
physique ; **glyphon** reçoit des tailles/positions physiques → texte **net**.

## Où l'échelle s'applique

- **`frus-core`** : `Scene::scaled(factor)` + `Primitive::scaled(factor)` (+
  `Rect::scale`, `Point::scale`) — met à l'échelle géométrie, rayon, bordure,
  flou, découpe et **taille de police** ; laisse couleurs et texte intacts.
- **`frus-shell`** : `App.scale` (lu via `window.scale_factor()`, mis à jour sur
  `ScaleFactorChanged`) ; curseur et `PixelDelta` de molette convertis en
  logique ; `view`/`build_ui` reçoivent la taille **logique** ; le rendu envoie
  `ui.scene().scaled(scale)`. `resize`/surface restent en **physique**.
- **Widgets / démo** : **inchangés** — ils étaient déjà écrits en « px logiques ».

## Décision technique (alternatives)

- **Transformer la scène en sortie** plutôt que scaler le viewport GPU ou le
  shader. Raison : le texte (glyphon) a sa propre résolution ; un viewport GPU
  logique laisserait le texte à la mauvaise taille. Scaler la scène unifie quads
  **et** texte, et garde `frus-gpu` inchangé. Coût : une copie/scale de la scène
  par frame (négligeable).

## Tests

- `Scene::scaled` : géométrie ×facteur (rect, rayon, bordure, position, taille de
  police), couleurs et chaînes préservées.
- Non-régression à **scale 1.0** : la démo WSL (scale 1.0) tourne à l'identique.

## Limites (v1)

- Impossible de valider interactivement un vrai HiDPI ici (WSL rapporte 1.0) ;
  couvert par les tests unitaires de `scaled` + revue.
- `ScaleFactorChanged` met à jour l'échelle et redessine ; le redimensionnement
  physique de surface associé reste géré par l'événement `Resized`.
