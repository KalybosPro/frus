# Démarrer une application frus

frus reste **cargo-natif** : pas d'outil propriétaire, pas de CLI maison.
`cargo run` / `cargo test` / `cargo apk run` suffisent.

## La plus petite application

Une app frus, c'est le modèle Elm au complet : une struct d'état, un `update`
**pur**, une `view`. L'exemple canonique vit dans
[`crates/frus-hello`](../crates/frus-hello/src/lib.rs) (~60 lignes, un
compteur) — copiez-le, ou générez un projet neuf avec le template ci-dessous.

```rust
impl Application for Counter {
    type Message = Msg;
    fn update(&mut self, m: Msg) -> Command<Msg> {
        match m { Msg::Increment => self.count += 1, Msg::Decrement => self.count -= 1 }
        Command::none()
    }
    fn view(&self, theme: &Theme, w: f32, h: f32) -> Box<dyn Widget<Msg>> { /* … */ }
}
```

Lancer sur bureau :

```sh
cargo run -p frus-hello
```

## Générer un nouveau projet (`cargo generate`)

Le template [`templates/app`](../templates/app) produit un projet frus prêt à
lancer (bureau + Android).

```sh
cargo install cargo-generate          # une seule fois
cargo generate --path templates/app --name my-app
cd my-app
cargo run
```

`cargo generate` demande le **chemin de votre checkout frus** (celui qui
contient `crates/`) — c'est là que le `Cargo.toml` généré pointe ses
dépendances, tant que frus n'est pas publié sur crates.io. Une fois publié,
les dépendances deviendront de simples `frus-shell = "0.1"`.

## Android

Le template inclut le point d'entrée `android_main` et les métadonnées
`cargo-apk`. Depuis le projet généré :

```sh
cargo install cargo-apk              # une seule fois
cargo apk run                        # build + install + lance sur l'appareil
```

Prérequis Android : SDK + NDK installés, `ANDROID_HOME`/`ANDROID_NDK_ROOT`
définis, un appareil branché (`adb devices`).

## Tester

`update` étant pur, la logique se teste **sans GPU ni fenêtre** :

```sh
cargo test
```

Pour les tests de rendu (snapshots/goldens), voir
[`frus-test`](../crates/frus-test/src/lib.rs).
