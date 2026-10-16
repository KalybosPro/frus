//! A pretend authentication server, kept in memory.
//!
//! It stands where a real backend would: it answers sign-in, registration, password resets
//! and profile changes, and refuses with an [`AuthError`] the way a server answers with an
//! error status. Nothing here waits — the delay a network adds is simulated by
//! [`Auth`](crate::common::auth::Auth), which calls this store a moment later.
//!
//! Plain Rust with no widget in sight, so every rule is tested below without a window.
//! Replacing it with HTTP calls (`frus::fetch`) changes this file and nothing else.

use std::fmt;

use crate::models::user::User;

/// The account every fresh start knows about, shown on the login page.
pub(crate) const DEMO_EMAIL: &str = "demo@frus.dev";
pub(crate) const DEMO_PASSWORD: &str = "password123";
const DEMO_NAME: &str = "Demo User";

/// Why the server said no. `Display` is the sentence the page shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AuthError {
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
struct Account {
    user: User,
    /// Kept in clear because this server is a toy. A real one keeps a slow hash
    /// (argon2, bcrypt) and never the password itself.
    password: String,
}

/// The accounts, and the password-reset emails the server has "sent".
pub(crate) struct UserStore {
    accounts: Vec<Account>,
    next_id: u32,
    /// The addresses a reset link went to — what a real server would hand to its mailer.
    outbox: Vec<String>,
}

/// Emails are compared the way people type them: surrounding spaces and capitals do not
/// make a different address.
fn normalize(email: &str) -> String {
    email.trim().to_lowercase()
}

impl UserStore {
    /// A server that knows nobody.
    pub(crate) fn empty() -> Self {
        Self {
            accounts: Vec::new(),
            next_id: 1,
            outbox: Vec::new(),
        }
    }

    /// A server with the demo account, the way the application starts.
    pub(crate) fn seeded() -> Self {
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
    pub(crate) fn sign_in(&self, email: &str, password: &str) -> Result<User, AuthError> {
        let account = self.account(email).ok_or(AuthError::UnknownEmail)?;
        if account.password != password {
            return Err(AuthError::WrongPassword);
        }
        Ok(account.user.clone())
    }

    /// Creates an account, and answers with its user.
    pub(crate) fn register(
        &mut self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<User, AuthError> {
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
    pub(crate) fn request_password_reset(&mut self, email: &str) {
        if let Some(account) = self.account(email) {
            let address = account.user.email.clone();
            self.outbox.push(address);
        }
    }

    /// The addresses a reset link was sent to (what the tests check).
    #[cfg(test)]
    pub(crate) fn outbox(&self) -> &[String] {
        &self.outbox
    }

    /// The user behind a session id — how a saved session is checked at start-up.
    pub(crate) fn user(&self, id: &str) -> Option<User> {
        self.accounts
            .iter()
            .find(|a| a.user.id == id)
            .map(|a| a.user.clone())
    }

    /// Changes a user's display name, and answers with the updated user.
    pub(crate) fn rename(&mut self, id: &str, name: &str) -> Result<User, AuthError> {
        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.user.id == id)
            .ok_or(AuthError::AccountGone)?;
        account.user.name = name.trim().to_string();
        Ok(account.user.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_account_signs_in() {
        let store = UserStore::seeded();
        let user = store.sign_in(DEMO_EMAIL, DEMO_PASSWORD);
        assert_eq!(user.map(|u| u.name), Ok(DEMO_NAME.to_string()));
    }

    #[test]
    fn the_email_is_matched_however_it_is_typed() {
        let store = UserStore::seeded();
        assert!(store.sign_in("  Demo@FRUS.dev ", DEMO_PASSWORD).is_ok());
    }

    #[test]
    fn each_refusal_says_why() {
        let mut store = UserStore::seeded();
        assert_eq!(
            store.sign_in("nobody@frus.dev", DEMO_PASSWORD),
            Err(AuthError::UnknownEmail)
        );
        assert_eq!(
            store.sign_in(DEMO_EMAIL, "not-the-password"),
            Err(AuthError::WrongPassword)
        );
        assert_eq!(
            store.register("Someone", DEMO_EMAIL, "whatever1"),
            Err(AuthError::EmailTaken)
        );
        assert_eq!(store.rename("u999", "Ghost"), Err(AuthError::AccountGone));
    }

    #[test]
    fn a_new_account_can_sign_in_and_has_its_own_id() {
        let mut store = UserStore::seeded();
        let ada = store.register(" Ada ", "ada@frus.dev", "analytical");
        let Ok(ada) = ada else {
            panic!("registering a new address should succeed: {ada:?}");
        };
        assert_eq!(ada.name, "Ada");
        assert_ne!(ada.id, "u1", "the demo account already has u1");
        assert_eq!(store.sign_in("ada@frus.dev", "analytical"), Ok(ada.clone()));
        assert_eq!(store.user(&ada.id), Some(ada));
    }

    #[test]
    fn a_reset_link_only_goes_to_a_known_address() {
        let mut store = UserStore::seeded();
        store.request_password_reset("nobody@frus.dev");
        store.request_password_reset(" DEMO@frus.dev");
        assert_eq!(store.outbox(), [DEMO_EMAIL.to_string()]);
    }

    #[test]
    fn renaming_changes_what_the_server_answers() {
        let mut store = UserStore::seeded();
        assert!(store.rename("u1", "Grace").is_ok());
        assert_eq!(store.user("u1").map(|u| u.name), Some("Grace".to_string()));
    }
}
