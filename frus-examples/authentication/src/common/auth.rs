//! The authentication service: who is signed in, and the requests that change it.
//!
//! One `Auth` is made when the application starts and handed to the router and to every
//! page that needs it. Cloning it is cheap and gives another handle on the **same**
//! service — the same signed-in user, the same server.
//!
//! Who is signed in lives in a `ValueNotifier`. Pages read it while they build; the router
//! *listens* to it (`refresh_listenable`), so signing in or out moves the person to the
//! right page without any page navigating by hand.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use frus::{host, ValueNotifier};

use crate::common::session::Session;
use crate::features::auth::controllers::user_store::{AuthError, UserStore};
use crate::models::user::User;

/// How long the pretend server takes to answer: long enough to see the buttons' loading
/// state.
const LATENCY: Duration = Duration::from_millis(900);

/// What a request reports back to the page that made it.
pub(crate) type Reply<T = ()> = Result<T, AuthError>;

#[derive(Clone)]
pub(crate) struct Auth {
    user: ValueNotifier<Option<User>>,
    /// The server. Shared by every handle, hence the `Rc<RefCell<..>>`.
    store: Rc<RefCell<UserStore>>,
    session: Session,
}

impl Auth {
    /// The service the application runs with: the seeded server, and the session saved on
    /// this device — if it still names an account, the person starts signed in.
    pub(crate) fn new() -> Self {
        Self::with(UserStore::seeded(), Session::on_device())
    }

    /// A service over `store`, remembering the session in `session`.
    pub(crate) fn with(store: UserStore, session: Session) -> Self {
        // A real application would check the saved token with its server, asynchronously,
        // showing a splash meanwhile. The pretend server is local, so it is asked directly.
        let restored = session.load().and_then(|id| store.user(&id));
        if restored.is_none() {
            session.clear();
        }
        Auth {
            user: ValueNotifier::new(restored),
            store: Rc::new(RefCell::new(store)),
            session,
        }
    }

    /// Who is signed in.
    pub(crate) fn user(&self) -> Option<User> {
        self.user.get()
    }

    pub(crate) fn is_signed_in(&self) -> bool {
        self.user.with(Option::is_some)
    }

    /// What the router listens to: a `ValueNotifier` is a `Listenable`.
    pub(crate) fn changes(&self) -> &ValueNotifier<Option<User>> {
        &self.user
    }

    /// Signs in. `done` is told how it went, a moment later, on the interface's thread.
    ///
    /// On success the user is set *after* `done` runs: the page that asked clears its
    /// loading state first, then the router — hearing the change — moves away from it.
    pub(crate) fn sign_in(&self, email: &str, password: &str, done: impl Fn(Reply) + 'static) {
        let (email, password) = (email.to_string(), password.to_string());
        let auth = self.clone();
        Self::later(move || {
            let reply = auth.store.borrow().sign_in(&email, &password);
            done(reply.clone().map(|_| ()));
            if let Ok(user) = reply {
                auth.start_session(user);
            }
        });
    }

    /// Creates an account and signs into it.
    pub(crate) fn register(
        &self,
        name: &str,
        email: &str,
        password: &str,
        done: impl Fn(Reply) + 'static,
    ) {
        let (name, email, password) = (name.to_string(), email.to_string(), password.to_string());
        let auth = self.clone();
        Self::later(move || {
            let reply = auth.store.borrow_mut().register(&name, &email, &password);
            done(reply.clone().map(|_| ()));
            if let Ok(user) = reply {
                auth.start_session(user);
            }
        });
    }

    /// Asks for a reset link. It always succeeds, whether or not the address has an account.
    pub(crate) fn request_password_reset(&self, email: &str, done: impl Fn(Reply) + 'static) {
        let email = email.to_string();
        let auth = self.clone();
        Self::later(move || {
            auth.store.borrow_mut().request_password_reset(&email);
            done(Ok(()));
        });
    }

    /// Changes the signed-in user's display name.
    pub(crate) fn update_name(&self, name: &str, done: impl Fn(Reply) + 'static) {
        let Some(id) = self.user.with(|user| user.as_ref().map(|u| u.id.clone())) else {
            done(Err(AuthError::AccountGone));
            return;
        };
        let name = name.to_string();
        let auth = self.clone();
        Self::later(move || {
            let reply = auth.store.borrow_mut().rename(&id, &name);
            done(reply.clone().map(|_| ()));
            if let Ok(user) = reply {
                // Every page that shows the name reads it from here, and setting the value
                // asks for a rebuild: they all show the new one.
                auth.user.set(Some(user));
            }
        });
    }

    /// Signs out, at once: nothing to wait for. The router hears it and goes to the login
    /// page.
    pub(crate) fn sign_out(&self) {
        self.session.clear();
        self.user.set(None);
    }

    fn start_session(&self, user: User) {
        self.session.save(&user.id);
        self.user.set(Some(user));
    }

    /// Runs `answer` once the pretend network has taken its time.
    ///
    /// `host::after` asks the shell for a timer — a `setTimeout` on the Web, the shell's
    /// timer wheel natively — and runs `answer` on the interface's thread, where it may
    /// change state. Never answer synchronously from a handler: the page's state is being
    /// changed at that moment, and `done` would try to change it again.
    fn later(answer: impl Fn() + 'static) {
        host::after(LATENCY, answer);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;

    use frus::host::Effect;

    use super::*;
    use crate::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};

    /// Plays the shell's part: runs the timers the service asked for, at once.
    pub(crate) fn answer_now() {
        for effect in host::take_effects() {
            if let Effect::After(_, callback) = effect {
                callback.call();
            }
        }
    }

    pub(crate) fn signed_out() -> Auth {
        Auth::with(UserStore::seeded(), Session::in_memory())
    }

    /// Runs a request and hands back its reply.
    fn reply_of(request: impl FnOnce(Box<dyn Fn(Reply)>)) -> Option<Reply> {
        let got: Rc<RefCell<Option<Reply>>> = Rc::default();
        let into = got.clone();
        request(Box::new(move |reply| *into.borrow_mut() = Some(reply)));
        answer_now();
        got.take()
    }

    #[test]
    fn signing_in_waits_for_the_server_then_sets_the_user() {
        let auth = signed_out();
        let replied = Rc::new(Cell::new(false));
        let flag = replied.clone();
        auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, move |reply| {
            assert_eq!(reply, Ok(()));
            flag.set(true);
        });
        assert!(
            !auth.is_signed_in(),
            "nothing happens before the server answers"
        );
        answer_now();
        assert!(replied.get());
        assert_eq!(auth.user().map(|u| u.email), Some(DEMO_EMAIL.to_string()));
    }

    #[test]
    fn a_refusal_reaches_the_page_and_signs_nobody_in() {
        let auth = signed_out();
        let reply = reply_of(|done| auth.sign_in(DEMO_EMAIL, "wrong-password", done));
        assert_eq!(reply, Some(Err(AuthError::WrongPassword)));
        assert!(!auth.is_signed_in());
    }

    #[test]
    fn registering_signs_the_new_account_in() {
        let auth = signed_out();
        let reply = reply_of(|done| auth.register("Ada", "ada@frus.dev", "analytical", done));
        assert_eq!(reply, Some(Ok(())));
        assert_eq!(auth.user().map(|u| u.name), Some("Ada".to_string()));
    }

    #[test]
    fn a_new_name_is_what_every_page_reads() {
        let auth = signed_out();
        reply_of(|done| auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, done));
        let reply = reply_of(|done| auth.update_name("Grace", done));
        assert_eq!(reply, Some(Ok(())));
        assert_eq!(auth.user().map(|u| u.name), Some("Grace".to_string()));
    }

    #[test]
    fn signing_out_tells_the_listeners() {
        let auth = signed_out();
        reply_of(|done| auth.sign_in(DEMO_EMAIL, DEMO_PASSWORD, done));
        let told = Rc::new(Cell::new(0));
        let count = told.clone();
        let _subscription =
            frus::Listenable::add_listener(auth.changes(), move || count.set(count.get() + 1));
        auth.sign_out();
        assert!(!auth.is_signed_in());
        assert_eq!(told.get(), 1);
    }

    #[test]
    fn a_saved_session_signs_in_at_start_up_until_signing_out() {
        let path = std::env::temp_dir()
            .join("frus-authentication-test")
            .join("session-restore");
        let first = Auth::with(UserStore::seeded(), Session::at(path.clone()));
        first.sign_out();
        reply_of(|done| first.sign_in(DEMO_EMAIL, DEMO_PASSWORD, done));

        // A restart: a new service, a new (seeded) server, the same file.
        let second = Auth::with(UserStore::seeded(), Session::at(path.clone()));
        assert_eq!(second.user().map(|u| u.email), Some(DEMO_EMAIL.to_string()));

        second.sign_out();
        let third = Auth::with(UserStore::seeded(), Session::at(path));
        assert!(!third.is_signed_in());
    }

    #[test]
    fn a_session_for_an_account_the_server_lost_is_dropped() {
        let path = std::env::temp_dir()
            .join("frus-authentication-test")
            .join("session-lost");
        Session::at(path.clone()).save("u42");
        let auth = Auth::with(UserStore::seeded(), Session::at(path.clone()));
        assert!(!auth.is_signed_in());
        assert_eq!(Session::at(path).load(), None, "and the file is cleared");
    }
}
