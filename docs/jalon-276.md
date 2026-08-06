# Jalon 276 — Préparer le terrain iOS : des `cfg` de plateforme nommés

## Objectif

Ce jalon **ne construit pas** le shell iOS. Il enlève la mine posée sous ses pieds.

Jusqu'ici, « bureau » s'écrivait dans `frus-shell` **par la négative** :

```rust
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
```

Tant qu'il n'existe que trois plateformes, c'est exact. Dès qu'on ajoute iOS, **iOS tombe
dans cette branche** et hérite du presse-papier `arboard`, d'`env_logger` et d'AccessKit —
trois choses sans backend UIKit. Le code aurait peut-être compilé, et il aurait été faux.

Le défaut est structurel : la formulation encode « les autres » au lieu d'encoder « bureau ».

## Alternatives pesées

1. **Étendre la liste à chaque site** — `not(any(target_os = "android", target_os = "ios",
   target_arch = "wasm32"))`. Mécanique, sans machinerie nouvelle. Rejeté : 66 sites, une
   liste qui rallonge à chaque plateforme, et **chaque site est une occasion d'oubli**
   silencieux. Le prochain portage (macOS natif, embarqué) rejouerait le même bug.
2. **Features Cargo** (`--features desktop`). Rejeté : la plateforme n'est pas un choix de
   l'utilisateur, et une feature oubliée donnerait un binaire sans presse-papier plutôt
   qu'une erreur de compilation.
3. **`build.rs` + `cfg_aliases`** — retenu. C'est exactement ce que font **winit et wgpu**,
   nos deux dépendances de socle. La crate est **déjà dans l'arbre** (winit s'en sert) :
   zéro dépendance téléchargée en plus.

## Décision

`crates/frus-shell/build.rs` nomme quatre plateformes :

```rust
web:     { target_arch = "wasm32" },
android: { target_os = "android" },
ios:     { target_os = "ios" },
desktop: { not(any(web, android, ios)) },
```

Ajouter une cible ne touche désormais **que ce fichier**.

> `desktop` est écrit en réutilisant les alias précédents, et non avec la liste
> `target_os`/`target_arch` complète : sous cette dernière forme, `cfg_aliases` sature sa
> limite de récursion (`recursion limit reached while expanding $crate::cfg_aliases!`).

### Deux frontières que les alias ne franchissent pas

Ce sont les deux pièges du dispositif, tous deux documentés dans le code :

- **La frontière de crate.** Les alias sont des `--cfg` passés à `frus-shell` seul. Le corps
  de la macro `main!` s'expanse dans le crate de l'**application**, où ils n'existent pas :
  un `#[cfg(desktop)]` y serait *toujours faux* et l'app n'aurait **aucun point d'entrée**.
  La macro garde donc ses prédicats `target_os` / `target_arch` explicites.
- **Cargo.** Les tables `[target.'cfg(…)'.dependencies]` sont évaluées par Cargo, qui ignore
  ces alias. La table des dépendances bureau garde la liste littérale — avec `target_os =
  "ios"` ajouté à la main, c'est elle qui empêche `arboard` et `accesskit_winit` d'atterrir
  sur iOS.

### La correction de fond : `not(desktop)`, pas `any(android, web)`

Le code portait des paires *implémentation / stub* de cette forme :

```rust
#[cfg(desktop)]            pub struct Clipboard(Option<arboard::Clipboard>);
#[cfg(any(android, web))]  pub struct Clipboard;                    // ← iOS : ni l'un ni l'autre
```

Sur iOS, **aucune** des deux branches ne s'applique : le type n'existe pas. Ces trois sites
passent à `#[cfg(not(desktop))]`, qui est la vraie intention (« tout ce qui n'est pas
bureau ») et qui reste correct pour toute plateforme future.

### Point d'entrée iOS

`run()` a maintenant deux corps : `#[cfg(desktop)]` (inchangé) et `#[cfg(ios)]`, ce dernier
sans `env_logger` (stderr n'est pas lisible sur appareil), sans presse-papier et sans
AccessKit. **La macro `main!` n'a pas eu à changer** : son prédicat `not(any(android,
wasm32))` couvre déjà iOS, et winit assure lui-même l'`UIApplicationMain`.

## Vérification

Le refactor est **sans effet** sur les trois plateformes en service — c'est sa propriété de
sûreté, et elle est vérifiable ici :

- `cargo build --workspace --all-targets` — OK ;
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK ;
- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **613 tests, 0 échec**.

**iOS lui-même n'est pas vérifiable depuis la machine de développement** (Windows : ni SDK
Apple ni linker, donc pas de compilation croisée crédible). C'est le nouveau job CI `ios`
(`macos-latest`) qui a tranché — il est là précisément pour dire la vérité sur ce que l'on
a écrit à l'aveugle. **Verdict : les deux cibles compilent.**

```
aarch64-apple-ios      →  Finished `dev` profile in 1m 33s
aarch64-apple-ios-sim  →  Finished `dev` profile in 16.17s
```

Le log confirme le dispositif au-delà du simple « ça compile » : `objc2-ui-kit` et `metal`
sont dans l'arbre (backend UIKit de winit, sortie Metal de wgpu), et **ni `arboard`, ni
`accesskit_winit`, ni `env_logger` n'y sont** — l'exclusion du `Cargo.toml` a tenu, et le
`run()` sous `#[cfg(ios)]` type-checke.

Le job est donc passé **bloquant** : ce qui vient d'être acquis ne doit pas régresser
silencieusement. Il prouve qu'iOS *compile*, pas qu'iOS *tourne*.

## Reste

Tout le shell iOS, en fait. Ce jalon n'a fait que rendre son ajout possible sans casser le
reste :

- cycle de vie UIKit (`applicationDidEnterBackground`…) et `Lifecycle` correspondant ;
- **safe-area insets** (encoche, indicateur d'accueil) → `WindowInsets`, comme Android ;
- IME et clavier logiciel ;
- logs vers `os_log` plutôt que stderr ;
- accessibilité UIKit ;
- empaquetage `.ipa` — le seul endroit où `cargo` n'a pas de réponse toute faite.
