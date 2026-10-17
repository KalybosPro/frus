use frus::{
    Align, AutofillHint, Button, Callback, Card, Column, FontWeight, Icons, Justify, KeyboardType,
    Row, State, StateContext, StatefulWidget, Text, TextField, TextInputAction, Variant, Widget,
};

use crate::common::auth::Auth;
use crate::common::routers::with_from;
use crate::common::routes::{FORGOT_PASSWORD, REGISTER};
use crate::common::shared::{app_logo, auth_frame, heading, server_error, submit_button};
use crate::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};
use crate::features::auth::states::loginstate::LoginState;

/// The sign-in page's configuration: the service it signs in with, and where the person
/// was heading (`?from=..`), which it passes on to the register and reset pages.
pub struct LoginPage {
    auth: Auth,
    from: Option<String>,
}

impl LoginPage {
    pub fn new(auth: Auth, from: Option<String>) -> Self {
        Self { auth, from }
    }
}

impl StatefulWidget for LoginPage {
    type State = LoginState;

    fn create_state(&self) -> LoginState {
        LoginState::default()
    }
}

impl State for LoginState {
    type Widget = LoginPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let page = cx.widget();
        let router = cx.router().clone();
        let form = self.form();

        // Submitting: the state decides whether to send (`Submission::start`), the service
        // sends, and its answer comes back through the state's handle — `cx.handle()` is
        // how something that finishes later (a timer, a request) changes the state.
        let sign_in = {
            let auth = page.auth.clone();
            let handle = cx.handle();
            cx.callback(move |s: &mut LoginState| {
                if s.submit.start(s.form().is_valid()) {
                    let handle = handle.clone();
                    auth.sign_in(&s.email, &s.password, move |reply| {
                        handle.set_state(move |s| s.submit.finish(reply))
                    });
                }
            })
        };
        // On success there is nothing to do here: the router hears the change and leaves
        // this page for wherever the person was heading.

        // `push` keeps this page underneath, so going back returns here; `from` travels on.
        let open = |path: &str| -> Callback {
            let (router, location) = (router.clone(), with_from(path, page.from.as_deref()));
            Callback::new(move || router.push(&location))
        };

        let mut email = TextField::new(self.email.as_str())
            .label("Email")
            .placeholder("you@example.com")
            .prefix_icon(Icons::MAIL_OUTLINE)
            .keyboard_type(KeyboardType::Email)
            .autofill([AutofillHint::Email, AutofillHint::Username])
            .action(TextInputAction::Next)
            .on_input(cx.handler(LoginState::set_email));
        if let Some(error) = self.submit.error(&form, "email") {
            email = email.error(error);
        }

        let mut password = TextField::new(self.password.as_str())
            .label("Password")
            .prefix_icon(Icons::LOCK_OUTLINE)
            .obscure(!self.visibility)
            .suffix_icon(if self.visibility {
                Icons::VISIBILITY_OFF
            } else {
                Icons::VISIBILITY
            })
            .on_suffix(cx.callback(LoginState::toggle_visibility))
            .autofill([AutofillHint::Password])
            .action(TextInputAction::Done)
            // Enter in the password field submits, as people expect.
            .on_submit(sign_in.clone())
            .on_input(cx.handler(LoginState::set_password));
        if let Some(error) = self.submit.error(&form, "password") {
            password = password.error(error);
        }

        let forgot = Row::new().main_axis_alignment(Justify::End).child(
            Button::new("Forgot password?")
                .variant(Variant::Text)
                .on_press(open(FORGOT_PASSWORD)),
        );

        let create_account = Row::new()
            .main_axis_alignment(Justify::Center)
            .cross_axis_alignment(Align::Center)
            .child(Text::new("No account yet?").color(cx.theme().muted))
            .child(
                Button::new("Create one")
                    .variant(Variant::Text)
                    .on_press(open(REGISTER)),
            );

        let subtitle = match &page.from {
            Some(from) => format!("Sign in to continue to {from}."),
            None => "Sign in to continue.".to_string(),
        };

        let content = Column::new()
            .spacing(16.0)
            .child_boxed(app_logo(cx))
            .child_boxed(heading(cx, "Welcome back", &subtitle))
            .child(demo_account(cx))
            .children(server_error(self.submit.server_error.as_deref()))
            .child(email)
            .child(password)
            .child(forgot)
            .child_boxed(submit_button(
                cx,
                "Sign in",
                "Signing in…",
                self.submit.loading,
                sign_in,
            ))
            .child(create_account);

        auth_frame(cx, content)
    }
}

/// The seeded account, so that the example can be tried at once — with a button that
/// fills the form in.
fn demo_account(cx: &StateContext<LoginState>) -> Card {
    let muted = cx.theme().muted;
    Card::new().outlined().padding(14.0).child(
        Row::new()
            .spacing(12.0)
            .cross_axis_alignment(Align::Center)
            .child(frus::Expanded::new(
                Column::new()
                    .spacing(2.0)
                    .cross_axis_alignment(Align::Start)
                    .child(
                        Text::new("Demo account")
                            .weight(FontWeight::SemiBold)
                            .size(13.0),
                    )
                    .child(Text::new(DEMO_EMAIL).size(13.0).color(muted))
                    .child(Text::new(DEMO_PASSWORD).size(13.0).color(muted)),
            ))
            .child(
                Button::new("Use it")
                    .variant(Variant::Tonal)
                    .on_press(cx.callback(LoginState::use_demo_account)),
            ),
    )
}
