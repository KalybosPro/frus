# Jalon 136 — Focus programmatique (rendre `first_invalid` actionnable)

## Analyse

Le jalon 135 sait dire **quel** champ est en échec (`Form::first_invalid`), mais
l'application ne pouvait pas **y sauter** : le focus vit dans le `Runtime` du shell, hors
d'atteinte de l'app. Il manquait un canal app → shell pour **demander le focus** d'un
champ — l'équivalent du `FocusNode` de Flutter ou du `text_input::focus(id)` d'Iced, mais
dans l'esprit Elm (une commande, pas un objet mutable).

## Décisions techniques

- **Focaliser par clé, pas par identité positionnelle.** Un `WidgetId` frus est
  positionnel (chemin dans l'arbre) — l'app ne peut pas le calculer. Mais un widget
  enveloppé par `keyed(k, …)` a une **identité stable dérivée de sa clé**
  (`parent.keyed(hash(k))`). L'app référence donc un champ par la **même clé** qu'elle lui
  a donnée : `keyed("email", TextInput…)` puis `Command::focus("email")`. Zéro nouvelle
  API de widget — on réutilise le mécanisme de clé existant.

- **`Command::focus(key)`, résolu après le build.** `Command` porte désormais, en plus de
  ses tâches, des **demandes de focus** (clés hachées comme `keyed`). Le shell les met en
  attente puis les résout contre l'arbre **fraîchement reconstruit** de la frame suivante
  (l'état a changé → la vue est rebâtie de toute façon), via `find_by_key` — qui rend
  l'identité du premier widget portant cette clé. La plus récente demande qui se résout
  l'emporte, et l'anneau de focus redevient visible (on « saute » au champ).

- **La résolution vise l'identité de focus réelle.** `find_by_key` calcule l'identité par
  le **même** `child_id` que le rendu, la collecte et le hit-test. Poser
  `runtime.input.focused` sur ce résultat route donc l'édition et le curseur exactement
  comme un clic (un test le vérifie : `find_by_key == focus_hit`).

- **`run_command` unifié.** Les deux versions (natif thread / Web `spawn_local`)
  fusionnent en une seule (`&mut self`) qui, en plus de lancer les tâches, empile les
  demandes de focus — un seul chemin, un `cfg` sur la seule ligne de lancement.

## Implémentation

- `frus-shell/src/command.rs` : `Command` gagne `focus: Vec<u64>` ; constructeur
  `focus(key)` (hash `DefaultHasher`, identique à `keyed`) ; `batch` fusionne ; `is_empty`
  compte le focus ; `into_parts()` remplace `into_tasks()`. Test du portage de clé.
- `frus-widgets/src/ui.rs` : `find_by_key(root, key) -> Option<WidgetId>` (exporté). Test :
  résolution distincte par clé, égale à l'identité de focus (`focus_hit`), `None` si
  inconnue.
- `frus-shell/src/app.rs` : champ `pending_focus` ; `run_command` unifié empile le focus ;
  après le build, résolution contre l'arbre frais → `runtime.input.focused`.

## Usage

```rust
// view : nommer les champs.
keyed("email", TextInput::new(&self.email).label("Email") /* .error(...) */)

// update : à la soumission, sauter au premier champ invalide.
let report = Form::new().field("email", &self.email, Rule::email("…")) /* … */;
if let Some(key) = report.first_invalid() {
    return Command::focus(key);
}
```

## Vérification

- **Unitaires** : `Command::focus` porte la bonne clé sans tâche ; `find_by_key` résout
  chaque clé vers une identité distincte, **égale à celle du hit-test de focus**, et rend
  `None` pour une clé inconnue.
- **Multi-cible** : compile natif **et** `wasm32` (le `run_command` unifié) ; suites
  `frus-widgets` + `frus-shell` vertes.

## Reste

- L'exemple complet (form app qui saute au premier champ) reste un **applicatif** à
  écrire ; le mécanisme, lui, est en place et testé.
- **Défilement vers le champ focalisé** (si hors écran) : à ajouter le jour où l'on aura de
  longs formulaires défilants.
