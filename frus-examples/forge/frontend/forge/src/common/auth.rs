use frus::ValueNotifier;

use crate::models::user::User;

#[derive(Clone)]
pub(crate) struct Auth {
    pub user: ValueNotifier<Option<User>>,
}

impl Auth {
    // Nobody is signed in.
    pub(crate) fn new() -> Self {
        Auth {
            user: ValueNotifier::new(None),
        }
    }

    // pub(crate) fn set_user(&self, user: Option<User>) {
    //     self.user.set(user)
    // }

    // pub(crate) fn user(&self) -> Option<User> {
    //     self.user.get()
    // }

    // pub(crate) fn is_signed_in(&self) -> bool {
    //     self.user.with(|user| user.is_some())
    // }

    // pub(crate) fn sign_in(&self, email: String, password: String) {
    //     self.user.set(Some(User {
    //         id: "".to_string(),
    //         name: "".to_string(),
    //         email,
    //         password,
    //     }));
    // }

    pub(crate) fn logout(&self) {
        self.user.set(None);
    }

    // What the router listens to: a `ValueNotifier` is a `Listenable`.
    // pub(crate) fn changes(&self) -> &ValueNotifier<Option<User>> {
    //     &self.user
    // }
}
