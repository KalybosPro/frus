use frus::{
    Align, BuildContext, Card, CircleAvatar, Column, Expanded, FontWeight, Row, StatelessWidget,
    Text, Widget,
};

use crate::models::user::User;

/// Who is signed in: an avatar with their initials, their name, email and id.
///
/// A `StatelessWidget` configured by its parent with the user to show — it knows nothing
/// of the service, so it can show any user anywhere.
pub struct UserCard {
    user: User,
}

impl UserCard {
    pub fn new(user: User) -> Self {
        Self { user }
    }
}

impl StatelessWidget for UserCard {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let avatar = CircleAvatar::initials(self.user.name.as_str())
            .radius(28.0)
            .background_color(theme.primary_container)
            .foreground_color(theme.on_primary_container);

        let details = Column::new()
            .spacing(4.0)
            .cross_axis_alignment(Align::Start)
            .child(
                Text::new(self.user.name.as_str())
                    .size(18.0)
                    .weight(FontWeight::SemiBold),
            )
            .child(Text::new(self.user.email.as_str()).color(theme.muted))
            .child(
                Text::new(format!("Session id: {}", self.user.id))
                    .size(12.0)
                    .color(theme.muted),
            );

        Box::new(
            Card::new().outlined().padding(20.0).child(
                Row::new()
                    .spacing(16.0)
                    .cross_axis_alignment(Align::Center)
                    .child(avatar)
                    .child(Expanded::new(details)),
            ),
        )
    }
}
