//! Tests for `widgets::features::state::widgets::stream_builder`, moved out of `src/` (milestone 662).

use widgets::features::state::widgets::stream_builder::is_prime;

#[test]
fn knows_small_primes() {
    let primes: Vec<u64> = (0..20).filter(|&n| is_prime(n)).collect();
    assert_eq!(primes, [2, 3, 5, 7, 11, 13, 17, 19]);
}
