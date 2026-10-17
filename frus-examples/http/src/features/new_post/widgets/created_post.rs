use frus::{
    Align, BuildContext, Card, Column, FontWeight, Icon, Icons, Row, StatelessWidget, Text, Widget,
};

use crate::models::post::Post;

/// What the server answered to `POST /posts`: the post as it stored it, with the id it
/// gave.
pub struct CreatedPost {
    post: Post,
}

impl CreatedPost {
    pub fn new(post: Post) -> Self {
        Self { post }
    }
}

impl StatelessWidget for CreatedPost {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let field = |name: &str, value: String| {
            Row::new()
                .spacing(8.0)
                .child(Text::new(format!("{name}:")).color(theme.muted))
                .child(Text::new(value))
        };
        Box::new(
            Card::new().outlined().padding(16.0).child(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(
                        Row::new()
                            .spacing(8.0)
                            .cross_axis_alignment(Align::Center)
                            .child(Icon::new(Icons::CHECK_CIRCLE).color(theme.primary))
                            .child(Text::new("Created").weight(FontWeight::SemiBold)),
                    )
                    .child(field("id", self.post.id.to_string()))
                    .child(field("userId", self.post.user_id.to_string()))
                    .child(field("title", self.post.title.clone()))
                    .child(Text::new(self.post.body.clone()))
                    // Worth saying, or the next question is "where is it in the list?"
                    .child(
                        Text::new(
                            "The test server echoes what it receives but stores nothing: \
                             every new post gets id 101, and the list does not change.",
                        )
                        .size(13.0)
                        .color(theme.muted),
                    ),
            ),
        )
    }
}
