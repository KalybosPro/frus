//! Tests for `widgets::features::state::widgets::hooks`, moved out of `src/` (milestone 664).

use widgets::features::state::widgets::hooks::count_primes;

#[test]
fn counts_the_primes_below_a_bound() {
    assert_eq!(count_primes(0), 0);
    assert_eq!(count_primes(3), 1);
    assert_eq!(count_primes(10), 4);
    assert_eq!(count_primes(100), 25);
}
