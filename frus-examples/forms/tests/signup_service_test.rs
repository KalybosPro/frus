//! Tests for `forms::features::signup::controllers::signup_service`, moved out of `src/` (milestone 664).

use forms::features::signup::controllers::signup_service::*;
use forms::models::profile::Profile;

#[test]
fn the_server_accepts_a_new_address_and_refuses_a_taken_one() {
    let profile = Profile::sample();
    assert_eq!(answer(profile.clone()), Ok(profile.clone()));

    let taken = Profile {
        email: "Taken@Example.com".into(),
        ..profile
    };
    assert_eq!(
        answer(taken),
        Err(SignupError::EmailTaken("Taken@Example.com".into()))
    );
}
