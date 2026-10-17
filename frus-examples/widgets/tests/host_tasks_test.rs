//! Tests for `widgets::features::state::widgets::host_tasks`, moved out of `src/` (milestone 664).

use widgets::features::state::widgets::host_tasks::fibonacci;

#[test]
fn small_fibonacci_numbers() {
    assert_eq!(fibonacci(0), 0);
    assert_eq!(fibonacci(1), 1);
    assert_eq!(fibonacci(10), 55);
}
