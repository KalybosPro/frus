use frus::{
    Align, BuildContext, Button, Callback, CircularProgressIndicator, Column, FontWeight, Row,
    StatelessWidget, Text, Variant, Widget,
};

use crate::common::api::ApiError;
use crate::common::remote::Loaded;
use crate::models::comment::Comment;

/// The comments under a post, as far as they have loaded.
///
/// A `StatelessWidget` configured with the request's state: the page owns the request,
/// this only draws it. The section has its own loading and error lines, so a post whose
/// comments fail still shows the post.
pub struct CommentsSection {
    comments: Loaded<Vec<Comment>>,
    on_retry: Callback,
}

impl CommentsSection {
    pub fn new(comments: Loaded<Vec<Comment>>, on_retry: Callback) -> Self {
        Self { comments, on_retry }
    }
}

impl StatelessWidget for CommentsSection {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let heading = |label: String| Text::new(label).size(18.0).weight(FontWeight::SemiBold);

        let section = match &self.comments.data {
            frus::RemoteData::NotAsked | frus::RemoteData::Loading => Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(12.0)
                .child(heading("Comments".into()))
                .child(
                    Row::new()
                        .spacing(12.0)
                        .cross_axis_alignment(Align::Center)
                        .child(CircularProgressIndicator::new().size(20.0))
                        .child(Text::new("Loading comments…").color(theme.muted)),
                ),
            frus::RemoteData::Failure(error) => Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(8.0)
                .child(heading("Comments".into()))
                .child(Text::new(error_line(error)).color(theme.error))
                .child(
                    Button::new("Retry")
                        .variant(Variant::Text)
                        .on_press(self.on_retry.clone()),
                ),
            frus::RemoteData::Success(comments) => comments.iter().fold(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(16.0)
                    .child(heading(format!("Comments ({})", comments.len()))),
                |column, comment| column.child(CommentView::new(comment.clone()).into_widget()),
            ),
        };
        Box::new(section)
    }
}

fn error_line(error: &ApiError) -> String {
    format!("Couldn't load the comments. {}", error.message())
}

/// One comment: who wrote it, then what they wrote.
pub struct CommentView {
    comment: Comment,
}

impl CommentView {
    pub fn new(comment: Comment) -> Self {
        Self { comment }
    }
}

impl StatelessWidget for CommentView {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        Box::new(
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(4.0)
                .child(Text::new(self.comment.name.clone()).weight(FontWeight::Medium))
                .child(
                    Text::new(self.comment.email.to_lowercase())
                        .size(13.0)
                        .color(theme.primary),
                )
                .child(Text::new(self.comment.body.replace('\n', " ")).color(theme.muted)),
        )
    }
}
