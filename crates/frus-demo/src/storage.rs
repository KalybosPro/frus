//! Loading and saving the task list. The only module that touches the disk — or, in a
//! browser, which has no disk, the page's `localStorage`.

use crate::prelude::*;

/// Where the tasks are persisted: a file in the temporary directory, or the name of the
/// `localStorage` entry in a browser.
pub(crate) fn todos_path() -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::temp_dir().join("frus-todos.txt")
    }
    // `temp_dir` panics where there is no filesystem, so the name is all it is.
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::from("frus-todos")
    }
}

/// Serialises the tasks as `done<TAB>text` lines.
fn encode(todos: &[(bool, String)]) -> String {
    let mut out = String::new();
    for (done, text) in todos {
        out.push(if *done { '1' } else { '0' });
        out.push('\t');
        // Neutralises the separators inside the text.
        out.push_str(&text.replace(['\t', '\n'], " "));
        out.push('\n');
    }
    out
}

/// The tasks in `content`; a line that is not one is skipped.
fn decode(content: &str) -> Vec<(bool, String)> {
    content
        .lines()
        .filter_map(|line| {
            let (flag, text) = line.split_once('\t')?;
            Some((flag == "1", text.to_string()))
        })
        .collect()
}

/// Writes the tasks to `path`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_todos(path: &Path, todos: &[(bool, String)]) -> std::io::Result<()> {
    std::fs::write(path, encode(todos))
}

/// Reads the tasks from the file (empty when it is missing/unreadable).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_todos(path: &Path) -> Vec<(bool, String)> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    decode(&content)
}

/// The page's `localStorage`, if the browser gives it out.
#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Writes the tasks to the `localStorage` entry `path` names.
#[cfg(target_arch = "wasm32")]
pub(crate) fn save_todos(path: &Path, todos: &[(bool, String)]) -> std::io::Result<()> {
    let unavailable = || std::io::Error::other("localStorage is not available");
    local_storage()
        .ok_or_else(unavailable)?
        .set_item(&path.to_string_lossy(), &encode(todos))
        .map_err(|_| unavailable())
}

/// Reads the tasks from the `localStorage` entry `path` names (empty when there is none).
#[cfg(target_arch = "wasm32")]
pub(crate) fn load_todos(path: &Path) -> Vec<(bool, String)> {
    local_storage()
        .and_then(|storage| storage.get_item(&path.to_string_lossy()).ok().flatten())
        .map(|content| decode(&content))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test of this module alone, so it lives in this module. Only tests that cut
    /// across several — a whole screen, an interaction from message to scene — go to
    /// `src/tests.rs`.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn save_then_load_roundtrips() {
        let path = std::env::temp_dir().join("frus-todos-test-roundtrip.txt");
        let items = vec![
            (false, "buy bread".to_string()),
            (true, "tidy the desk".to_string()),
        ];
        save_todos(&path, &items).unwrap();
        assert_eq!(load_todos(&path), items);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn what_is_encoded_decodes_to_the_same_tasks() {
        let items = vec![
            (false, "buy bread".to_string()),
            (true, "tidy\tthe\ndesk".to_string()),
        ];
        assert_eq!(
            decode(&encode(&items)),
            [
                (false, "buy bread".to_string()),
                (true, "tidy the desk".to_string())
            ],
            "a tab or a newline in a task is a space once it has been through"
        );
    }
}
