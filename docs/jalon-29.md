# Jalon 29 — Navigation clavier / accessibilité

Le focus n'existait qu'au **clic**, et seul `TextInput` le recevait. Ce jalon
ajoute la navigation **Tab**, l'**activation clavier** des contrôles, et un
**anneau de focus visible** — l'a11y clavier de base.

## Ce qui est ajouté

- **Focusables** : `Button`, `Checkbox`, `Switch` renvoient `focusable() = true`.
  C'est **tout** ce qu'un widget doit faire pour devenir accessible → a11y quasi
  gratuite (DX). Un widget accessible = focusable + `on_click`.
- **Tab / Shift+Tab** (shell) : `Ui::focus_next(courant, sens)` parcourt
  `ui.focusables` (déjà collectés en ordre d'arbre = ordre visuel), avec bouclage.
  Fonctionne même **sans focus** initial (→ premier / dernier).
- **Activation** (shell) : sur **Entrée / Espace**, si le focalisé est focusable et
  a un `on_click`, on émet ce message (bouton / case / interrupteur). Les champs
  texte (sans `on_click`) retombent sur l'édition (Entrée = soumettre, Espace =
  espace) — la présence d'`on_click` distingue proprement les deux.
- **Anneau de focus générique** (`build_ui`) : dessiné autour du focalisé
  focusable, couleur `theme.focus`, intensité animée par `focus_progress`. Un
  widget qui gère son propre focus l'inhibe via `draws_own_focus() = true`
  (`TextInput`, qui garde sa bordure).

## Trait

```rust
fn draws_own_focus(&self) -> bool { false }   // ajout (défaut : anneau générique)
```

Aucune surface applicative nouvelle : les widgets existants deviennent navigables
au clavier **sans aucun changement côté app**.

## Tests

- `Ui::focus_next` : ordre + bouclage, premier/dernier sans focus.
- `Button`/`Checkbox`/`Switch` focusables (via `focusables.len()`).
- Anneau : un bouton focalisé ajoute une primitive bordée `theme.focus` ; un
  `TextInput` focalisé n'en ajoute pas (gère le sien).
- 43 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Slider` / `Dropdown` / `RadioGroup` : navigation fine au clavier (flèches)
  reportée ; ils deviendront focusables ensuite.
- Pas de rôles/annonces ARIA (winit n'expose pas d'API lecteur d'écran) : a11y =
  **clavier + focus visible** pour l'instant.
