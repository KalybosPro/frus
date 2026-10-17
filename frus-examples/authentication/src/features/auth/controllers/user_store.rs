//! A pretend authentication server, kept in memory.
//!
//! It stands where a real backend would: it answers sign-in, registration, password resets
//! and profile changes, and refuses with an [`AuthError`] the way a server answers with an
//! error status. Nothing here waits — the delay a network adds is simulated by
//! [`Auth`](crate::common::auth::Auth), which calls this store a moment later.
//!
//! Plain Rust with no widget in sight, so every rule is tested in `tests/user_store_test.rs` without a window.
//! Replacing it with HTTP calls (`frus::fetch`) changes this file and nothing else.

use std::fmt;

use crate::models::user::User;

/// The account every fresh start knows about, shown on the login page.
pub const DEMO_EMAIL: &str = "demo@frus.dev";
pub const DEMO_PASSWORD: &str = "password123";
pub const DEMO_NAME: &str = "Demo User";

/// Why the server said no. `Display` is the sentence the page shows.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    /// No account has this email address.
    UnknownEmail,
    /// The account exists, the password is not its password.
    WrongPassword,
    /// Registering an address that already has an account.
    EmailTaken,
    /// The session names an account the server no longer has.
    AccountGone,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A production service often answers the first two with the same sentence, so that
        // the form does not tell a stranger which addresses have an account. Here they are
        // kept apart to show that each error reaches the page.
        f.write_str(match self {
            AuthError::UnknownEmail => "No account uses this email address.",
            AuthError::WrongPassword => "That password is not right. Try again.",
            AuthError::EmailTaken => "An account already uses this email address.",
            AuthError::AccountGone => "This account no longer exists. Sign in again.",
        })
    }
}

/// One account on the server: the user, and the secret the server checks.
pub struct Account {
    user: User,
    /// Kept in clear because this server is a toy. A real one keeps a slow hash
    /// (argon2, bcrypt) and never the password itself.
    password: String,
}

/// The accounts, and the password-reset emails the server has "sent".
pub struct UserStore {
    accounts: Vec<Account>,
    next_id: u32,
    /// The addresses a reset link went to — what a real server would hand to its mailer.
    outbox: Vec<String>,
}

/// Emails are compared the way people type them: surrounding spaces and capitals do not
/// make a different address.
pub fn normalize(email: &str) -> String {
    email.trim().to_lowercase()
}

impl UserStore {
    /// A server that knows nobody.
    pub fn empty() -> Self {
        Self {
            accounts: Vec::new(),
            next_id: 1,
            outbox: Vec::new(),
        }
    }

    /// A server with the demo account, the way the application starts.
    pub fn seeded() -> Self {
        let mut store = Self::empty();
        // Cannot fail on an empty store; the result is ignored rather than unwrapped.
        let _ = store.register(DEMO_NAME, DEMO_EMAIL, DEMO_PASSWORD);
        store
    }

    fn account(&self, email: &str) -> Option<&Account> {
        let email = normalize(email);
        self.accounts.iter().find(|a| a.user.email == email)
    }

    /// Checks the credentials, and answers with the user they belong to.
    pub fn sign_in(&self, email: &str, password: &str) -> Result<User, AuthError> {
        let account = self.account(email).ok_or(AuthError::UnknownEmail)?;
        if account.password != password {
            return Err(AuthError::WrongPassword);
        }
        Ok(account.user.clone())
    }

    /// Creates an account, and answers with its user.
    pub fn register(&mut self, name: &str, email: &str, password: &str) -> Result<User, AuthError> {
        if self.account(email).is_some() {
            return Err(AuthError::EmailTaken);
        }
        let user = User {
            // The demo account is always `u1`, so a saved session finds it again after a
            // restart, when this in-memory server has been seeded afresh.
            id: format!("u{}", self.next_id),
            name: name.trim().to_string(),
            email: normalize(email),
        };
        self.next_id += 1;
        self.accounts.push(Account {
            user: user.clone(),
            password: password.to_string(),
        });
        Ok(user)
    }

    /// "Sends" a reset link if the address has an account. It answers the same either way:
    /// whether an address is registered is not the business of whoever typed it.
    pub fn request_password_reset(&mut self, email: &str) {
        if let Some(account) = self.account(email) {
            let address = account.user.email.clone();
            self.outbox.push(address);
        }
    }

    /// The addresses a reset link was sent to (what the tests check).
    pub fn outbox(&self) -> &[String] {
        &self.outbox
    }

    /// The user behind a session id — how a saved session is checked at start-up.
    pub fn user(&self, id: &str) -> Option<User> {
        self.accounts
            .iter()
            .find(|a| a.user.id == id)
            .map(|a| a.user.clone())
    }

    /// Changes a user's display name, and answers with the updated user.
    pub fn rename(&mut self, id: &str, name: &str) -> Result<User, AuthError> {
        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.user.id == id)
            .ok_or(AuthError::AccountGone)?;
        account.user.name = name.trim().to_string();
        Ok(account.user.clone())
    }
}
