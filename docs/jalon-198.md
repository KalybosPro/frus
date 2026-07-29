# Jalon 198 — TextInput : suffixe cliquable (clic positionnel)

## Analyse

L'icône **suffixe** d'un `TextInput` était décorative. Or beaucoup de champs veulent une action
*dans* le champ : un « ✕ » pour effacer, un œil pour révéler un mot de passe, une loupe pour
chercher. Le clic de frus est **position-indépendant** (`msg_for`, précalculé depuis `on_click`) :
impossible de distinguer « clic sur l'icône » de « clic dans le texte » sans routage positionnel.

## Décisions techniques

- **Un hook de clic positionnel dans le trait `Widget`.** Nouvelle méthode
  `positional_click(local_x, local_y, width) -> Option<Msg>` (défaut `None`), **prioritaire** sur
  `on_click`. Le shell, au relâchement sur le widget cliqué, calcule les coordonnées **locales**
  (curseur − coin du widget, via `ui.widget_rect`) et l'interroge ; s'il rend `Some`, ce message
  l'emporte, sinon on retombe sur `on_click` (comportement inchangé pour tous les widgets). La
  méthode est **forwardée** par `Box<dyn Widget>`, `Keyed` et `Responsive` pour traverser les
  enveloppes.

- **`TextInput::on_suffix(msg)`.** Rend l'icône suffixe cliquable : `positional_click` émet `msg`
  quand le clic tombe dans la **zone du suffixe** (bord droit de la boîte, `suffix_hit`) ; et
  `cursor_at` y renvoie `None` pour **ne pas** y placer le caret. Sans `on_suffix`, l'icône reste
  purement décorative.

- **Démo : bouton effacer.** Le champ de saisie des tâches, **non vide**, porte une icône « ✕ »
  (`IconName::Close`) qui émet `ClearDraft` → vide le champ. Débloque aussi, à terme, l'œil de
  révélation (avec une icône œil).

## Implémentation

- `widget.rs` : méthode `positional_click` (trait + forwarder `Box`).
- `keyed.rs` / `responsive.rs` : forwarders.
- `textinput.rs` : champ `suffix_action` + `on_suffix` ; `suffix_hit` ; `positional_click` ;
  garde dans `cursor_at`.
- `frus-shell/src/app.rs` : au relâchement, `positional_click` (coords locales) prioritaire sur
  `msg_for`.
- `frus-demo/src/lib.rs` : `Msg::ClearDraft` + icône suffixe conditionnelle sur le champ de saisie.
- `goldens.rs` : `textinput_clear`.

## Vérification

- **Unitaire** : `clickable_suffix_emits_and_blocks_caret` — clic sur le suffixe émet le message
  et ne place pas de caret ; clic dans le corps place un caret et n'émet rien ; sans `on_suffix`,
  aucun clic positionnel. `clear_draft_empties_the_field` (démo). Les tests existants restent
  **verts** (19 tests démo).
- **Golden** `textinput_clear` **inspecté** : champ « Buy milk » avec l'icône « ✕ » à droite.
- `cargo build -p frus-shell` **propre**.

## Reste

- **Icône œil de révélation** : réutilise `on_suffix` pour basculer `obscure` — reste à ajouter
  une icône œil (contour) au jeu.
- **Survol du suffixe** (curseur main / surbrillance de l'icône) — via un état de survol de
  sous-région.
