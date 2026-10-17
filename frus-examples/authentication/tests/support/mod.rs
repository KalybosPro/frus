//! What the tests share: the shell's part played at once, and a signed-out service.

use frus::host::{self, Effect};

use authentication::common::auth::Auth;
use authentication::common::session::Session;
use authentication::features::auth::controllers::user_store::UserStore;

/// Plays the shell's part: runs the timers the service asked for, at once.
pub fn answer_now() {
    for effect in host::take_effects() {
        if let Effect::After(_, callback) = effect {
            callback.call();
        }
    }
}

pub fn signed_out() -> Auth {
    Auth::with(UserStore::seeded(), Session::in_memory())
}
