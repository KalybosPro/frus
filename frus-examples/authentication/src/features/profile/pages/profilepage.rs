use frus::{
    Alert, Align, AppBar, AutofillHint, Button, Callback, Center, CircleAvatar, Column,
    ConstrainedBox, Icon, IconButton, Icons, Scaffold, SingleChildScrollView, State, StateContext,
    StatefulWidget, Text, TextField, TextInputAction, Variant, Widget,
};

use crate::common::auth::Auth;
use crate::common::routes::HOME;
use crate::common::shared::{server_error, submit_button};
use crate::features::profile::states::profilestate::ProfileState;

/// Edit the display name, or sign out.
pub(crate) struct ProfilePage {
    auth: Auth,
}

impl ProfilePage {
    pub(crate) fn new(auth: Auth) -> Self {
        Self { auth }
    }
}

impl StatefulWidget for ProfilePage {
    type State = ProfileState;

    /// The field starts with the current name — read from the service once, when the page
    /// appears; after that the field is the state's.
    fn create_state(&self) -> ProfileState {
        ProfileState::new(self.auth.user().map(|u| u.name).unwrap_or_default())
    }
}

impl State for ProfileState {
    type Widget = ProfilePage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();
        let auth = cx.widget().auth.clone();
        // The bar does not add a back arrow by itself: a page that can be left says so.
        // The route sits under home's, so even a page opened straight at `/profile` has
        // home underneath to pop back to.
        let back = {
            let router = cx.router().clone();
            Callback::new(move || {
                if !router.pop() {
                    router.go(HOME);
                }
            })
        };
        let app_bar = AppBar::new()
            .leading(
                IconButton::new(Icons::ARROW_BACK)
                    .label("Back")
                    .on_press(back),
            )
            .title(Text::new("Profile"))
            .build();

        let Some(user) = auth.user() else {
            return Scaffold::new()
                .background(theme.background)
                .app_bar(app_bar)
                .build();
        };
        let form = self.form(&user.name);

        let save = {
            let auth = auth.clone();
            let handle = cx.handle();
            cx.callback(move |s: &mut ProfileState| {
                let current = auth.user().map(|u| u.name).unwrap_or_default();
                if s.submit.start(s.form(&current).is_valid()) {
                    let handle = handle.clone();
                    auth.update_name(&s.name, move |reply| {
                        handle.set_state(move |s| s.finish(reply))
                    });
                }
            })
        };

        let mut name = TextField::new(self.name.as_str())
            .label("Display name")
            .prefix_icon(Icons::BADGE)
            .autofill([AutofillHint::Name])
            .action(TextInputAction::Done)
            .on_submit(save.clone())
            .on_input(cx.handler(ProfileState::set_name));
        if let Some(error) = self.submit.error(&form, "name") {
            name = name.error(error);
        }

        // The address is shown, not edited: changing it would need the new one confirmed.
        let email = TextField::new(user.email.as_str())
            .label("Email")
            .prefix_icon(Icons::MAIL_OUTLINE)
            .read_only();

        let saved = self
            .saved
            .then(|| Box::new(Alert::new("Your name was saved.").success()) as Box<dyn Widget>);

        let sign_out = Button::with_child(Text::new("Sign out"))
            .variant(Variant::Outlined)
            .icon(Icon::new(Icons::LOGOUT))
            .height(48.0)
            .on_press(Callback::new(move || auth.sign_out()));

        let content = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Stretch)
            .child(Center::new(
                CircleAvatar::initials(user.name.as_str())
                    .radius(40.0)
                    .background_color(theme.primary_container)
                    .foreground_color(theme.on_primary_container),
            ))
            .children(server_error(self.submit.server_error.as_deref()))
            .children(saved)
            .child(name)
            .child(email)
            .child_boxed(submit_button(
                cx,
                "Save",
                "Saving…",
                self.submit.loading,
                save,
            ))
            .child(sign_out);

        Scaffold::new()
            .background(theme.background)
            .app_bar(app_bar)
            .body(
                SingleChildScrollView::new()
                    .padding_each(24.0, 20.0, 24.0, 20.0)
                    .child(Center::new(ConstrainedBox::new(content).max_width(480.0))),
            )
            .build()
    }
}
