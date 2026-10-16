use frus::{
    Align, AutofillHint, Button, Callback, Column, Icons, Justify, KeyboardType, Row, State,
    StateContext, StatefulWidget, Text, TextField, TextInputAction, Variant, Widget,
};

use crate::common::auth::Auth;
use crate::common::routers::with_from;
use crate::common::routes::LOGIN;
use crate::common::shared::{app_logo, auth_frame, heading, server_error, submit_button};
use crate::features::auth::states::registerstate::RegisterState;

/// The registration page. Creating the account signs into it, so — like the login page —
/// it never navigates on success: the router does.
pub(crate) struct RegisterPage {
    auth: Auth,
    from: Option<String>,
}

impl RegisterPage {
    pub(crate) fn new(auth: Auth, from: Option<String>) -> Self {
        Self { auth, from }
    }
}

impl StatefulWidget for RegisterPage {
    type State = RegisterState;

    fn create_state(&self) -> RegisterState {
        RegisterState::default()
    }
}

impl State for RegisterState {
    type Widget = RegisterPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let page = cx.widget();
        let form = self.form();
        // Each field shows its error once the form has been submitted.
        let error = |key| self.submit.error(&form, key);

        let register = {
            let auth = page.auth.clone();
            let handle = cx.handle();
            cx.callback(move |s: &mut RegisterState| {
                if s.submit.start(s.form().is_valid()) {
                    let handle = handle.clone();
                    auth.register(&s.name, &s.email, &s.password, move |reply| {
                        handle.set_state(move |s| s.submit.finish(reply))
                    });
                }
            })
        };

        // Back to sign in: the page underneath when there is one, else a fresh one.
        let to_login = {
            let router = cx.router().clone();
            let login = with_from(LOGIN, page.from.as_deref());
            Callback::new(move || {
                if !router.pop() {
                    router.go(&login);
                }
            })
        };

        let name = with_error(
            TextField::new(self.name.as_str())
                .label("Name")
                .prefix_icon(Icons::PERSON_OUTLINE)
                .autofill([AutofillHint::Name])
                .action(TextInputAction::Next)
                .on_input(cx.handler(RegisterState::set_name)),
            error("name"),
        );
        let email = with_error(
            TextField::new(self.email.as_str())
                .label("Email")
                .prefix_icon(Icons::MAIL_OUTLINE)
                .keyboard_type(KeyboardType::Email)
                .autofill([AutofillHint::Email, AutofillHint::NewUsername])
                .action(TextInputAction::Next)
                .on_input(cx.handler(RegisterState::set_email)),
            error("email"),
        );
        let visibility = if self.visibility {
            Icons::VISIBILITY_OFF
        } else {
            Icons::VISIBILITY
        };
        let password = with_error(
            TextField::new(self.password.as_str())
                .label("Password")
                .helper("At least 8 characters, one of them a digit")
                .prefix_icon(Icons::LOCK_OUTLINE)
                .obscure(!self.visibility)
                .suffix_icon(visibility)
                .on_suffix(cx.callback(RegisterState::toggle_visibility))
                // A *new* password: the platform offers to save it rather than fill it.
                .autofill([AutofillHint::NewPassword])
                .action(TextInputAction::Next)
                .on_input(cx.handler(RegisterState::set_password)),
            error("password"),
        );
        let confirm = with_error(
            TextField::new(self.confirm.as_str())
                .label("Confirm password")
                .prefix_icon(Icons::LOCK_OUTLINE)
                .obscure(!self.visibility)
                .autofill([AutofillHint::NewPassword])
                .action(TextInputAction::Done)
                .on_submit(register.clone())
                .on_input(cx.handler(RegisterState::set_confirm)),
            error("confirm"),
        );

        let have_account = Row::new()
            .main_axis_alignment(Justify::Center)
            .cross_axis_alignment(Align::Center)
            .child(Text::new("Already registered?").color(cx.theme().muted))
            .child(
                Button::new("Sign in")
                    .variant(Variant::Text)
                    .on_press(to_login),
            );

        let content = Column::new()
            .spacing(16.0)
            .child_boxed(app_logo(cx))
            .child_boxed(heading(
                cx,
                "Create your account",
                "It takes a few seconds, and signs you in.",
            ))
            .children(server_error(self.submit.server_error.as_deref()))
            .child(name)
            .child(email)
            .child(password)
            .child(confirm)
            .child_boxed(submit_button(
                cx,
                "Create account",
                "Creating your account…",
                self.submit.loading,
                register,
            ))
            .child(have_account);

        auth_frame(cx, content)
    }
}

/// `field`, showing `error` when there is one.
fn with_error(field: TextField, error: Option<String>) -> TextField {
    match error {
        Some(error) => field.error(error),
        None => field,
    }
}
