# Jalon 83 — Démarrage en une commande (`cargo generate`) : clôture du §13

## Analyse

Dernier maillon DX du §13 : « `cargo new --template frus-app` marche, restez
dans cargo autant que possible ». Un nouveau venu doit obtenir une app frus qui
tourne **sans outil propriétaire**, juste avec l'écosystème cargo standard.

## Livrables

### 1. `crates/frus-hello` — l'app canonique minimale
La plus petite app frus complète : un **compteur** (~60 lignes) — état, `update`
pur, `view`, entrées bureau **et** Android. Étant un membre du workspace, elle
**compile et se teste à chaque `cargo test --workspace`** : la référence ne
peut pas pourrir. C'est le « Hello, world! » du framework et la source du
template.

### 2. `templates/app` — le gabarit `cargo generate`
Le même compteur, paramétré (Liquid) :
- `{{project-name}}` / `{{crate_name}}` pour le nommage ;
- `{{frus_path}}` : chemin du checkout frus (frus n'étant pas encore publié sur
  crates.io, les dépendances sont des `path = …` ; un commentaire indique le
  passage à `frus-shell = "0.1"` une fois publié).
- `src/bin/{{project-name}}.rs` : le binaire bureau délègue à la lib.
- Métadonnées `cargo-apk` incluses (Android prêt).

Exclu du workspace (`exclude = ["templates"]`) : ses fichiers `{{…}}` ne sont
pas du Rust compilable.

### 3. `docs/getting-started.md`
Le parcours complet : plus petite app, `cargo generate`, `cargo run`,
`cargo apk run`, `cargo test`.

## Usage

```sh
cargo install cargo-generate
cargo generate --path templates/app --name my-app
cd my-app && cargo run
```

## Validation

- `cargo test --workspace` : 21 suites vertes (frus-hello ajouté ; son test
  `counting_is_pure` illustre l'argument Elm — logique testée sans GPU).
- **Template rendu → projet buildable** : placeholders substitués
  (`hello-app`), `cargo build` OK et `cargo test` vert dans le projet généré
  (hors workspace, dépendances `path` vers frus).
- `frus-hello` tourne comme le demo (même chemin `frus_shell::run`, exit
  identique au smoke-run WSL).
- Warning `Dimension` inutilisé (résidu du jalon Alert-paragraphe) nettoyé au
  passage.

## §13 clos

Tests headless/goldens (`frus-test`, J77), inspecteur runtime (J78),
live-reload (J79), et maintenant démarrage cargo-natif : la DX du §13 est
couverte. Restent hors §13 : §14 (RTL/i18n), AccessKit (a11y).
