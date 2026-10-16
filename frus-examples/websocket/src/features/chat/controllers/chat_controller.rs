//! The glue between the page's state and its [`Connection`]: opening one, routing its
//! events into the state, and scheduling the retry after a failure.
//!
//! It works through a [`StateHandle`], the way to reach a `State` from outside `build`:
//! from an event that arrives later, or from a timer.

use std::time::Duration;

use frus::{host, StateHandle};

use crate::features::chat::controllers::connection::{Connection, Event};
use crate::features::chat::states::chatstate::ChatState;
use crate::models::message::TimeOfDay;

/// Opens a connection to `state.url`, replacing any previous one.
///
/// Takes the state by `&mut` because it is called from places that already hold it: a
/// handler made with `cx.callback`, and `init_state`.
pub(crate) fn connect(state: &mut ChatState, handle: &StateHandle<ChatState>) {
    let session = state.begin_connect();
    let handle = handle.clone();
    let connection = Connection::open(&state.url, move |event| deliver(&handle, session, event));
    state.attach(connection);
}

/// One event, on the interface's thread: into the state, then a retry if it asks for one.
fn deliver(handle: &StateHandle<ChatState>, session: u64, event: Event) {
    let mut retry_in = None;
    // `set_state` changes the state and asks for a rebuild — and does nothing once the
    // page has left the tree, so a late event is harmless.
    handle.set_state(|state| retry_in = state.apply(session, event, TimeOfDay::now()));
    if let Some(delay) = retry_in {
        schedule_retry(handle.clone(), session, delay);
    }
}

/// Tries again after `delay`, unless the reason for it has gone by then.
///
/// `host::after` runs its function on the interface's thread once `delay` has passed. It
/// cannot be cancelled, so the function checks when it runs: the page must still be there
/// ([`StateHandle::is_mounted`]) and still want this very retry (`should_retry`).
fn schedule_retry(handle: StateHandle<ChatState>, session: u64, delay: Duration) {
    host::after(delay, move || {
        if handle.is_mounted() && handle.read(|state| state.should_retry(session)) {
            let again = handle.clone();
            handle.set_state(|state| connect(state, &again));
        }
    });
}
