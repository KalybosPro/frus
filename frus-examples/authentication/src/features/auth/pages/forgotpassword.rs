use frus::{
    AutofillHint, Button, Callback, Center, Column, Icon, Icons, KeyboardType, State, StateContext,
    StatefulWidget, Text, TextAlign, TextField, TextInputAction, Variant, Widget,
};

use crate::common::auth::Auth;
use crate::common::routers::with_from;
use crate::common::routes::LOGIN;
use crate::common::shared::{app_logo, auth_frame, heading, server_error, submit_button};
use crate::features::auth::states::forgotpasswordstate::ForgotPasswordState;

/// Asks for a password reset link, then says to check the inbox.
pub struct ForgotPasswordPage {
    auth: Auth,
    from: Option<String>,
}

impl ForgotPasswordPage {
    pub fn new(auth: Auth, from: Option<String>) -> Self {
        Self { auth, from }
    }
}

impl StatefulWidget for ForgotPasswordPage {
    type State = ForgotPasswordState;

    fn create_state(&self) -> ForgotPasswordState {
        ForgotPasswordState::default()
    }
}

impl State for ForgotPasswordState {
    type Widget = ForgotPasswordPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let page = cx.widget();

        let to_login = {
            let router = cx.router().clone();
            let login = with_from(LOGIN, page.from.as_deref());
            Callback::new(move || {
                if !router.pop() {
                    router.go(&login);
                }
            })
        };

        // One page, two faces: the state says which.
        let content = match &self.sent_to {
            Some(email) => check_your_email(cx, email, to_login),
            None => self.request_form(cx, to_login),
        };
        auth_frame(cx, content)
    }
}

impl ForgotPasswordState {
    fn request_form(&self, cx: &StateContext<Self>, to_login: Callback) -> Column {
        let form = self.form();

        let send = {
            let auth = cx.widget().auth.clone();
            let handle = cx.handle();
            cx.callback(move |s: &mut ForgotPasswordState| {
                if s.submit.start(s.form().is_valid()) {
                    let handle = handle.clone();
                    auth.request_password_reset(&s.email, move |reply| {
                        handle.set_state(move |s| s.finish(reply))
                    });
                }
            })
        };

        let mut email = TextField::new(self.email.as_str())
            .label("Email")
            .prefix_icon(Icons::MAIL_OUTLINE)
            .keyboard_type(KeyboardType::Email)
            .autofill([AutofillHint::Email])
            .action(TextInputAction::Send)
            .on_submit(send.clone())
            .on_input(cx.handler(ForgotPasswordState::set_email));
        if let Some(error) = self.submit.error(&form, "email") {
            email = email.error(error);
        }

        Column::new()
            .spacing(16.0)
            .child_boxed(app_logo(cx))
            .child_boxed(heading(
                cx,
                "Forgot your password?",
                "Enter the email address of your account and we will send you a link to \
                 choose a new one.",
            ))
            .children(server_error(self.submit.server_error.as_deref()))
            .child(email)
            .child_boxed(submit_button(
                cx,
                "Send the link",
                "Sending…",
                self.submit.loading,
                send,
            ))
            .child(
                Button::new("Back to sign in")
                    .variant(Variant::Text)
                    .on_press(to_login),
            )
    }
}

/// What the page shows once the request is in. It says the same whether or not the address
/// has an account (see `UserStore::request_password_reset`).
fn check_your_email(
    cx: &StateContext<ForgotPasswordState>,
    email: &str,
    to_login: Callback,
) -> Column {
    let theme = cx.theme();
    Column::new()
        .spacing(16.0)
        .child(Center::new(
            Icon::new(Icons::MARK_EMAIL_READ)
                .size(64.0)
                .color(theme.primary),
        ))
        .child_boxed(heading(
            cx,
            "Check your email",
            &format!(
                "If an account uses {email}, a link to reset its password is on its way. \
                 It can take a minute to arrive."
            ),
        ))
        .child(
            Text::new("Nothing after a few minutes? Look in the spam folder, or try again.")
                .size(13.0)
                .color(theme.muted)
                .align(TextAlign::Center)
                .wrap(),
        )
        .child(
            Button::with_child(Text::new("Back to sign in"))
                .height(48.0)
                .on_press(to_login),
        )
        .child(
            Button::new("Use another address")
                .variant(Variant::Text)
                .on_press(cx.callback(ForgotPasswordState::try_again)),
        )
}
