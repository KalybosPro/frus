# Jalon 165 — Accessibilité : annonces vocales (région live)

## Analyse

Le réordonnancement de colonnes offrait un repère **visuel** (fantôme + coulissement) et,
au repos, une position **re-lisible** (« column N of M » dans la sémantique de l'en-tête).
Mais un utilisateur de lecteur d'écran ne « voit » ni le fantôme, ni le glissement : au
moment du dépôt (souris) ou du pas clavier (Ctrl+Flèche), **rien n'était énoncé**. Il
fallait une **région live** — l'équivalent de `SemanticsService.announce` de Flutter ou de
`aria-live="polite"` du Web : un texte que la technologie d'assistance lit **quand il
change**, sans déplacer le focus.

## Décisions techniques

- **Un nœud live dédié, poli.** Le pont AccessKit gagne un nœud réservé (`LIVE_ID`,
  hors de portée des `WidgetId` décalés de `+1`), enfant de la racine, de rôle `Label` et
  marqué `Live::Polite` (n'interrompt pas la lecture en cours). Son libellé porte le message.
  Il n'apparaît que **si un message est présent** (aucun coût sinon).

- **Re-énoncé au seul changement.** Le message **persiste** dans l'instantané : reconduit
  tel quel à chaque frame, AccessKit ne le répète pas (il ne parle qu'au changement de
  texte). Le shell n'a donc rien à « effacer » — il pose un nouveau texte, l'annonce part.

- **Piloté par le shell, générique.** Une méthode `set_announcement(String)` (bureau
  uniquement, no-op sur Android/Web) alimente le champ, publié chaque frame via
  `a11y.update(..., announce)`. N'importe quel événement du shell peut ainsi énoncer.

- **Les deux chemins de réordonnancement.** Au **dépôt** d'un glissé et au **pas clavier**
  (Ctrl+Flèche — le chemin des utilisateurs de lecteur d'écran, qui ne glissent pas), le
  shell énonce « Column moved to position N ».

## Implémentation

- `a11y.rs` : constante `LIVE_ID`, `live_node(message)` (rôle `Label` + `Live::Polite`),
  `build_tree_update` prend `announce: &str` (nœud live + enfant de racine si non vide),
  `Snapshot.announce`, `A11y::update(..., announce)`.
- `app.rs` : champ `announce: String`, `set_announcement`, appelée au dépôt (`pointer_up`,
  branche `Drag::Reorder`) et au pas clavier (branche `on_key` Gauche/Droite d'un en-tête
  réordonnable) ; passée à `a11y.update`.

## Vérification

- **Unitaire** : `announcement_adds_a_polite_live_region` — sans message, aucun nœud live ;
  avec message, un nœud `Live::Polite` portant le texte, référencé par la racine.
- Suite `frus-shell` **verte** (23 tests). `cargo test --workspace` **vert**.

## Reste

- **Débit** : deux annonces **identiques** consécutives (même position atteinte deux fois)
  ne se répètent pas — acceptable ici ; un bascule (espace insécable) forcerait la reprise.
- Étendre les annonces à d'autres gestes (sélection multiple « N selected », tri « sorted
  by X ascending »), sur le même mécanisme.
- Le provider AccessKit **Android** reste un chantier distinct (bureau uniquement ici).
