//! Tests for `widgets::features::navigation::widgets::steps`, moved out of `src/` (milestone 664).

use widgets::features::navigation::widgets::steps::*;

#[test]
fn next_waits_for_the_terms() {
    let mut wizard = WizardState::default();
    wizard.next();
    assert_eq!(wizard.current, 0);
    wizard.terms_accepted = true;
    wizard.next();
    assert_eq!(wizard.current, 1);
}

#[test]
fn markers_only_jump_backwards() {
    let mut wizard = WizardState {
        current: 1,
        terms_accepted: true,
        ..WizardState::default()
    };
    wizard.jump(2);
    assert_eq!(wizard.current, 1);
    wizard.jump(0);
    assert_eq!(wizard.current, 0);
}

#[test]
fn finishing_the_last_step_marks_it_done() {
    let mut wizard = WizardState {
        current: 2,
        terms_accepted: true,
        ..WizardState::default()
    };
    wizard.next();
    assert!(wizard.finished && wizard.is_done(2));
}
