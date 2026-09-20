//! The demo's **data**: what a task is, the fixed sets the screens draw, and the rules that
//! read them.
//!
//! Nothing here holds state. What is kept lives where it is used — a screen's own in its
//! `State`, what more than one screen needs in [`Demo`](crate::demo::Demo) — and what is
//! *derived* from it lives here as plain functions, so it cannot drift into three slightly
//! different versions of the same count.

/// One task of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Todo {
    pub(crate) id: u64,
    pub(crate) text: String,
    pub(crate) done: bool,
}

/// Display filter of the task list.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub(crate) enum Filter {
    #[default]
    All,
    Active,
    Done,
}

impl Filter {
    /// This filter's index (for the segmented control).
    pub(crate) fn index(self) -> usize {
        match self {
            Filter::All => 0,
            Filter::Active => 1,
            Filter::Done => 2,
        }
    }

    /// The filter matching a segment index.
    pub(crate) fn from_index(index: usize) -> Filter {
        match index {
            1 => Filter::Active,
            2 => Filter::Done,
            _ => Filter::All,
        }
    }

    /// Does this filter show `todo`?
    pub(crate) fn shows(self, todo: &Todo) -> bool {
        match self {
            Filter::All => true,
            Filter::Active => !todo.done,
            Filter::Done => todo.done,
        }
    }
}

/// The tasks a list is **showing**, in the order it shows them: the filter, in one place,
/// because a screen that decides which rows to draw and a move that decides which row was
/// carried have to agree on the answer.
pub(crate) fn visible(todos: &[Todo], filter: Filter) -> Vec<&Todo> {
    todos.iter().filter(|t| filter.shows(t)).collect()
}

/// Number of tasks that are not done.
pub(crate) fn active_count(todos: &[Todo]) -> usize {
    todos.iter().filter(|t| !t.done).count()
}

/// Number of tasks that are done.
pub(crate) fn done_count(todos: &[Todo]) -> usize {
    todos.iter().filter(|t| t.done).count()
}

// --- The demo's fixed data, and the validation that reads it ---

/// The data table's static dataset (name, role, score) — milestone 237.
pub(crate) const DATA_PEOPLE: [(&str, &str, u32, &str); 12] = [
    ("Ada Lovelace", "Engineer", 92, "High"),
    ("Alan Turing", "Cryptographer", 88, "Medium"),
    ("Grace Hopper", "Admiral", 95, "High"),
    ("Katherine Johnson", "Mathematician", 90, "Medium"),
    ("Edsger Dijkstra", "Researcher", 84, "Low"),
    ("Barbara Liskov", "Professor", 91, "High"),
    ("Donald Knuth", "Author", 87, "Low"),
    ("Margaret Hamilton", "Director", 93, "High"),
    ("Tim Berners-Lee", "Inventor", 89, "Medium"),
    ("Linus Torvalds", "Maintainer", 86, "Low"),
    ("Radia Perlman", "Engineer", 90, "Medium"),
    ("Vint Cerf", "Architect", 85, "Low"),
];

/// Titles of the Kanban columns (a demo, milestone 247).
pub(crate) const KANBAN_TITLES: [&str; 3] = ["To do", "Doing", "Done"];
/// The Kanban's starting cards, per column (a demo, milestone 247).
pub(crate) const KANBAN_SEED: [&[&str]; 3] = [
    &["Design API", "Write spec", "Triage bugs"],
    &["Build widget"],
    &["Kickoff", "Research"],
];
/// The labels of the board's strip (a demo, milestone 527): enough of them to run well past a
/// phone's width, so the strip scrolls and a label carried to either edge scrolls it.
pub(crate) const BOARD_LABELS: [&str; 10] = [
    "Bug", "Feature", "Design", "Docs", "Urgent", "Later", "Ideas", "Ops", "Tests", "Release",
];

/// Categories (the x axis) of the chart dashboard.
pub(crate) const CHART_CATS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
/// The dashboard's series: `(name, values)`. Series 0 = the theme's accent; 1.. = `CHART_COLORS`.
pub(crate) const CHART_SERIES: [(&str, [f32; 5]); 3] = [
    ("Sales", [3.0, 7.0, 5.0, 8.0, 4.0]),
    ("Costs", [2.0, 4.0, 3.0, 5.0, 2.0]),
    ("Profit", [1.0, 3.0, 2.0, 3.0, 2.0]),
];

/// **Pure** validation of a grid cell: `Name` (col 0) is required, `Email` (col 2) must look
/// like an address. `None` = valid. It demonstrates `TextField::error` per cell.
pub(crate) fn grid_cell_error(col: usize, value: &str) -> Option<&'static str> {
    match col {
        0 if value.trim().is_empty() => Some("Required"),
        2 if !(value.is_empty() || value.contains('@') && value.contains('.')) => {
            Some("Invalid email")
        }
        _ => None,
    }
}

/// Total number of invalid cells in the grid (milestone 204/207) — it gates the submission.
pub(crate) fn grid_error_count(grid: &[Vec<String>]) -> usize {
    grid.iter()
        .flat_map(|row| {
            (0..3).filter(move |&c| {
                grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or("")).is_some()
            })
        })
        .count()
}

/// Every invalid cell `(row, column)`, in row-by-row order.
pub(crate) fn grid_faults(grid: &[Vec<String>]) -> Vec<(usize, usize)> {
    grid.iter()
        .enumerate()
        .flat_map(|(r, row)| {
            (0..3).filter_map(move |c| {
                grid_cell_error(c, row.get(c).map(String::as_str).unwrap_or(""))
                    .is_some()
                    .then_some((r, c))
            })
        })
        .collect()
}

/// The faulty cell **after** `after` (row-by-row order, wrapping at the end) — so every fault
/// can be cycled through (milestone 214). `after = None` returns the first one.
pub(crate) fn grid_next_error(
    grid: &[Vec<String>],
    after: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let faults = grid_faults(grid);
    match after {
        None => faults.first().copied(),
        Some(cur) => faults
            .iter()
            .copied()
            .find(|&f| f > cur)
            .or_else(|| faults.first().copied()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo(id: u64, done: bool) -> Todo {
        Todo {
            id,
            text: format!("task {id}"),
            done,
        }
    }

    #[test]
    fn a_filter_shows_what_it_says() {
        let todos = vec![todo(1, false), todo(2, true), todo(3, false)];
        let ids = |f| visible(&todos, f).iter().map(|t| t.id).collect::<Vec<_>>();
        assert_eq!(ids(Filter::All), [1, 2, 3]);
        assert_eq!(ids(Filter::Active), [1, 3]);
        assert_eq!(ids(Filter::Done), [2]);
        assert_eq!((active_count(&todos), done_count(&todos)), (2, 1));
    }

    #[test]
    fn a_filter_round_trips_through_its_segment_index() {
        for f in [Filter::All, Filter::Active, Filter::Done] {
            assert_eq!(Filter::from_index(f.index()), f);
        }
        assert_eq!(Filter::from_index(9), Filter::All, "nonsense is everything");
    }

    #[test]
    fn grid_next_error_cycles_through_all_faults() {
        let grid = vec![
            vec!["".to_string(), "Engineer".into(), "a@b.c".into()],
            vec!["Alan".into(), "x".into(), "nope".into()],
        ];
        assert_eq!(grid_faults(&grid), [(0, 0), (1, 2)]);
        assert_eq!(grid_error_count(&grid), 2);
        assert_eq!(grid_next_error(&grid, None), Some((0, 0)));
        assert_eq!(grid_next_error(&grid, Some((0, 0))), Some((1, 2)));
        assert_eq!(grid_next_error(&grid, Some((1, 2))), Some((0, 0)), "wraps");
        assert_eq!(grid_next_error(&[vec!["ok".to_string()]], None), None);
    }
}
