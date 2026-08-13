//! Loading and saving the task list. The only module that touches the disk.

use crate::prelude::*;

/// Path of the file the tasks are persisted to.
pub(crate) fn todos_path() -> PathBuf {
    std::env::temp_dir().join("frus-todos.txt")
}

/// Serialises the tasks as `done<TAB>text` lines.
pub(crate) fn save_todos(path: &Path, todos: &[(bool, String)]) -> std::io::Result<()> {
    let mut out = String::new();
    for (done, text) in todos {
        out.push(if *done { '1' } else { '0' });
        out.push('\t');
        // Neutralises the separators inside the text.
        out.push_str(&text.replace(['\t', '\n'], " "));
        out.push('\n');
    }
    std::fs::write(path, out)
}

/// Reads the tasks from the file (empty when it is missing/unreadable).
pub(crate) fn load_todos(path: &Path) -> Vec<(bool, String)> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| {
            let (flag, text) = line.split_once('\t')?;
            Some((flag == "1", text.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test of this module alone, so it lives in this module. Only tests that cut
    /// across several — a whole screen, an interaction from message to scene — go to
    /// `src/tests.rs`.
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
}
