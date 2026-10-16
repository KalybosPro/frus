use frus::{
    Align, AppBar, BuildContext, Callback, Card, Column, ConstrainedBox, FontWeight, IconButton,
    Icons, Scaffold, SingleChildScrollView, StatelessWidget, Text, Widget,
};

use crate::common::auth::Auth;
use crate::common::routes::PROFILE;
use crate::features::home::widgets::user_card::UserCard;

/// The page behind the sign-in. It has no state of its own — what it shows is the
/// service's — so it is a `StatelessWidget`.
pub(crate) struct HomePage {
    auth: Auth,
}

impl HomePage {
    pub(crate) fn new(auth: Auth) -> Self {
        Self { auth }
    }
}

impl StatelessWidget for HomePage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let router = cx.router().clone();

        // Signing out only changes the service. The router listens to it, runs the guard
        // again, and the guard sends the person to the login page.
        let sign_out = {
            let auth = self.auth.clone();
            Callback::new(move || auth.sign_out())
        };

        let app_bar = AppBar::new()
            .title(Text::new("Home"))
            .action(
                IconButton::new(Icons::ACCOUNT_CIRCLE)
                    .label("Profile")
                    .on_press(Callback::new(move || router.push(PROFILE))),
            )
            .action(
                IconButton::new(Icons::LOGOUT)
                    .label("Sign out")
                    .on_press(sign_out),
            )
            .build();

        // The guard keeps this page for signed-in people, but a page is built from what is
        // true *now*: during the frame that signs out, there may be nobody.
        let Some(user) = self.auth.user() else {
            return Scaffold::new()
                .background(theme.background)
                .app_bar(app_bar)
                .build();
        };

        let first_name = user.name.split_whitespace().next().unwrap_or("there");
        let note = Card::new().filled().padding(16.0).child(
            Column::new()
                .spacing(6.0)
                .cross_axis_alignment(Align::Start)
                .child(Text::new("Your session is remembered").weight(FontWeight::SemiBold))
                .child(
                    Text::new(
                        "Close the application and start it again: you are still signed in. \
                         Sign out from the bar above or from your profile.",
                    )
                    .size(13.0)
                    .color(theme.muted)
                    .wrap(),
                ),
        );

        let content = Column::new()
            .spacing(20.0)
            .cross_axis_alignment(Align::Stretch)
            .child(
                Text::new(format!("Welcome, {first_name}!"))
                    .size(28.0)
                    .weight(FontWeight::SemiBold),
            )
            .child(UserCard::new(user).into_widget())
            .child(note);

        Scaffold::new()
            .background(theme.background)
            .app_bar(app_bar)
            .body(
                SingleChildScrollView::new()
                    .padding_each(24.0, 20.0, 24.0, 20.0)
                    .child(ConstrainedBox::new(content).max_width(640.0)),
            )
            .build()
    }
}
