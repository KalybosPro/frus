use frus::*;

use crate::common::auth::Auth;
use crate::common::routes::*;
use crate::features::auth::pages::forgetpassword::ForgetPasswordPage;
use crate::features::auth::pages::loginpage::LoginPage;
use crate::features::auth::pages::register::RegisterPage;
use crate::features::home::pages::homepage::HomePage;

pub(crate) fn router(auth: Auth) -> GoRouter {
    let (for_home, for_login,
        //  guard
    ) = (auth.clone(), auth.clone(),
    //  auth.clone()
);

    GoRouter::new(vec![
        GoRoute::new(LOGIN, move |_, _| {
            LoginPage::new(for_login.clone()).into_widget()
        }),
        GoRoute::new(HOME, move |_, _| {
            HomePage::new(for_home.clone()).into_widget()
        }),
        GoRoute::new(REGISTER, move |_, _| RegisterPage::new().into_widget()),
        GoRoute::new(FORGET_PASSWORD, move |_, _| {
            ForgetPasswordPage::new().into_widget()
        }),
    ])
    .initial_location(LOGIN)
    // .redirect(move |state| guard_redirect(guard.is_signed_in(), state.location()))
    // .refresh_listenable(auth.changes())
}

// /// Pages reachable without being signed in.
// const PUBLIC_ROUTES: [&str; 3] = [LOGIN, REGISTER, FORGET_PASSWORD];

// fn guard_redirect(signed_in: bool, location: &str) -> Option<String> {
//     let public = PUBLIC_ROUTES.contains(&location);
//     match (signed_in, public) {
//         (false, true) => None,
//         (false, false) => Some(HOME.to_string()),
//         (true, true) => Some(HOME.to_string()),
//         (true, false) => None,
//     }
// }
