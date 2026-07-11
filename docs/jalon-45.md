# Jalon 45 — Responsivité widgets avancée

Trois compléments à la navigation adaptative : un **tiroir latéral**, des
**badges de notification** sur les destinations, et deux **nouveaux axes** de
responsivité (orientation et hauteur).

## Lot A — `Drawer` (tiroir latéral, 3ᵉ palier Material)

`Drawer` complète `NavRail` (rail) et `BottomBar` (barre) : un panneau
plein-hauteur qui glisse depuis le bord gauche par-dessus le corps, avec un
voile qui le referme au clic extérieur.

```rust
Drawer::new(open)
    .on_dismiss(Msg::CloseMenu)
    .panel(nav_list)   // contenu du tiroir
    .body(main_screen) // fond, toujours visible
```

Implémentation : un nouveau **placement d'overlay** `Placement::Left`. Le
panneau (`DrawerPanel`, interne) a une largeur fixe (`DRAWER_WIDTH = 280`) et une
hauteur `Percent(1.0)` ; l'overlay `Left` est calculé avec la **hauteur
contrainte à la fenêtre** (largeur libre), de sorte que le panneau se déploie sur
toute la hauteur. Le voile et la fermeture au clic réutilisent le mécanisme des
modales `Center`. Fermé, le `Drawer` n'émet aucun overlay (le corps seul est
rendu).

## Lot B — Badges / compteurs sur les destinations

Une destination de navigation peut porter un **compteur de notifications** : une
pastille rouge en haut-droite du glyphe, plafonnée à `99+`.

```rust
NavRail::new(sel, Msg::Go).item("✉", "Mail").badge(5)
BottomBar::new(sel, Msg::Go).item("✉", "Mail").badge(5)
NavScaffold::new(class, sel, Msg::Go).destination("✔", "Tasks").badge(active)
```

`.badge(count)` décore la **dernière** destination ajoutée ; `count == 0` ne peint
rien (badge masqué). Le rouge est constant (une alerte se lit rouge quel que soit
le thème).

## Lot C — Axes supplémentaires : orientation & hauteur

`frus-core` gagne :

- `Orientation { Portrait, Landscape }` avec `Orientation::from_size(w, h)`
  (convention : carré → portrait), `is_portrait()`, `is_landscape()` ;
- `SizeClass::from_height(h)` — mêmes seuils que la largeur, pour piloter l'axe
  **vertical** (fenêtre courte → `Compact` en hauteur).

Réexportés depuis `frus-widgets` et `frus-shell`. L'app compose ces primitives
comme elle veut (le shell fournit déjà `on_resize(w, h)`).

## Démo

- **Tiroir** : un bouton « ☰ » dans l'en-tête ouvre un tiroir listant les
  sections (Tasks / Stats / About) + un accès aux réglages ; choisir une section
  ou naviguer le referme. Le geste retour est neutralisé tant qu'il est ouvert.
- **Badge** : la destination « Tasks » du `NavScaffold` affiche le nombre de
  tâches actives.
- **Orientation / hauteur** : `on_resize` journalise l'orientation ; en fenêtre
  **courte** (`from_height == Compact`), l'astuce est masquée et la liste se
  réduit (200 px au lieu de 320).

## Tests

- `frus-core` : `from_height`, `Orientation::from_size` (portrait/paysage/carré).
- `frus-widgets` : `Drawer` — overlay présent seulement si ouvert, voile +
  panneau plein-hauteur, aucun hit de fermeture si fermé ; badge — décore le bon
  élément, plafond `99+`.
- `frus-demo` : le tiroir bascule et se referme au choix d'une section / à la
  navigation ; `on_resize` suit l'orientation.

## Limites (v1)

- Pas d'animation de glissement du tiroir (ouverture/fermeture instantanées).
- `Placement::Left` uniquement (pas de tiroir à droite) — trivial à ajouter au
  besoin.
- Pas de tiroir *permanent* (toujours visible en Expanded) : c'est le rail qui
  joue ce rôle ; le tiroir reste modal.
