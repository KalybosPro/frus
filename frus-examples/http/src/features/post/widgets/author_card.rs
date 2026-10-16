use frus::{
    Align, AsyncSnapshot, BuildContext, Card, CircleAvatar, CircularProgressIndicator, Column,
    Component, Expanded, FontWeight, FutureBuilder, Row, Text, Widget,
};

use crate::common::api::{self, ApiError};
use crate::models::user::User;

/// The post's author, from `GET /users/{id}`.
///
/// `FutureBuilder` is the widget form of `cx.use_future`: it runs the future made from
/// its key once per key, and builds from what is known of it. It suits a piece of a page
/// that loads on its own: this card asks for the author only once the post — which says
/// who the author is — has arrived, and the rest of the page does not wait for it.
///
/// `attempt` is part of the key so the page's Retry asks for the author again too.
pub(crate) fn author_card(user_id: u32, attempt: u64) -> Component {
    FutureBuilder::new(
        (user_id, attempt),
        |&(user_id, _): &(u32, u64)| api::user(user_id),
        |cx, snapshot| Box::new(card(cx, snapshot)),
    )
}

fn card(cx: &BuildContext, snapshot: AsyncSnapshot<Result<User, ApiError>>) -> Card {
    let theme = cx.theme();
    // `data` is `None` while the request is out, then the future's output — a `Result`,
    // because a failure is an answer like any other.
    let content: Box<dyn Widget> = match snapshot.data {
        None => Box::new(
            Row::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                .child(CircularProgressIndicator::new().size(20.0))
                .child(Text::new("Loading the author…").color(theme.muted)),
        ),
        Some(Err(error)) => Box::new(
            Text::new(format!("Author unavailable — {}", error.title())).color(theme.error),
        ),
        Some(Ok(user)) => Box::new(
            Row::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                .child(CircleAvatar::initials(user.name.clone()))
                .child(Expanded::new(
                    Column::new()
                        .cross_axis_alignment(Align::Start)
                        .spacing(2.0)
                        .child(Text::new(user.name).weight(FontWeight::SemiBold))
                        .child(
                            Text::new(format!("@{} · {}", user.username, user.email))
                                .size(13.0)
                                .color(theme.muted)
                                .ellipsis(),
                        )
                        .child(
                            Text::new(format!(
                                "{} — “{}” · {}",
                                user.company.name, user.company.catch_phrase, user.website
                            ))
                            .size(13.0)
                            .color(theme.muted),
                        ),
                )),
        ),
    };
    Card::new().filled().padding(16.0).child(content)
}
