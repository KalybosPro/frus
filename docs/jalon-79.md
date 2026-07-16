# Jalon 79 — Live-reload préservant l'état (§13)

## Analyse

La faiblesse DX identifiée (but 4) : itérer sur la `view` demande de relancer
et de re-naviguer. Le cahier d'idées pointe l'avantage Elm : « l'état est une
struct unique ; sérialisez-la, rechargez, ré-hydratez ». Premier palier — la
**relance à chaud sur recompilation, état conservé** — sans hot-patching
(`subsecond`) ni dylib à ABI fragile : pur cargo.

## Architecture

- **`Application`** gagne deux hooks à défaut neutre : `save_state() ->
  Option<Vec<u8>>` (format libre, propriété de l'app) et
  `restore_state(&[u8])` (appelé **avant `init`** ; les octets viennent d'une
  *autre version du code* → tolérer, jamais paniquer).
- **`frus-shell/reload.rs`** : sous `FRUS_WATCH=1` (builds debug), un
  `ReloadWatcher` sonde le **mtime de l'exécutable** (700 ms) ; quand
  `cargo build` le remplace : capture de l'instantané → fichier temp →
  `spawn` du nouveau binaire avec `FRUS_HOT_STATE=<chemin>` → `exit(0)`.
  Au démarrage, `restore_from_env` lit et **consomme** l'instantané.
  - Piège évité : après remplacement, `/proc/self/exe` pointe l'inode
    supprimé (« (deleted) ») — le chemin est **capturé au démarrage**.
  - La boucle se réveille via la politique centralisée `idle_control_flow()`
    (min des échéances appui-long / sondage reload — remplace les deux
    `set_control_flow` épars).
- **Démo** : instantané ligne-à-ligne versionné (`frus-demo-state v1`) —
  tâches, brouillon, filtre, thème (mode + graine), onglet, écran empilé ;
  `init` saute le rechargement disque quand l'état vient d'un instantané
  (sinon `Loaded` l'écraserait).

## Usage

```sh
FRUS_WATCH=1 cargo run -p frus-demo     # terminal 1
cargo watch -x 'build -p frus-demo'     # terminal 2 — éditer, sauver, voir
```

Caveat Windows natif : l'exécutable en cours est verrouillé (cargo ne peut
pas le remplacer) — le flux vise Linux/WSL/macOS ; Android non concerné.

## Validation

- Tests : `restore_reads_and_consumes_the_snapshot`,
  `watcher_requires_opt_in_and_tracks_mtime` (shell) ;
  `live_reload_state_round_trips` (démo : tâches/brouillon/filtre/thème/
  graine/route survivent, `init` n'émet plus de `Loaded`, instantané corrompu
  ou d'une autre version ignoré sans panique). 275 → 278.
- **E2E réel (WSL)** : démo lancée sous `FRUS_WATCH=1`, `touch` du binaire →
  logs `binaire recompilé : relance` puis `état réhydraté (68 octets)`,
  nouveau pid. La boucle Flutter-like fonctionne.

## Reste du §13

Hot-patching intra-processus (`subsecond`) si le besoin dépasse la relance ;
template `cargo new` ; inspecteur : sélection figée au clic, état retenu.
