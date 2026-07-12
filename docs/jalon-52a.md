# Jalon 52a — AppBar adaptative (barre d'application Material)

L'en-tête de la démo entassait ~12 contrôles dans une seule rangée ; en Compact,
les actions passaient à la ligne et **percutaient le titre**. Le symptôme même de
« ne respecte pas les principes de mobile » : une barre d'outils desktop plaquée
sur un téléphone.

## Principe : un seul code, adaptation par le framework

Le développeur déclare **un** titre, un `leading` optionnel et une liste
d'**actions** — sans jamais dire « ceci est pour mobile / desktop ». Le nouveau
widget [`AppBar`] décide **seul**, d'après la largeur disponible, combien
d'actions tiennent en ligne et **replie le reste dans un menu overflow `⋯`**.
Large → tout en ligne ; étroit → overflow. C'est le comportement d'une AppBar
Material.

```rust
AppBar::new("My Tasks")
    .width(available_width)                 // une taille, pas une plateforme
    .leading(button("☰", Msg::ToggleDrawer))
    .overflow(app.actions_open, Msg::ToggleActions)
    .action("Pause", Msg::ToggleTimer)
    .action("Settings →", Msg::Push(Route::Settings))
    // …
    .build()
```

## Mécanique

- La largeur exacte de chaque action est **mesurée** (`frus_text::measure`, comme
  `Button`), donc le repli est précis, pas estimé.
- Empilage glouton : si toutes les actions tiennent dans le budget (largeur − tête
  − titre − marges), tout passe en ligne, **sans** bouton `⋯` ; sinon on réserve le
  `⋯` et on garde autant d'actions en ligne que possible, le reste va au menu.
- Le menu overflow réutilise [`Menu`] (contrôlé : ouvert/fermé par l'app, overlay
  différé, fermeture au clic extérieur) — donc il vit dans l'arbre retenu, pas dans
  un `LayoutBuilder` (dont le contenu n'a pas d'overlay).
- La largeur est **passée** par l'app (c'est la taille dont elle dispose déjà, pas
  un indicateur de plateforme) : conforme au principe « un seul codebase ».

## Validation

- `frus-widgets` : `wide_bar_shows_all_actions_inline` (3/3 en ligne),
  `narrow_bar_collapses_into_overflow` (repli + `⋯`). 120 tests verts.
- **Sur l'appareil** (Android, largeur ~232 px logiques) : en-tête propre
  `[☰] My Tasks · [Pause] [⋯]`, plus aucun chevauchement ; le menu `⋯` déroule
  Light / A+ / A− / Log → / Settings → / Quick actions / Save / Clear completed.
- Démo : aucune branche `SizeClass` dans l'en-tête ; l'ancienne rangée `Wrap` +
  `Menu` + spinner disparaît au profit d'un seul `AppBar::new(...).build()`.

## Limites (→ 52b)

- L'AppBar est **placée dans le body qui défile** ; elle devrait être **épinglée**
  en haut (chrome fixe), tout comme la barre de navigation basse en bas. C'est le
  rôle d'un `Scaffold` (appBar / body scrollable / bottomNavigationBar) — jalon 52b.
- La tête (`leading`) réserve une largeur fixe (56 px) ; un leading large serait
  sous-estimé dans le budget.
- Cibles tactiles encore sous 48 dp (jalon dédié).
