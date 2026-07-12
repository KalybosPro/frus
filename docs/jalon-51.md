# Jalon 51 — Insets système (zone de sécurité / SafeArea)

Sur mobile, l'interface débordait **sous les barres système** (barre d'état en
haut, barre de navigation en bas). Le framework remonte désormais les **insets**
de la plateforme jusqu'à l'application, qui écarte son contenu de ces zones — le
fond, lui, s'étend bord à bord. C'est l'équivalent de `MediaQuery.padding` +
`SafeArea` de Flutter.

## Source des insets (Android)

`android-activity` expose `AndroidApp::content_rect()` : le rectangle de contenu
**hors** barres système, en px physiques. Le shell garde une poignée de
l'activité (`run_android`), et à chaque frame en dérive les insets :

```
inset.top    = content.top
inset.left   = content.left
inset.right  = largeur_surface  − content.right
inset.bottom = hauteur_surface  − content.bottom
```

converti en px **logiques** (÷ échelle). Un rectangle dégénéré (avant la première
mise en page) donne des insets nuls. Sur les plateformes de bureau : toujours zéro
(pas de poignée Android).

## Propagation à l'application

Nouveau point d'entrée du trait `Application`, sur le modèle de `on_resize` :

```rust
fn on_insets(&mut self, _insets: Insets) {}   // défaut : no-op
```

Le shell appelle `on_insets` quand les insets **changent** (juste après
`on_resize`, avant la construction de la vue). L'app les stocke et s'en sert.

## Application côté démo

L'interface est construite aux dimensions **internes** (fenêtre moins insets),
puis enrobée d'un conteneur plein-fenêtre à fond `background` avec
`padding_each(insets)` :

```rust
let w = width  - insets.left - insets.right;
let h = height - insets.top  - insets.bottom;
let nav = build_view(self, theme, w, h);
Container::new().width(width).height(height).color(theme.background)
    .padding_each(insets.top, insets.right, insets.bottom, insets.left)
    .child(nav)
```

Le fond couvre tout l'écran (y compris sous les barres) ; le contenu reste dans
la zone sûre. Insets nuls → aucun enrobage (desktop inchangé).

## Validation

- **Sur l'appareil** (Huawei, Android 10) : insets mesurés `top 84 px`,
  `bottom 45 px`, `left/right 0` ; le contenu dégage la barre d'état et la barre
  de navigation, fond bord à bord (capture confirmée).
- **Desktop** : `Insets::ZERO` partout, aucune régression (build + tests verts,
  test `on_insets_updates_safe_area`).

## Limites (v1)

- Les **overlays** (feuille modale, tiroir, menus) se positionnent encore par
  rapport à la fenêtre entière, pas à la zone sûre — une feuille peut affleurer
  la barre de navigation.
- Pas encore de `viewInsets` du **clavier logiciel** (remontée du contenu au-dessus
  du clavier) : ce sera le jalon IME.
- Les insets ne reconfigurent pas le **palier** responsive (calculé sur la largeur
  pleine) ; sans effet en pratique (insets latéraux nuls en portrait).
