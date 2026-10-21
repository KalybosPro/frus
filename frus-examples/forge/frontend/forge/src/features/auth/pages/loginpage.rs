use std::ops::Sub;

use frus::{
    Align, Button, Callback, Center, CircularProgressIndicator, Colors, Column, Container, Divider,
    Expanded, FontWeight, GestureDetector, Icons, Insets, Justify, KeyboardType, MediaQuery,
    Padding, RichText, Row, Scaffold, SingleChildScrollView, SizedBox, State, StateContext,
    StatefulWidget, Text, TextAlign, TextField, TextSpan, TextStyle, Widget,
};

use crate::{
    common::{
        auth::Auth, colors::{black, gray, grey, primary_color, white}, routes::{FORGET_PASSWORD, HOME, REGISTER}, shared::{app_logo, card, input_style, input_title, on},
    }, features::auth::states::loginstate::LoginState,
};

pub(crate) struct LoginPage {
    auth: Auth,
}

impl LoginPage {
    pub(crate) fn new(auth: Auth) -> Self {
        Self { auth }
    }
}

impl StatefulWidget for LoginPage {
    type State = LoginState;

    fn create_state(&self) -> LoginState {
        LoginState {
            email: String::new(),
            password: String::new(),
            visibility: false,
            submitted: false,
            loading: false,
        }
    }
}

impl State for LoginState {
    type Widget = LoginPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let router = cx.router().clone();
        // instantiate auth got from widget
        let auth = cx.widget().auth.clone();

        let width = MediaQuery::of().size.width;

        let form = self.form();
        let error_of = |key| {
            if self.submitted {
                form.error(key)
            } else {
                None
            }
        };

        // The submit callback changes the state, then signs in only if the form is valid
        // let sign_in = cx.callback(move |s: &mut LoginState| {
        //     s.submitted = true;
        //     if s.form().is_valid() && !s.loading {
        //         s.loading = true;
        //         auth.sign_in(s.email.clone(), s.password.clone());
        //     }
        // });

        let go_to = |path: &'static str| -> Callback {
            let r = router.clone();
            on(move || {
                r.push(path);
            })
        };

        // Button to use to sign in

        let child: Box<dyn Widget> = if self.loading {
            Box::new(Center::new(CircularProgressIndicator::new()))
        } else {
            Box::new(Expanded::new(
                Button::new("Sign in")
                    .padding(8.0)
                    .color(primary_color())
                    .radius(12.0)
                    .elevation(0.0)
                    .label_color(white())
                    .label_style(TextStyle::new(14.0).weight(FontWeight::Regular))
                    .on_press(go_to(HOME)),
            ))
        };

        let login_button = Container::new().child(child).width(width.sub(40.0));

        let mut email_field = TextField::new(self.email.as_str())
            .placeholder("kalybos@frus.dev")
            .on_input(cx.handler(LoginState::set_email))
            .borderless()
            .keyboard_type(KeyboardType::Email)
            .style(input_style());

        if let Some(msg) = error_of("email") {
            email_field = email_field.error(msg);
        }

        let email = Column::new()
            .child(input_title("Email".to_string()))
            .child(card(Box::new(email_field)))
            .cross_axis_alignment(Align::Start)
            .spacing(8.0);

        let mut password_field = TextField::new(self.password.as_str())
            .placeholder("••••••••••••")
            .on_input(cx.handler(LoginState::set_password))
            .borderless()
            .keyboard_type(KeyboardType::Password)
            .obscure(!self.visibility)
            .style(input_style())
            .suffix_icon(if self.visibility {
                Icons::VISIBILITY_OFF
            } else {
                Icons::VISIBILITY
            })
            .on_suffix(cx.callback(LoginState::toggle_visibility));

        if let Some(msg) = error_of("password") {
            password_field = password_field.error(msg);
        }

        let password = Column::new()
            .child(input_title("Password".to_string()))
            .child(card(Box::new(password_field)))
            .cross_axis_alignment(Align::Start)
            .spacing(8.0);

        let input = Column::new()
            .child(email)
            .child(SizedBox::empty().height(24.0))
            .child(password);

        let content = Column::new().spacing(8.0).child(input).child(
            Row::new()
                .child(
                    GestureDetector::new(
                        Text::new("Forgot password?")
                            .color(primary_color())
                            .size(14.0)
                            .weight(FontWeight::Regular),
                    )
                    .on_tap(go_to(FORGET_PASSWORD)),
                )
                .main_axis_alignment(Justify::End),
        );

        let welcome = Column::new()
            .spacing(8.0)
            .main_axis_alignment(Justify::Center)
            .cross_axis_alignment(Align::Center)
            .child(
                Text::new("Welcome back")
                    .size(30.0)
                    .weight(FontWeight::SemiBold)
                    .color(black()),
            )
            .child(
                Text::new("Sign in to continue collaborating with your team.")
                    .weight(FontWeight::Regular)
                    .size(14.0)
                    .color(grey()),
            );

        let or_widget = Row::new()
            .child(Expanded::new(Divider::new().color(gray())))
            .child(Padding::new(
                Insets::new(0.0, 16.0, 0.0, 16.0),
                Text::new("OR")
                    .color(gray())
                    .size(16.0)
                    .weight(FontWeight::Medium),
            ))
            .child(Expanded::new(Divider::new().color(gray())));

        let github_button = card(Box::new(
            Text::new("Continue with GitHub")
                .size(14.0)
                .color(black())
                .align(TextAlign::Center)
                .weight(FontWeight::Regular),
        ));

        let create_account_button = GestureDetector::new(
            RichText::new(
                TextSpan::new("Don't have an account? ")
                    .color(grey())
                    .weight(FontWeight::Regular)
                    .size(14.0)
                    .child(
                        TextSpan::new("Create account")
                            .color(primary_color())
                            .weight(FontWeight::Regular)
                            .size(14.0),
                    ),
            )
            .align(TextAlign::Center),
        )
        .on_tap(go_to(REGISTER));

        let footer = Text::new("Forge . Learn by building together")
            .color(grey())
            .size(12.0)
            .align(TextAlign::Center);

        let page = Column::new()
            .child(SizedBox::empty().height(32.0))
            .child(app_logo())
            .child(SizedBox::empty().height(95.0))
            .child(welcome)
            .child(SizedBox::empty().height(32.0))
            .child(content)
            .child(SizedBox::empty().height(24.0))
            .child(login_button)
            .child(SizedBox::empty().height(32.0))
            .child(or_widget)
            .child(SizedBox::empty().height(23.5))
            .child(github_button)
            .child(SizedBox::empty().height(23.5))
            .child(create_account_button)
            .child(SizedBox::empty().height(110.0))
            .child(footer);

        Box::new(
            Scaffold::new()
                .background(Colors::WHITE)
                .body(
                    SingleChildScrollView::new()
                        .child(page)
                        .padding_each(32.0, 20.0, 32.0, 20.0),
                )
                .build(),
        )
    }
}
