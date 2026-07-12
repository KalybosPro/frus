# Jalon 52b — `Scaffold` unifié (ossature d'écran Material)

L'AppBar (52a) et la barre de navigation vivaient **dans le corps** : elles
défilaient avec le contenu au lieu d'être un chrome fixe. C'est le rôle du
`Scaffold` de Flutter — le **coordinateur central** de la structure : barre haute
épinglée, corps défilant, navigation épinglée, plus tiroir / feuille / FAB.

## Un widget `Scaffold` qui subsume la navigation

Le développeur déclare des **slots** ; le Scaffold les assemble et **choisit seul**
la présentation selon la largeur (barre basse en étroit, rail latéral en large) —
il absorbe `NavScaffold`. Un seul code, sans brancher sur mobile/desktop.

```rust
Scaffold::new(width, height)
    .background(theme.background)
    .app_bar(header)                       // épinglé en haut
    .body(section)                         // défile entre les barres
    .nav(app.section, Msg::SetSection)     // navigation adaptative (rail | barre)
    .destination("✔", "Tasks").badge(n)
    .destination("▦", "Stats").destination("★", "About")
    .end_drawer(menu, app.drawer_open, Msg::ToggleDrawer)
    .bottom_sheet(sheet, app.sheet_open, Msg::ToggleSheet)
    .build()
```

**Assemblage :**
- **Compact** : colonne `[barre haute · corps (Scroll flex) · barre basse]`.
- **Medium/Expanded** : rangée `[rail · colonne[barre haute · corps]]`.
- Le corps est enveloppé d'un `Scroll` qui **défile** dès que le contenu dépasse
  le viewport.
- Tiroir (droite, modal) et feuille modale enveloppent l'ossature en overlays
  (réutilisent `Drawer` / `BottomSheet`).
- `inset_pad` n'enveloppe un slot que si un inset est **non nul** — sinon il
  préserve l'étirement du parent (sans quoi la barre basse se tasse).

Les **insets** restent gérés en amont par `view` (jalon 51, qui passe des
dimensions sûres) ; le Scaffold s'épingle dans ce viewport. À terme, quand toutes
les routes passeront par un Scaffold, il pourra reprendre la zone de sécurité.

## Démo

Tout le montage manuel (NavScaffold + Drawer + Stack(toast) + Container) est
remplacé par **un** `Scaffold`. Le corps (input / filtres / **liste**) défile en
entier — l'ancien `Scroll` interne à hauteur fixe disparaît. `todo_screen` et
`screen` renvoient désormais `Box<dyn Widget>`.

## Validation (sur l'appareil)

- Barre haute **épinglée** (sous la barre d'état), barre basse **épinglée**
  (au-dessus de la nav système) — capture confirmée.
- **Navigation fonctionnelle** : taper Tasks / Stats / About commute la section
  (vérifié par log `[demo] section -> n`).
- **Corps défilant** : avec 40 tâches, un glissement fait défiler la liste (#0–6
  → #8–17) pendant que les deux barres restent fixes.
- Desktop : `frus-widgets` 122 tests, `frus-demo` 15 tests, build sans avertissement.

## Limite connue → jalon 52c

- Le **FAB** (`Scaffold::fab`) est **désactivé dans la démo** : il est superposé
  via une couche `Stack` plein écran, or une telle couche supérieure **intercepte
  les clics** de la moitié basse de l'écran (limite du hit-test des `Stack` — le
  même symptôme guette un `Toast` persistant). À corriger par un overlay non
  bloquant avant de réactiver le FAB.
- Le tiroir permanent en Expanded (3 zones) n'est pas repris ici (tiroir modal
  uniquement dans le Scaffold v1).
