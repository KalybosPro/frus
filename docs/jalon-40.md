# Jalon 40 — Nouveaux widgets : Popover, Autocomplete, Kbd

Lot « overlays & saisie ». (Textes UI en anglais, conformément à la préférence.)

## Widgets

- **`Popover::new(anchor, open, on_dismiss).content(widget)`** — panneau flottant
  à **contenu libre**, ancré, contrôlé, fermeture au clic extérieur. Généralise
  `Menu` (qui n'accepte que des items d'action) ; réutilise `Portal` (auto-flip +
  voile cliquable + `overlay_dismiss`).
- **`Autocomplete::new(value, on_input, on_pick).suggestion("...")`** — champ de
  saisie (`TextInput`) avec une **liste de suggestions** flottante (via overlay
  `Below`). **Contrôlé** : l'app fournit la valeur *et* les suggestions déjà
  filtrées ; la liste ne flotte que si elle est non vide. Taper → `on_input` ;
  cliquer une suggestion → `on_pick`.
- **`Kbd::new("Enter")`** — capuchon de touche clavier (indice de raccourci) :
  petit cadre arrondi + libellé discret.

## Démo (onglet « About »)

- **`Popover`** « Info » ouvrant un panneau de détails (contenu libre).
- **`Autocomplete`** « tag » avec suggestions filtrées par la saisie
  (`State.tag_draft`, `Msg::TagInput/TagPick`).
- **`Kbd`** : ligne « Shortcuts: [Enter] add [Tab] navigate ».

## Tests

- `Popover` : fermé → pas d'overlay ; ouvert → overlay + `overlay_dismiss` ; contenu.
- `Autocomplete` : pas d'overlay sans suggestion ; sinon liste flottante ; clic
  suggestion → `on_pick(label)`.
- `Kbd` : capuchon (bordure) + libellé peints.
- 90 tests frus-widgets ; démo + chrono non régressés.

## Limites (v1)

- `Autocomplete` : pas de navigation clavier dans les suggestions (flèches) ; le
  filtrage est du ressort de l'app.
- `Popover` : placement `Below` (pas d'ancrage haut/gauche/droite au choix).
