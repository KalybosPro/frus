//! Tests for `widgets::features::display::widgets::table`, moved out of `src/` (milestone 662).

use widgets::features::display::widgets::table::sorted_planets;

#[test]
fn sorts_by_the_chosen_column() {
    let by_moons = sorted_planets(1, false);
    assert_eq!(by_moons[0].0, "Jupiter");
    let by_name = sorted_planets(0, true);
    assert_eq!(by_name[0].0, "Earth");
}
