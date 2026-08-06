//! **Live-reload préservant l'état** (§13, dev uniquement).
//!
//! Le principe (celui du cahier d'idées) : l'état Elm est **une struct
//! unique** — il suffit de la sérialiser pour survivre à un remplacement de
//! binaire. Sous `FRUS_WATCH=1` (builds debug), le shell surveille le mtime de
//! son propre exécutable ; quand `cargo build` (ou `cargo watch`) le remplace,
//! il capture [`Application::save_state`], relance le **nouveau** binaire avec
//! le chemin de l'instantané en environnement, et s'efface. Le nouveau
//! processus réhydrate via [`Application::restore_state`] avant `init`.
//!
//! Pur `cargo` : aucune ABI à stabiliser, aucun outillage propriétaire —
//! `FRUS_WATCH=1 cargo run` dans un terminal, `cargo watch -x build` dans un
//! autre, et l'itération ressemble à Flutter (l'état survit à l'édition).
//!
//! [`Application::save_state`]: crate::Application::save_state
//! [`Application::restore_state`]: crate::Application::restore_state

// `Path` ne sert qu'à `restore_from_env` (bureau) ; `PathBuf` sert partout où le
// module compile (via `state_file_path`).
#[cfg(desktop)]
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use web_time::Instant;

/// Variable d'environnement qui **active** la surveillance (dev).
const WATCH_ENV: &str = "FRUS_WATCH";
/// Variable d'environnement portant le chemin de l'instantané à réhydrater.
const STATE_ENV: &str = "FRUS_HOT_STATE";
/// Cadence de vérification du mtime de l'exécutable.
const POLL_INTERVAL: Duration = Duration::from_millis(700);

/// Surveille le binaire courant et orchestre la relance.
pub(crate) struct ReloadWatcher {
    /// Chemin de l'exécutable **capturé au démarrage** : après remplacement,
    /// `/proc/self/exe` pointerait sur l'inode supprimé (« (deleted) ») — le
    /// chemin d'origine, lui, désigne le binaire fraîchement compilé.
    exe: PathBuf,
    mtime: SystemTime,
    next_poll: Instant,
}

impl ReloadWatcher {
    /// `Some` seulement en build debug, sous `FRUS_WATCH=1`, si l'exécutable
    /// est observable.
    pub(crate) fn new() -> Option<Self> {
        if !cfg!(debug_assertions) || std::env::var_os(WATCH_ENV).map_or(true, |v| v != "1") {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let mtime = std::fs::metadata(&exe).ok()?.modified().ok()?;
        Some(Self {
            exe,
            mtime,
            next_poll: Instant::now() + POLL_INTERVAL,
        })
    }

    /// Prochaine échéance de vérification (pour `ControlFlow::WaitUntil`).
    pub(crate) fn deadline(&self) -> Instant {
        self.next_poll
    }

    /// Vérifie (au plus une fois par cadence) si le binaire a été remplacé.
    pub(crate) fn binary_changed(&mut self) -> bool {
        let now = Instant::now();
        if now < self.next_poll {
            return false;
        }
        self.next_poll = now + POLL_INTERVAL;
        match std::fs::metadata(&self.exe).and_then(|m| m.modified()) {
            Ok(mtime) if mtime != self.mtime => {
                self.mtime = mtime;
                true
            }
            _ => false,
        }
    }

    /// Relance le binaire recompilé et s'efface : écrit l'instantané éventuel,
    /// le transmet par environnement, lance le nouveau processus puis termine
    /// celui-ci. Sans instantané, la relance repart d'un état neuf.
    pub(crate) fn handoff(&self, state: Option<Vec<u8>>) -> ! {
        let mut command = std::process::Command::new(&self.exe);
        if let Some(bytes) = state {
            let path = state_file_path();
            if std::fs::write(&path, bytes).is_ok() {
                command.env(STATE_ENV, &path);
            }
        }
        match command.spawn() {
            Ok(_) => eprintln!("[frus] binaire recompilé : relance ({})", self.exe.display()),
            Err(err) => eprintln!("[frus] relance impossible : {err}"),
        }
        std::process::exit(0);
    }
}

/// Chemin de l'instantané d'état (unique par processus parent).
fn state_file_path() -> PathBuf {
    std::env::temp_dir().join(format!("frus-hot-{}.state", std::process::id()))
}

/// À l'amorçage : réhydrate l'état depuis l'instantané laissé par le binaire
/// précédent, s'il existe (le fichier est consommé). À appeler avant `init`.
/// Bureau uniquement — le live-reload n'a pas de sens sur Android ni sur le Web,
/// où `run` (son unique appelant) n'existe pas.
#[cfg(desktop)]
pub(crate) fn restore_from_env<A: crate::Application>(app: &mut A) {
    let Some(path) = std::env::var_os(STATE_ENV) else {
        return;
    };
    let path = Path::new(&path);
    if let Ok(bytes) = std::fs::read(path) {
        app.restore_state(&bytes);
        eprintln!("[frus] état réhydraté ({} octets)", bytes.len());
    }
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::{Theme, Widget};

    struct Probe {
        restored: Option<Vec<u8>>,
    }

    impl crate::Application for Probe {
        type Message = ();
        fn update(&mut self, _: ()) -> crate::Command<()> {
            crate::Command::none()
        }
        fn view(&self, _: &Theme, _: f32, _: f32) -> Box<dyn Widget<()>> {
            unimplemented!("jamais rendu dans ce test")
        }
        fn restore_state(&mut self, bytes: &[u8]) {
            self.restored = Some(bytes.to_vec());
        }
    }

    /// L'instantané pointé par l'environnement est réhydraté puis consommé ;
    /// sans variable, rien ne se passe.
    #[test]
    fn restore_reads_and_consumes_the_snapshot() {
        let path = std::env::temp_dir().join("frus-hot-test.state");
        std::fs::write(&path, b"snapshot").unwrap();
        std::env::set_var(STATE_ENV, &path);
        let mut app = Probe { restored: None };
        restore_from_env(&mut app);
        std::env::remove_var(STATE_ENV);
        assert_eq!(app.restored.as_deref(), Some(&b"snapshot"[..]));
        assert!(!path.exists(), "l'instantané est consommé");

        let mut fresh = Probe { restored: None };
        restore_from_env(&mut fresh);
        assert!(fresh.restored.is_none(), "sans variable : démarrage neuf");
    }

    /// Le watcher n'existe que sous `FRUS_WATCH=1`, et ne signale un
    /// changement qu'après un vrai saut de mtime (cadence respectée).
    #[test]
    fn watcher_requires_opt_in_and_tracks_mtime() {
        std::env::remove_var(WATCH_ENV);
        assert!(ReloadWatcher::new().is_none(), "opt-in obligatoire");

        std::env::set_var(WATCH_ENV, "1");
        let watcher = ReloadWatcher::new();
        std::env::remove_var(WATCH_ENV);
        // En debug : le binaire de test est observable → Some.
        let Some(mut watcher) = watcher else {
            panic!("watcher attendu en debug sous FRUS_WATCH=1");
        };
        // La première vérification est retenue par la cadence.
        assert!(!watcher.binary_changed());
        // Une fois l'échéance passée, sans recompilation : toujours rien.
        watcher.next_poll = Instant::now() - Duration::from_millis(1);
        assert!(!watcher.binary_changed(), "mtime inchangé → pas de relance");
    }
}
