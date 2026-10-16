//! Remembering who is signed in across restarts.
//!
//! frus has no storage API of its own: an application keeps its data the way any Rust
//! program does. On the desktop that is a small file in the user's configuration folder.
//! What is written is the session's id, never the password; at start-up the server is
//! asked whether that id still names an account (see `Auth::new`).
//!
//! On Android and the Web this example keeps the session in memory only — the README
//! shows what to add for each (the app's files directory, the browser's `localStorage`).

use std::path::PathBuf;

/// Where the session is kept, if anywhere.
#[derive(Clone, Debug, Default)]
pub(crate) struct Session {
    path: Option<PathBuf>,
}

impl Session {
    /// A session kept nowhere: forgotten when the application stops. What the tests use.
    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        Self { path: None }
    }

    /// A session kept in a file at `path`.
    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// The platform's place for it: a file in the configuration folder on a desktop.
    pub(crate) fn on_device() -> Self {
        Self {
            path: config_dir().map(|dir| dir.join("frus-authentication-example").join("session")),
        }
    }

    /// The saved session id, if there is one.
    pub(crate) fn load(&self) -> Option<String> {
        let path = self.path.as_ref()?;
        let id = std::fs::read_to_string(path).ok()?;
        let id = id.trim();
        (!id.is_empty()).then(|| id.to_string())
    }

    /// Saves the session id. A failure is logged and otherwise ignored: the person is
    /// signed in either way, they will just have to sign in again next time.
    pub(crate) fn save(&self, id: &str) {
        let Some(path) = &self.path else { return };
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(path, id));
        if let Err(error) = written {
            frus::log::warn!("could not save the session to {}: {error}", path.display());
        }
    }

    /// Forgets the saved session.
    pub(crate) fn clear(&self) {
        if let Some(path) = &self.path {
            // A missing file is already what we want.
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The user's configuration folder on a desktop; `None` where there is none to write to.
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn config_dir() -> Option<PathBuf> {
    let var = |name| std::env::var_os(name).map(PathBuf::from);
    if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|home| home.join(".config")))
    }
}

/// Android and the Web: no file here (see the module's documentation).
#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn config_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_session_is_read_back_then_forgotten() {
        let path = std::env::temp_dir()
            .join("frus-authentication-test")
            .join("session-roundtrip");
        let session = Session::at(path);
        session.save("u1");
        assert_eq!(session.load(), Some("u1".to_string()));
        session.clear();
        assert_eq!(session.load(), None);
    }

    #[test]
    fn an_in_memory_session_keeps_nothing() {
        let session = Session::in_memory();
        session.save("u1");
        assert_eq!(session.load(), None);
    }
}
