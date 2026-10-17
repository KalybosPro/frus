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
pub const LATENCY: Duration = Duration::from_millis(900);

/// What a request reports back to the page that made it.
pub type Reply<T = ()> = Result<T, AuthError>;

#[derive(Clone)]
pub struct Auth {
    user: ValueNotifier<Option<User>>,
    /// The server. Shared by every handle, hence the `Rc<RefCell<..>>`.
    store: Rc<RefCell<UserStore>>,
    session: Session,
}

impl Auth {
    /// The service the application runs with: the seeded server, and the session saved on
    /// this device — if it still names an account, the person starts signed in.
    // Not `Default`: it reads the session saved on the device, which a default should not.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with(UserStore::seeded(), Session::on_device())
    }

    /// A service over `store`, remembering the session in `session`.
    pub fn with(store: UserStore, session: Session) -> Self {
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
    pub fn user(&self) -> Option<User> {
        self.user.get()
    }

    pub fn is_signed_in(&self) -> bool {
        self.user.with(Option::is_some)
    }

    /// What the router listens to: a `ValueNotifier` is a `Listenable`.
    pub fn changes(&self) -> &ValueNotifier<Option<User>> {
        &self.user
    }

    /// Signs in. `done` is told how it went, a moment later, on the interface's thread.
    ///
    /// On success the user is set *after* `done` runs: the page that asked clears its
    /// loading state first, then the router — hearing the change — moves away from it.
    pub fn sign_in(&self, email: &str, password: &str, done: impl Fn(Reply) + 'static) {
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
    pub fn register(
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
    pub fn request_password_reset(&self, email: &str, done: impl Fn(Reply) + 'static) {
        let email = email.to_string();
        let auth = self.clone();
        Self::later(move || {
            auth.store.borrow_mut().request_password_reset(&email);
            done(Ok(()));
        });
    }

    /// Changes the signed-in user's display name.
    pub fn update_name(&self, name: &str, done: impl Fn(Reply) + 'static) {
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
    pub fn sign_out(&self) {
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
