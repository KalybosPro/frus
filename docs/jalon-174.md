# Jalon 174 — Piège de focus des menus ouverts

## Analyse

Les overlays **modaux** (voilés : modale, tiroir) **piégeaient** déjà le focus clavier —
Tab/flèches bouclent dans leurs focusables tant qu'ils sont ouverts (scope de focus). Mais
les overlays **ancrés** (`Placement::Below`) ne piégeaient pas : un **menu** ouvert (menu de
colonne du jalon 172, menu flottant) laissait Tab s'échapper vers la page derrière. Le motif
clavier attendu d'un menu est pourtant : focus **dans** les items, Échap pour sortir.

Piéger **tous** les overlays ancrés serait faux : un **tooltip** ne prend pas le focus, et la
liste d'une **autocomplétion** garde le focus sur le champ (les flèches y naviguent les
suggestions). Il fallait donc un piège **opt-in**.

## Décisions techniques

- **Opt-in via `Widget::overlay_traps_focus()`.** Nouvelle méthode de trait (défaut `false`,
  forwardée `Box`/`Keyed`/`Responsive`). Un overlay ancré ne piège le focus que s'il la
  renvoie `true`. Les overlays **modaux** piègent toujours (inchangé).

- **Seul le `Menu` s'y inscrit (pour l'instant).** `Menu::overlay_traps_focus` renvoie
  `self.open` : un menu **ouvert** piège ses items ; fermé, non. `Échap` / clic extérieur
  ferme via `on_dismiss` (déjà en place) — pas de cul-de-sac. `Dropdown`, `Autocomplete`,
  tooltips gardent le défaut `false` (comportement inchangé).

- **Drapeau porté par l'overlay différé.** Le tuple d'overlay différé gagne un booléen
  `traps`, lu du widget porteur au moment de l'empiler ; à la pose, le scope de focus démarre
  si l'overlay est **modal OU piégeant**.

## Implémentation

- `widget.rs` : `overlay_traps_focus()` (défaut `false`) + forwarders (`Box`, `keyed.rs`,
  `responsive.rs`).
- `menu.rs` : `Menu::overlay_traps_focus` = `self.open`.
- `ui.rs` : booléen `traps` dans le tuple d'overlay différé (type + `push` + `pop`) ; le
  scope de focus démarre si `modal || traps`.

## Vérification

- **Unitaire** : `open_menu_traps_focus_in_its_items` — un `Menu` ouvert piège Tab dans ses
  items (« one » → « two » → boucle), le fond est hors scope (pointeur) ; un menu **fermé**
  ne piège pas (Tab commence au fond).
- Non-régression : `modal_traps_tab_arrows_and_pointer_focus` (modales) et les tests
  d'autocomplétion/tooltip restent **verts** (ils ne piègent pas). `cargo test --workspace` **vert**.

## Reste

- **`Dropdown` en menu de colonne** : lui faire renvoyer `overlay_traps_focus` selon son
  état ouvert le piégerait aussi — à activer si l'UX le demande (la sélection unique diffère
  d'un menu d'actions).
- **Retour du focus à l'ancre** à la fermeture du menu : le shell pourrait restaurer le focus
  sur le déclencheur (motif complet « roving focus ») — extension.
