//! Tests for `widgets::features::feedback::widgets::progress_indicator`, moved out of `src/` (milestone 662).

use frus::StatefulWidget;
use widgets::features::feedback::widgets::progress_indicator::*;

#[test]
fn the_download_stops_at_the_end() {
    let mut state = ProgressDemo.create_state();
    state.start();
    for _ in 0..100 {
        state.tick();
    }
    assert_eq!(state.progress, 1.0);
    assert!(!state.running);
    // Starting again from the end starts from zero.
    state.start();
    assert_eq!(state.progress, 0.0);
}
