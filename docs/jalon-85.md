# Jalon 85 — Accessibilité : annotation sémantique + pont AccessKit

## Analyse

Le §14 recommande : *n'inventez PAS l'accessibilité — adoptez **AccessKit**
(le standard Rust cross-plateforme : UIA Windows, AT-SPI Linux, macOS). frus
annote la sémantique par widget (rôle/label/valeur/état), AccessKit parle aux
lecteurs d'écran natifs.* Et : *bakez le libellé dans les widgets dès
maintenant, branchez AccessKit ensuite.*

## Architecture

- **`frus-core/semantics.rs`** (zéro-dep) : `Role` (Button, CheckBox, Switch,
  Slider, TextInput, Label, ProgressBar, Tab, ListItem…), `Toggled`
  (None/False/True), et `Semantics { role, label, value, toggled, clickable,
  disabled, range }` avec des builders. Type frus-natif, **mappable** vers
  `accesskit`.
- **`Widget::semantics()`** : hook (défaut `None` = conteneur de mise en page),
  délégué par `Box`/`Keyed`/`Responsive`. Implémenté sur Button, Text,
  Checkbox, Switch, Slider, ProgressBar, TextInput.
- **Collecte** : `build_ui` récolte les nœuds porteurs de sens (rôle non nul ou
  libellé) avec leurs bornes visibles, dans l'**ordre de peinture** (= ordre de
  lecture) → `Ui::semantics()`.
- **`frus-shell/a11y.rs`** (bureau uniquement) : mappe l'arbre frus vers un
  `accesskit::TreeUpdate` (une racine `Window` + un nœud par widget), et câble
  l'adaptateur `accesskit_winit` :
  - un **instantané partagé** (`Arc<Mutex>`) écrit chaque frame, lu par
    l'`ActivationHandler` sur le thread de l'AT ;
  - une **file d'actions** : l'`ActionHandler` traduit un clic/focus demandé par
    l'AT en `A11yAction`, rejouée dans la boucle (`drain_a11y_actions` →
    `dispatch(msg)` / focus) — l'AT peut donc **lire ET activer** l'UI.
  - `Deactivation` no-op (l'arbre se reconstruit à la frame suivante).

Contrainte : `accesskit_winit` exige que la fenêtre soit **cachée** à la
création de l'adaptateur → on la crée `with_visible(false)` puis on la révèle
après (bureau seulement ; Android n'est pas concerné).

## Décisions

- Arbre **plat** sous la racine (ordre de peinture) pour ce premier jet : les
  lecteurs d'écran naviguent une liste ordonnée. Une hiérarchie fidèle
  (groupes) viendra si besoin.
- Android exclu par `cfg` : AccessKit y a un provider distinct (chantier
  séparé). Le hook sémantique, lui, est cross-plateforme et déjà baké.
- `WidgetId::from_u64` ajouté (public) : l'action venue de l'AT est réidentifiée
  vers le widget (inverse de `as_u64`).

## Tests (287 → 296)

- `frus-core` : builders `Semantics`, `is_meaningful`.
- `frus-widgets` : `semantics_tree_carries_roles_and_labels` (l'arbre porte
  rôles/labels/états ; le conteneur est ignoré).
- `frus-shell` : mapping des rôles vers AccessKit, structure du `TreeUpdate`
  (racine + enfants), focus pointant un nœud présent, aller-retour `node_id`.

## Validation

- 21 suites vertes ; smoke-run bureau : l'app tourne avec l'adaptateur (fenêtre
  cachée→révélée), **inerte** sous WSL faute de bus AT, sans crash. Android
  compile (a11y exclu).
- **Limite honnête** : la lecture réelle par un lecteur d'écran (NVDA/Orca/
  VoiceOver) n'est pas observable dans cet environnement (pas de bus AT-SPI
  sous WSL). Le pont est correct par construction et testé au niveau du mapping ;
  la validation AT de bout en bout est une tâche bureau-avec-lecteur à part.

## Reste

- Adoption progressive de `semantics()` sur plus de widgets (RadioGroup, Tabs,
  liens, images).
- Provider Android AccessKit.
- Hiérarchie sémantique (groupes/régions) si un lecteur d'écran le réclame.
