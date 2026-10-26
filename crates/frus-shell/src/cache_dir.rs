//! Where an application keeps what it can lose: the renderer's compiled pipelines (milestones
//! 685 and 697, #217). Plain functions of the environment, so every platform's answer is tested
//! on any of them.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// The folder kept for the pipelines of `program` on a desktop `os` (`std::env::consts::OS`),
/// from the environment's variables `var`:
///
/// - Linux and the other Unixes: `$XDG_CACHE_HOME/frus/<program>`, or `~/.cache/frus/<program>`;
/// - macOS: `~/Library/Caches/frus/<program>`;
/// - Windows: `%LOCALAPPDATA%\frus\<program>`.
///
/// `None` when the variable it needs is unset or not a full path: then nothing is kept.
#[cfg_attr(not(desktop), allow(dead_code))]
pub(crate) fn desktop(
    os: &str,
    var: impl Fn(&str) -> Option<OsString>,
    program: &OsStr,
) -> Option<PathBuf> {
    let full = |name: &str| {
        var(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() || looks_absolute(path))
    };
    let base = match os {
        "windows" => full("LOCALAPPDATA")?,
        "macos" => full("HOME")?.join("Library").join("Caches"),
        _ => full("XDG_CACHE_HOME").or_else(|| full("HOME").map(|home| home.join(".cache")))?,
    };
    (!program.is_empty()).then(|| base.join("frus").join(program))
}

/// Whether `path` is a full path on some platform: `C:\…` is one on Windows even when the tests
/// run elsewhere, and `/…` is one on Unix even when they run on Windows.
fn looks_absolute(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    text.starts_with('/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}

/// The folder kept for the pipelines on Android, from the application's `files` folder
/// (`AndroidApp::internal_data_path`): its `cache` sibling — what `Context.getCacheDir` names —,
/// which the system may empty when it runs short and the user empties with "Clear cache". A
/// `files` folder laid out otherwise keeps them inside it.
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) fn android(files: &Path) -> PathBuf {
    match files.parent() {
        Some(data) if files.file_name() == Some(OsStr::new("files")) => {
            data.join("cache").join("frus-pipelines")
        }
        _ => files.join("frus-pipelines"),
    }
}

/// Where milestone 685 kept them on Android, before they moved to the cache folder: removed at
/// the next start, since nothing reads it any more.
#[cfg_attr(not(android), allow(dead_code))]
pub(crate) fn android_before_697(files: &Path) -> PathBuf {
    files.join("frus-pipelines")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| OsString::from(v))
        }
    }

    const APP: &str = "counter";

    #[test]
    fn linux_uses_xdg_cache_home_then_home() {
        let both = env(&[("XDG_CACHE_HOME", "/xdg"), ("HOME", "/home/me")]);
        assert_eq!(
            desktop("linux", both, OsStr::new(APP)),
            Some(PathBuf::from("/xdg").join("frus").join(APP))
        );
        let home = env(&[("HOME", "/home/me")]);
        assert_eq!(
            desktop("linux", home, OsStr::new(APP)),
            Some(
                PathBuf::from("/home/me")
                    .join(".cache")
                    .join("frus")
                    .join(APP)
            )
        );
        // A relative XDG_CACHE_HOME is to be ignored (the XDG rules).
        let relative = env(&[("XDG_CACHE_HOME", "cache"), ("HOME", "/home/me")]);
        assert_eq!(
            desktop("freebsd", relative, OsStr::new(APP)),
            Some(
                PathBuf::from("/home/me")
                    .join(".cache")
                    .join("frus")
                    .join(APP)
            )
        );
    }

    #[test]
    fn macos_uses_library_caches() {
        let home = env(&[("HOME", "/Users/me"), ("XDG_CACHE_HOME", "/xdg")]);
        assert_eq!(
            desktop("macos", home, OsStr::new(APP)),
            Some(
                PathBuf::from("/Users/me")
                    .join("Library")
                    .join("Caches")
                    .join("frus")
                    .join(APP)
            )
        );
    }

    #[test]
    fn windows_uses_local_app_data() {
        let local = env(&[
            ("LOCALAPPDATA", r"C:\Users\me\AppData\Local"),
            ("HOME", "/home/me"),
        ]);
        assert_eq!(
            desktop("windows", local, OsStr::new(APP)),
            Some(
                PathBuf::from(r"C:\Users\me\AppData\Local")
                    .join("frus")
                    .join(APP)
            )
        );
    }

    /// Without the variable the platform needs, or without a program's name, nothing is kept —
    /// never a folder relative to wherever the program was started.
    #[test]
    fn nothing_is_kept_without_a_full_path() {
        assert_eq!(desktop("linux", env(&[]), OsStr::new(APP)), None);
        assert_eq!(desktop("macos", env(&[]), OsStr::new(APP)), None);
        assert_eq!(
            desktop("windows", env(&[("HOME", "/h")]), OsStr::new(APP)),
            None
        );
        assert_eq!(
            desktop("windows", env(&[("LOCALAPPDATA", "")]), OsStr::new(APP)),
            None
        );
        assert_eq!(
            desktop("linux", env(&[("HOME", "home")]), OsStr::new(APP)),
            None
        );
        assert_eq!(
            desktop("linux", env(&[("HOME", "/home/me")]), OsStr::new("")),
            None
        );
    }

    /// Android: the `cache` folder beside `files`, and the folder used before inside `files`.
    #[test]
    fn android_keeps_them_in_the_cache_folder() {
        let files = Path::new("/data/user/0/com.example.counter/files");
        assert_eq!(
            android(files),
            Path::new("/data/user/0/com.example.counter")
                .join("cache")
                .join("frus-pipelines")
        );
        assert_ne!(android(files), android_before_697(files));
        let odd = Path::new("/somewhere/else");
        assert_eq!(android(odd), odd.join("frus-pipelines"));
    }
}
