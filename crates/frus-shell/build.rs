//! Alias de `cfg` pour les plateformes de frus.
//!
//! `frus-shell` est la seule couche dépendante de la plateforme, et jusqu'ici
//! « bureau » s'y écrivait **par la négative** :
//! `not(any(target_os = "android", target_arch = "wasm32"))`. Cette formulation
//! a un défaut fatal dès qu'on ajoute une 4e plateforme : **iOS y tomberait
//! silencieusement**, héritant du presse-papier `arboard`, d'`env_logger` et
//! d'AccessKit — trois choses qui n'ont pas de sens (ni de backend) sur iOS.
//! Le code compilerait peut-être, et il serait faux.
//!
//! On nomme donc les plateformes explicitement, une bonne fois. Ajouter une
//! cible ne touche plus que ce fichier.
//!
//! Deux limites à connaître :
//!
//! 1. Ces alias sont des `--cfg` passés à **ce crate seulement**. Ils ne sont
//!    pas visibles dans le crate de l'application — d'où les prédicats
//!    `target_os` / `target_arch` **explicites**, conservés tels quels, dans le
//!    corps de la macro `frus_shell::main!` (qui, elle, s'expanse chez
//!    l'utilisateur).
//! 2. Cargo n'évalue pas ces alias dans les tables
//!    `[target.'cfg(…)'.dependencies]` du `Cargo.toml` : la sélection des
//!    dépendances y reste écrite en `target_os` / `target_arch`.

fn main() {
    cfg_aliases::cfg_aliases! {
        // Les trois plateformes en service.
        web: { target_arch = "wasm32" },
        android: { target_os = "android" },
        // La cible en cours d'amorçage (jalon 276 et suivants).
        ios: { target_os = "ios" },
        // « Bureau » = Windows / macOS / Linux : winit avec fenêtre, presse-papier
        // système, AccessKit et logs sur stderr. Toujours défini en excluant les
        // autres, mais **iOS en est désormais exclu** — c'est tout l'objet du jalon.
        //
        // Écrit en réutilisant les alias ci-dessus : `cfg_aliases` sature sa limite
        // de récursion si on lui donne la liste `target_os`/`target_arch` en entier.
        desktop: { not(any(web, android, ios)) },
    }
}
