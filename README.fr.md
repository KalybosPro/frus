<div align="center">

<img src="crates/frus-shell/assets/logo.png" alt="frus" width="140" height="140">

# frus

**Un framework UI multiplateforme écrit entièrement en Rust.**

Un seul code → bureau, Android et le Web. Rendu GPU. Architecture Elm. Pas de DSL, pas de génération de code, pas de CLI maison — juste `cargo`.

[![CI](https://github.com/KalybosPro/frus/actions/workflows/ci.yml/badge.svg)](https://github.com/KalybosPro/frus/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#licence)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![Statut](https://img.shields.io/badge/statut-pré--alpha-yellow.svg)](#état-du-projet)

[Démarrer](#démarrer) · [Galerie](#à-quoi-ça-ressemble) · [Architecture](#architecture) · [État](#état-du-projet) · [Contribuer](CONTRIBUTING.md) · [English](README.md)

<br>

<img src="docs/media/tour.gif" alt="Quatre écrans de l'application de démonstration, ouverts et refermés par les transitions à ressort de frus" width="620">

<sub>L'application d'exemple passant d'un écran à l'autre. Chaque pixel — la mise en page, la typographie, les graphiques, les ressorts — est dessiné par frus sur le GPU.</sub>

</div>

---

## Qu'est-ce que frus ?

frus est une tentative *greenfield* de framework UI conçu en Rust dès le premier jour : **tout le framework — rendu, mise en page, widgets, gestes, thèmes, animations, accessibilité — est en Rust**. Aucune VM embarquée, aucun second langage pour la logique applicative, aucun canal de plateforme entre votre code et les pixels.

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
| **`update` pur, cœur testable** | L'architecture Elm fait de votre machine à états une fonction pure. ~970 tests de ce dépôt tournent sans GPU ni fenêtre. |
| **Rendu GPU natif** | `wgpu` vise Vulkan, Metal, DX12 et WebGPU depuis un seul backend. Les chemins vectoriels sont tessellés par `lyon`, le texte façonné par `cosmic-text`. |
| **Tout est surchargeable** | Les widgets fournissent des valeurs par défaut *thémées*, jamais codées en dur. Si un widget le dessine, vous pouvez le restyler ou remplacer l'emplacement. |
| **cargo-natif** | Pas de `frus doctor`, pas de gestionnaire de paquets maison, pas de dossier de build généré. `cargo build`, `cargo test`, `cargo apk run`. |

## À quoi ça ressemble

Tout ceci est **une seule** application — `crates/frus-demo` — et un seul arbre de sources.

| | |
|:--:|:--:|
| <img src="docs/media/tasks.png" alt="La liste de tâches : barre d'application, alertes, contrôle segmenté, champ de saisie, cases à cocher, cibles de glisser-déposer et bouton d'action flottant" width="440"> | <img src="docs/media/charts.png" alt="Un tableau de bord : une courbe avec légende cliquable au-dessus d'un histogramme groupé" width="440"> |
| **Widgets, gestes, thème** — la liste, avec réorganisation par glisser-déposer et balayage pour supprimer. | **Graphiques** — courbes, aires, barres groupées et empilées, avec une légende qui filtre les séries. |
| <img src="docs/media/board.png" alt="Un tableau Kanban de trois colonnes de cartes" width="440"> | <img src="docs/media/data.png" alt="Un tableau de données avec recherche, en-têtes triables, cases de sélection et pagination" width="440"> |
| **Glisser-déposer** — les cartes changent de colonne et le reste se réagence en direct sous le doigt. | **Tableaux de données** — tri, sélection, pagination, et une variante éditable en ligne. |

<table>
<tr>
<td width="34%" align="center">
<img src="docs/media/android.png" alt="La même application sur un téléphone Android" width="240">
</td>
<td>

**Le même code sur un téléphone.** Android est une cible de premier rang, pas un
portage : activité native, Vulkan, une vraie saisie IME avec composition et frappe
glissée, marges système, cycle de vie. C'est une photographie d'un appareil, pas un
rendu — c'est la seule image ici qui ne vaudrait rien autrement.

<img src="docs/media/light.png" alt="L'écran des réglages en thème clair" width="380">

**Le thème n'est pas une couche de peinture.** Clair et sombre sont engendrés à
partir d'une couleur graine, et chaque widget prend ses couleurs du thème plutôt que
d'une constante — une application peut donc restyler toute la bibliothèque, ou un
seul widget, sans la forker.

</td>
</tr>
</table>

<sub>Toutes les images ci-dessus sauf le téléphone sont <b>rendues</b>, par le même pipeline
qu'une fenêtre : <code>cargo run -p frus-demo --features shots --bin shots -- docs/media</code>.
On les régénère après un changement au lieu de les laisser se périmer.</sub>

## Démarrer

**Prérequis :** une toolchain Rust stable récente et un GPU avec des pilotes Vulkan, Metal ou DX12. (Aucune version minimale de Rust n'est encore fixée — le développement se fait sur la stable courante.)

```sh
git clone https://github.com/KalybosPro/frus
cd frus

cargo run -p frus-hello        # le compteur ci-dessus
cargo run -p frus-demo         # une app plus large (todo / kanban)
cargo run -p frus-transforms   # vitrine d'animations et de transforms
cargo test --workspace         # ~970 tests
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
- Un site de documentation navigable, construit à partir des notes de conception.

Voir [ROADMAP.md](ROADMAP.md) pour le tableau complet.

## Par où commencer

Le projet est assez jeune pour qu'une seule *pull request* façonne un sous-système.
Celles-ci sont réelles, ouvertes, et rédigées avec où regarder et comment savoir que
c'est fini :

| | |
|---|---|
| 🟢 [Un README par crate](https://github.com/KalybosPro/frus/labels/good%20first%20issue) | Quinze crates, aucune page d'accueil. **Une seule crate fait une très bonne PR.** |
| 🟢 Fixer une version minimale de Rust | Personne ne sait où est le plancher. Le trouver, le fixer, l'ajouter à la CI. |
| 🟢 `NavBar` se recroqueville sur son bouton retour | Un petit bug réel, déjà diagnostiqué, avec de quoi le voir. |
| 🟡 [Publier sur crates.io](https://github.com/KalybosPro/frus/labels/help%20wanted) | Le principal obstacle entre le projet et quiconque voudrait l'essayer. |
| 🟡 Le planificateur de lots est en O(n²) | 16× les primitives coûtent 127× le temps. Le benchmark est fourni. |
| 🟡 Presse-papiers et accessibilité sur le Web | Les deux existent sur bureau ; le Web les laisse tomber. |
| 🔴 [Une couche iOS](https://github.com/KalybosPro/frus/labels/design%20first) | L'architecture parie que c'est un travail circonscrit. Personne n'a testé le pari. |

🟢 *good first issue* · 🟡 *help wanted* · 🔴 *design first* — [toutes les issues ouvertes](https://github.com/KalybosPro/frus/issues)

Vous ne savez pas où vous situer ? Ouvrez une issue en disant ce que vous aimez faire.
En français ou en anglais, les deux conviennent.

## Contribuer

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
- [Structurer une application](docs/app-structure.md) — découper une application qui grandit en modules
- [Architecture](ARCHITECTURE.md) — comment les crates s'assemblent
- [Feuille de route](ROADMAP.md) — la suite, et où l'aide est souhaitée
- [Index des notes de conception](docs/README.md) — 305 notes, une par jalon : l'analyse, les alternatives, la décision et ses raisons. C'est la mémoire réelle du projet.

## Licence

Sous licence, au choix :

- Licence Apache, version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- Licence MIT ([LICENSE-MIT](LICENSE-MIT))

Sauf mention contraire explicite de votre part, toute contribution soumise intentionnellement pour inclusion dans ce travail, telle que définie par la licence Apache-2.0, sera doublement licenciée comme ci-dessus, sans terme ni condition supplémentaire.
