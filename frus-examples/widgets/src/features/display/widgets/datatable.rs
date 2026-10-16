//! `DataTable` — a table of text rows that sorts, filters, paginates and selects for
//! you, while the state it shows (sort, page, query, selection) stays in your app.

use frus::{
    Align, Button, Column, DataTable, State, StateContext, StatefulWidget, Text, Variant, Widget,
};

const PEOPLE: [(&str, &str, u32); 12] = [
    ("Ada", "Research", 92),
    ("Alan", "Research", 88),
    ("Barbara", "Platform", 75),
    ("Dennis", "Platform", 81),
    ("Edsger", "Research", 67),
    ("Frances", "Compilers", 95),
    ("Grace", "Compilers", 99),
    ("Ken", "Platform", 72),
    ("Linus", "Kernel", 84),
    ("Margaret", "Flight", 97),
    ("Niklaus", "Compilers", 70),
    ("Radia", "Networks", 90),
];

const PER_PAGE: usize = 5;

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(PeopleTable.into_widget())
}

/// The demo is a small `StatefulWidget`: the table's whole state lives in one struct, and
/// each handler changes it through a method — the same shape as a real page.
struct PeopleTable;

#[derive(Default)]
struct PeopleTableState {
    /// The sorted column and its direction; `None` keeps the source order.
    sort: Option<(usize, bool)>,
    /// 1-based page.
    page: usize,
    query: String,
    /// Checked rows, as indices into `PEOPLE` (the table translates through sorting).
    checked: Vec<usize>,
}

impl PeopleTableState {
    /// A click on a header: a new column sorts ascending, the same one flips.
    fn sort_by(&mut self, column: usize) {
        let ascending = match self.sort {
            Some((current, ascending)) if current == column => !ascending,
            _ => true,
        };
        self.sort = Some((column, ascending));
        self.page = 1;
    }

    fn search(&mut self, query: String) {
        self.query = query;
        self.page = 1;
    }

    fn toggle(&mut self, row: usize) {
        if let Some(at) = self.checked.iter().position(|&r| r == row) {
            self.checked.remove(at);
        } else {
            self.checked.push(row);
        }
    }

    /// The header box: everything, or nothing if everything was already checked.
    fn toggle_all(&mut self) {
        if self.checked.len() == PEOPLE.len() {
            self.checked.clear();
        } else {
            self.checked = (0..PEOPLE.len()).collect();
        }
    }
}

impl StatefulWidget for PeopleTable {
    type State = PeopleTableState;

    fn create_state(&self) -> PeopleTableState {
        PeopleTableState {
            page: 1,
            ..PeopleTableState::default()
        }
    }
}

impl State for PeopleTableState {
    type Widget = PeopleTable;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let rows = PEOPLE
            .iter()
            .map(|(name, team, score)| vec![name.to_string(), team.to_string(), score.to_string()]);

        let mut table = DataTable::new(["Name", "Team", "Score"], rows)
            .column_widths(&[110.0, 110.0, 70.0])
            // Searching filters every column, before sorting and paging.
            .searchable(
                self.query.as_str(),
                cx.handler(|s: &mut Self, q: String| s.search(q)),
            )
            .empty_text("Nobody matches that search")
            // A header click reports the column; the app decides the direction.
            .on_sort(cx.handler(|s: &mut Self, column: usize| s.sort_by(column)))
            // Checkboxes for multi-selection, with a "check all" box in the header.
            .checkboxes(
                cx.handler(|s: &mut Self, row: usize| s.toggle(row)),
                cx.callback(Self::toggle_all),
            )
            .selected(&self.checked)
            // Shown above the table only while rows are checked.
            .bulk_actions({
                let clear = cx.callback(|s: &mut Self| s.checked.clear());
                move || {
                    vec![Box::new(
                        Button::new("Clear")
                            .variant(Variant::Text)
                            .on_press(clear.clone()),
                    ) as Box<dyn Widget>]
                }
            })
            .paginated(
                self.page,
                PER_PAGE,
                cx.handler(|s: &mut Self, page: usize| s.page = page),
            );
        if let Some((column, ascending)) = self.sort {
            table = table.sorted(column, ascending);
        }

        Box::new(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(table)
                .child(
                    Text::new(format!("{} checked", self.checked.len())).color(cx.theme().muted),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorting_the_same_column_flips_direction() {
        let mut state = PeopleTableState::default();
        state.sort_by(2);
        assert_eq!(state.sort, Some((2, true)));
        state.sort_by(2);
        assert_eq!(state.sort, Some((2, false)));
        state.sort_by(0);
        assert_eq!(state.sort, Some((0, true)));
    }

    #[test]
    fn check_all_toggles_everything() {
        let mut state = PeopleTableState::default();
        state.toggle(3);
        state.toggle_all();
        assert_eq!(state.checked.len(), PEOPLE.len());
        state.toggle_all();
        assert!(state.checked.is_empty());
    }
}
