# Jalon 3 — Arbre de widgets déclaratif

Introduit la couche « à la Flutter » : on décrit l'interface avec des **widgets**
composables, traduits automatiquement en mise en page puis en rendu.

## Ce qui est livré

- **`Scene` déplacée dans `frus-core`** : liste d'affichage pure (`Primitive`),
  indépendante du GPU. `frus-gpu` la consomme (le `Painter` construit ses
  instances à partir des primitives) ; `frus-widgets` la produit.
- **Nouveau crate `frus-widgets`** :
  - trait `Widget` (style de layout, enfants, peinture),
  - widgets de base `Container` (boîte décorée) et `Flex` (rangée/colonne),
  - `build_scene(root, size)` : pilote widget → layout → peinture.
- **Démo** décrite en widgets (plus aucun appel manuel au layout).

## Architecture

```
frus-core (Scene pure) ─┬─► frus-gpu     (rend la Scene)
                        └─► frus-widgets (produit la Scene)   [pas de dépendance GPU]

Arbre de Widgets
   │ build_scene :
   │   1. Widget -> nœuds frus-layout
   │   2. compute flexbox -> rects absolus
   │   3. chaque Widget peint sa décoration
   ▼
 Scene -> frus-gpu -> écran
```

L'appariement widget ↔ rectangle repose sur un parcours **préfixe** identique de
part et d'autre (l'arbre de widgets et `Layout::absolute_rects` produisent le
même ordre), donc on peut zipper les deux.

## Décisions

- **Modèle retenu** (arbre persistant, façon Flutter) plutôt qu'immédiat, mais
  **sans reconciliation** à ce stade (pas encore d'état à differ). L'abstraction
  est prête à l'accueillir.
- **`Scene` dans `frus-core`** : `frus-widgets` reste indépendant du backend de
  rendu (pas de dépendance à wgpu). Meilleure modularité.
- **Widgets = objets-traits** (`Box<dyn Widget>`) : composition dynamique simple.

## API

```rust
let ui = Flex::column().padding(16.0).gap(12.0)
    .child(Container::new().height(56.0).color(green))
    .child(Flex::row().flex(1.0).gap(12.0)
        .child(Container::new().width(200.0).color(red))
        .child(Container::new().flex(1.0).color(blue)));

let scene = frus_widgets::build_scene(&ui, Size::new(w, h));
```

## Tests

- `frus-core` : `Scene::fill_rect` empile la bonne primitive.
- `frus-widgets` : un `Flex::row` `[Container(120px), Container(flex:1)]`
  (400×100, padding 10, gap 8) → `build_scene` produit 2 primitives aux rects
  absolus attendus (réutilise le calcul flex validé au Jalon 2).

## Limites (prochains jalons)

- Pas encore d'**état** ni d'**événements** (clic, survol, saisie) — la
  reconciliation d'arbre viendra avec.
- Peu de widgets (`Container`, `Flex`) et de propriétés de style ; pas de texte.
