use frus::{
    Align, AppBar, Column, Divider, FontWeight, RemoteData, Scaffold, SingleChildScrollView, State,
    StateContext, StatefulWidget, StatelessWidget, Text, Widget,
};

use crate::common::api::{self, ApiError};
use crate::common::remote::use_api;
use crate::common::shared::{back_button, ErrorView, LoadingView};
use crate::features::post::states::poststate::PostState;
use crate::features::post::widgets::{author_card::author_card, comments::CommentsSection};
use crate::features::posts::widgets::post_tile::capitalize;

/// One post, its author and its comments: `/posts/:id`.
///
/// The configuration is the id read from the address — `None` when the address held
/// something that is not a number.
pub struct PostPage {
    id: Option<u32>,
}

impl PostPage {
    pub fn new(id: Option<u32>) -> Self {
        Self { id }
    }
}

impl StatefulWidget for PostPage {
    type State = PostState;

    fn create_state(&self) -> PostState {
        PostState::new()
    }
}

impl State for PostState {
    type Widget = PostPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let id = cx.widget().id;
        let key = (id, self.attempt);

        // **Two requests in parallel.** Each hook starts its own future the first time it
        // is built, and both are built here, one after the other, before either has an
        // answer: the post and its comments travel at the same time, and each part of the
        // page shows up when its own answer does.
        //
        // An id that is not a number never reaches the network: it is answered as the
        // server would answer an unknown id.
        let post = use_api(cx, key, move || async move {
            match id {
                Some(id) => api::post(id).await,
                None => Err(ApiError::NotFound),
            }
        });
        let comments = use_api(cx, key, move || async move {
            match id {
                Some(id) => api::comments(id).await,
                None => Err(ApiError::NotFound),
            }
        });

        let retry = cx.callback(PostState::retry);
        let body: Box<dyn Widget> = match post.data {
            RemoteData::NotAsked | RemoteData::Loading => {
                Box::new(LoadingView::new("Loading the post…").into_widget())
            }
            RemoteData::Failure(error) => Box::new(ErrorView::new(error, retry).into_widget()),
            RemoteData::Success(post) => {
                let theme = cx.theme();
                let content = Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(16.0)
                    .child(
                        Text::new(capitalize(&post.title))
                            .size(24.0)
                            .weight(FontWeight::SemiBold),
                    )
                    // The third request, which needs the post's `userId`: it can only
                    // start now, so it lives in a part of the page that is only built
                    // now.
                    .child(author_card(post.user_id, self.attempt))
                    .child(Text::new(capitalize(&post.body.replace('\n', " "))).size(16.0))
                    .child(
                        Text::new(format!("Post #{}", post.id))
                            .size(13.0)
                            .color(theme.muted),
                    )
                    .child(Divider::new())
                    .child(CommentsSection::new(comments, retry).into_widget());
                Box::new(
                    SingleChildScrollView::new()
                        .padding_each(16.0, 16.0, 32.0, 16.0)
                        .child(content),
                )
            }
        };

        let app_bar = AppBar::new()
            .leading(back_button(cx))
            .title(Text::new("Post"))
            .build();

        Scaffold::new()
            .background(cx.theme().background)
            .app_bar(app_bar)
            .body(body)
            .build()
    }
}
