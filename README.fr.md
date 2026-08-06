<div align="center">

<img src="crates/frus-shell/assets/logo.png" alt="frus" width="140" height="140">

# frus

**Un framework UI multiplateforme écrit entièrement en Rust.**

Un seul code → bureau, Android et le Web. Rendu GPU. Architecture Elm. Pas de DSL, pas de génération de code, pas de CLI maison — juste `cargo`.

[![CI](https://github.com/KalybosPro/frus/actions/workflows/ci.yml/badge.svg)](https://github.com/KalybosPro/frus/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#licence)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![Statut](https://img.shields.io/badge/statut-pré--alpha-yellow.svg)](#état-du-projet)

[Démarrer](#démarrer) · [Architecture](#architecture) · [État](#état-du-projet) · [Contribuer](CONTRIBUTING.md) · [English](README.md)

</div>

---

## Qu'est-ce que frus ?

frus est une tentative *greenfield* du framework que Flutter serait s'il avait été conçu en Rust dès le premier jour : **tout le framework — rendu, mise en page, widgets, gestes, thèmes, animations, accessibilité — est en Rust**. Aucune VM embarquée, aucun second langage pour la logique applicative, aucun canal de plateforme entre votre code et les pixels.

Les parties qui *doivent* être natives (création de fenêtre, IME, lecteurs d'écran, activité Android) vivent derrière une seule crate mince, `frus-shell`. Tout ce qui est au-dessus est portable.

```rust
use frus::{button, column, row, text, Align, Application, Command, Theme, Variant, Widget};

#[derive(Default)]
struct Counter { count: i32 }

#[derive(Clone)]
enum Msg { Increment, Decrement }

impl Application for Counter {
    type Message = Msg;

    // `update` est pur — testable sans GPU ni fenêtre.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
        }
        Command::none()
    }

    fn view(&self, _theme: &Theme, _w: f32, _h: f32) -> Box<dyn Widget<Msg>> {
        Box::new(column![
            text(format!("{}", self.count)).size(48.0),
            row![
                button("+", Msg::Increment).variant(Variant::Primary),
                button("−", Msg::Decrement).variant(Variant::Secondary),
            ].gap(20.0),
        ].gap(16.0).align(Align::Center))
    }
}

// Une seule déclaration câble les points d'entrée bureau, Android et Web.
frus::main!(Counter::default());
```

C'est une application complète et exécutable. `cargo run` sur bureau, `cargo apk run` sur Android, `wasm-bindgen` pour le navigateur — la source ne change pas.

### Pourquoi un framework de plus ?

| | |
|---|---|
| **Un seul langage, de bout en bout** | Logique applicative, widgets, layout et moteur de rendu sont tous en Rust. Pas de frontière FFI dans le chemin chaud, pas de sérialisation à travers un pont. |
| **`update` pur, cœur testable** | L'architecture Elm fait de votre machine à états une fonction pure. ~700 tests de ce dépôt tournent sans GPU ni fenêtre. |
| **Rendu GPU natif** | `wgpu` vise Vulkan, Metal, DX12 et WebGPU depuis un seul backend. Les chemins vectoriels sont tessellés par `lyon`, le texte façonné par `cosmic-text`. |
| **Tout est surchargeable** | Les widgets fournissent des valeurs par défaut *thémées*, jamais codées en dur. Si un widget le dessine, vous pouvez le restyler ou remplacer l'emplacement. |
| **cargo-natif** | Pas de `frus doctor`, pas de gestionnaire de paquets maison, pas de dossier de build généré. `cargo build`, `cargo test`, `cargo apk run`. |

## Démarrer

**Prérequis :** une toolchain Rust stable récente et un GPU avec des pilotes Vulkan, Metal ou DX12. (Aucune version minimale de Rust n'est encore fixée — le développement se fait sur la stable courante.)

```sh
git clone https://github.com/KalybosPro/frus
cd frus

cargo run -p frus-hello        # le compteur ci-dessus
cargo run -p frus-demo         # une app plus large (todo / kanban)
cargo run -p frus-transforms   # vitrine d'animations et de transforms
cargo test --workspace         # ~700 tests
```

### Créer votre propre application

Le dépôt fournit un template `cargo generate` qui produit un projet câblé pour le bureau **et** Android :

```sh
cargo install cargo-generate                          # une fois
cargo generate --path templates/app --name my-app
cd my-app && cargo run
```

Le template demande le chemin de votre checkout frus — frus n'est pas encore sur crates.io, les dépendances passent donc par `path`. Voir [`docs/getting-started.md`](docs/getting-started.md).

### Android

```sh
cargo install cargo-apk        # une fois
cargo apk run -p frus-demo     # build, installe, lance
```

Nécessite le SDK + NDK Android avec `ANDROID_HOME` / `ANDROID_NDK_ROOT` définis et un appareil visible par `adb devices`.

### Web

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

cargo build -p frus-hello --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript \
  --out-dir crates/frus-hello/web/pkg \
  target/wasm32-unknown-unknown/web-release/frus_hello.wasm

cd crates/frus-hello/web && python3 -m http.server 8080
```

Nécessite un navigateur WebGPU (Chrome/Edge 113+) sur un contexte sécurisé. Détails dans [`crates/frus-hello/web/README.md`](crates/frus-hello/web/README.md).

## Architecture

Quatre couches. Les dépendances ne pointent que vers le bas, et seul `frus-shell` sait sur quelle plateforme il tourne.

```
┌──────────────────────────────────────────────────────────────┐
│  Application     ce que vous écrivez                         │
│  frus (façade) · frus-hello · frus-demo · frus-transforms    │
├──────────────────────────────────────────────────────────────┤
│  Shell           couche plateforme                           │
│  frus-shell — Application, Command, Subscription,            │
│               cycle de vie, IME, a11y, net, main!            │
├──────────────────────────────────────────────────────────────┤
│  Widgets         UI & interaction                            │
│  frus-widgets — Ui/scène, arbre de widgets, gestes, thème    │
├──────────────────────────────────────────────────────────────┤
│  Fondations      rendu & mesure                              │
│  frus-core · frus-layout · frus-text · frus-gpu ·            │
│  frus-image · frus-l10n                                      │
└──────────────────────────────────────────────────────────────┘
```

| Crate | Rôle |
|---|---|
| [`frus`](crates/frus) | Façade — la dépendance unique d'une application. Ré-exporte shell + widgets + `main!`. |
| [`frus-core`](crates/frus-core) | Géométrie, couleur (dont HCT), chemins, décorations, styles de texte, animation, graphe de scène, sémantique. |
| [`frus-layout`](crates/frus-layout) | Mise en page flexbox au-dessus de [`taffy`](https://github.com/DioxusLabs/taffy). |
| [`frus-text`](crates/frus-text) | Shaping et mesure via [`cosmic-text`](https://github.com/pop-os/cosmic-text). |
| [`frus-gpu`](crates/frus-gpu) | Device `wgpu`, peintre 2D, tessellation de chemins, atlas de glyphes, compositeur, rendu hors écran. |
| [`frus-image`](crates/frus-image) | Décodage PNG/JPEG vers `ImageData`. |
| [`frus-l10n`](crates/frus-l10n) | i18n via bundles Fluent + négociation de locale. |
| [`frus-widgets`](crates/frus-widgets) | La bibliothèque de widgets et le modèle d'interaction (~80 modules). |
| [`frus-shell`](crates/frus-shell) | Fenêtre, boucle d'événements, cycle de vie, `Command`/`Subscription`, IME, AccessKit, `fetch`. |
| [`frus-test`](crates/frus-test) | Rendu headless, snapshots, comparaison d'images de référence. |
| [`frus-hello`](crates/frus-hello) | L'application minimale canonique. Source du template `cargo generate`. |
| [`frus-demo`](crates/frus-demo) | Application d'exemple plus large, exerçant la plupart des widgets. |
| [`frus-fetch-example`](crates/frus-fetch-example) | Exemple réseau de bout en bout : `RemoteData`, états chargement/erreur/donnée. |
| [`frus-transforms`](crates/frus-transforms) | Vitrine animée : transforms, ratio d'aspect, dimensionnement fractionnaire. |

Lisez [ARCHITECTURE.md](ARCHITECTURE.md) avant votre première modification non triviale — le document explique où va quel type de code, et pourquoi.

## État du projet

**Pré-alpha.** Le cœur est réel et exercé par trois applications d'exemple, mais l'API n'est pas stable et rien n'est publié sur crates.io.

| Plateforme | État | Notes |
|---|---|---|
| **Bureau** (Windows / Linux / macOS) | Fonctionnel | winit + wgpu, presse-papier, accessibilité lecteur d'écran via AccessKit, live-reload en dev |
| **Android** | Fonctionnel | Activité native, Vulkan, IME réel (composition et swipe), insets, cycle de vie — validé sur appareil |
| **Web** (wasm + WebGPU) | Fonctionnel | Rendu, entrée, animations, souscriptions, effets async et `fetch`. Presse-papier, a11y et live-reload non câblés |
| **iOS / macOS natif** | Non démarré | La couche shell est isolée : ajouter une cible reste un chantier circonscrit |

**Ce qui marche aujourd'hui :** mise en page flex/grille/wrap, défilement 1D et 2D avec fill-then-scroll, saisie de texte avec IME, glisser-déposer avec reflow en direct, tables de données, grilles éditables, graphiques, sélecteurs de date/heure, listes déroulantes, arbres, toasts, modales, tiroirs, navigation à transitions ressort et geste de retour, thème surchargeable, RTL et i18n, animations à ressort, cycle de vie, effets et souscriptions, HTTP asynchrone avec JSON typé, et tests par images de référence.

**Manques connus** — les meilleurs points d'entrée pour aider :

- Publication sur crates.io (tout passe par `path` aujourd'hui).
- Presse-papier, accessibilité et live-reload sur le Web.
- Shells iOS et macOS natif.
- Cas limites du rendu de texte, et couverture golden plus large.
- Documentation en anglais (les notes de conception sont en français).

Voir [ROADMAP.md](ROADMAP.md) et les [*good first issues*](https://github.com/KalybosPro/frus/labels/good%20first%20issue).

## Contribuer

Les contributions sont les bienvenues — le projet est assez jeune pour qu'une seule PR façonne un sous-système.

Commencez par **[CONTRIBUTING.md](CONTRIBUTING.md)**. En résumé :

```sh
cargo test --workspace          # doit être vert
cargo clippy --workspace --all-targets
cargo fmt --all
```

Chaque changement arrive avec ses tests ; chaque changement non trivial arrive avec sa note de conception. Les discussions ont lieu dans les [issues](https://github.com/KalybosPro/frus/issues) et les [discussions](https://github.com/KalybosPro/frus/discussions) — en anglais ou en français, les deux conviennent.

En participant, vous acceptez le [Code de conduite](CODE_OF_CONDUCT.md).

## Documentation

- [Démarrer](docs/getting-started.md) — écrire et lancer votre première application
- [Architecture](ARCHITECTURE.md) — comment les crates s'assemblent
- [Feuille de route](ROADMAP.md) — la suite, et où l'aide est souhaitée
- [Index des notes de conception](docs/README.md) — 276 notes, une par jalon : l'analyse, les alternatives, la décision et ses raisons. C'est la mémoire réelle du projet.

## Licence

Sous licence, au choix :

- Licence Apache, version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- Licence MIT ([LICENSE-MIT](LICENSE-MIT))

Sauf mention contraire explicite de votre part, toute contribution soumise intentionnellement pour inclusion dans ce travail, telle que définie par la licence Apache-2.0, sera doublement licenciée comme ci-dessus, sans terme ni condition supplémentaire.
