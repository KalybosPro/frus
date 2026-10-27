//! **Which thread the interface runs on**, for the calls that must never run there: the
//! system's file dialogs and notifications wait for the person, and would stop every frame
//! while they waited.

use std::sync::OnceLock;
use std::thread::ThreadId;

/// The thread the event loop runs on.
static INTERFACE: OnceLock<ThreadId> = OnceLock::new();

/// The window exists: the caller's thread is the interface's.
pub(crate) fn window_made() {
    INTERFACE.get_or_init(|| std::thread::current().id());
}

/// Whether the caller runs on the interface's thread.
pub(crate) fn current() -> bool {
    INTERFACE.get() == Some(&std::thread::current().id())
}

/// Makes the test's own thread the interface's, unless another test already claimed it:
/// whether it did.
#[cfg(test)]
pub(crate) fn claim_for_test() -> bool {
    let me = std::thread::current().id();
    *INTERFACE.get_or_init(|| me) == me
}
