use std::rc::Rc;

use frus::{
    AppBar, Callback, Column, Expanded, FloatingActionButton, IconButton, Icons,
    LinearProgressIndicator, ListView, RefreshIndicator, RemoteData, Scaffold, SizedBox, State,
    StateContext, StatefulWidget, StatelessWidget, Text, Widget,
};

use crate::common::api;
use crate::common::remote::use_api;
use crate::common::routes::{new_post_location, post_location};
use crate::common::shared::{ErrorView, LoadingView};
use crate::features::posts::states::postsstate::PostsState;
use crate::features::posts::widgets::post_tile::{post_tile, POST_TILE_HEIGHT};
use crate::models::post::Post;

/// The first page: every post, from `GET /posts`.
pub(crate) struct PostsPage;

impl StatefulWidget for PostsPage {
    type State = PostsState;

    fn create_state(&self) -> PostsState {
        PostsState::new()
    }
}

impl State for PostsState {
    type Widget = PostsPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The request, started by the build that needs it. It runs once per
        // `generation`: rebuilding for any other reason — a hover, a resize — does not
        // send it again. `api::posts` is an `async fn`; the hook takes the function that
        // makes the future, and runs that future on the platform's executor.
        let posts = use_api(cx, self.generation, api::posts);

        let body: Box<dyn Widget> = match posts.data {
            RemoteData::NotAsked | RemoteData::Loading => {
                Box::new(LoadingView::new("Loading posts…").into_widget())
            }
            RemoteData::Failure(error) => {
                Box::new(ErrorView::new(error, cx.callback(PostsState::reload)).into_widget())
            }
            RemoteData::Success(list) => self.list(cx, list, posts.reloading),
        };

        let app_bar = AppBar::new()
            .title(Text::new("Posts"))
            .action(
                IconButton::new(Icons::REFRESH)
                    .label("Refresh")
                    .on_press(cx.callback(PostsState::reload)),
            )
            .build();

        Scaffold::new()
            .background(cx.theme().background)
            .app_bar(app_bar)
            .body(body)
            .fab(
                FloatingActionButton::extended("New post")
                    .icon(Icons::ADD)
                    .on_press(go_to(cx, new_post_location())),
            )
            .build()
    }
}

impl PostsState {
    /// The loaded list: pull it down to reload it.
    fn list(&self, cx: &StateContext<Self>, posts: Vec<Post>, reloading: bool) -> Box<dyn Widget> {
        // The list builds its rows on demand, from a closure that outlives this `build`,
        // so it owns what it reads: the posts behind an `Rc`, and the router.
        let posts = Rc::new(posts);
        let router = cx.router();
        let muted = cx.theme().muted;
        let rows = ListView::new(posts.len(), POST_TILE_HEIGHT, move |index| {
            let post = &posts[index];
            let router = router.clone();
            let open = post_location(post.id);
            // `push`, not `go`: the detail page goes on top of the list, and back
            // returns to it where it was scrolled.
            post_tile(post, muted, Callback::new(move || router.push(&open)))
        });

        // The framework reports that the user pulled; the page decides when the answer
        // has arrived. The indicator spins for exactly as long as `refreshing` is true —
        // here, until the new list replaces the old one.
        let pullable = RefreshIndicator::new(rows)
            .on_refresh(cx.callback(PostsState::pull_to_refresh))
            .refreshing(reloading && self.pulled);

        // A reload from the app bar has no pull to spin, so it gets a bar instead. The
        // same height either way, so the list does not jump when it comes and goes.
        let progress: Box<dyn Widget> = if reloading && !self.pulled {
            Box::new(LinearProgressIndicator::indeterminate())
        } else {
            Box::new(SizedBox::empty().height(4.0))
        };

        Box::new(Column::new().child(progress).child(Expanded::new(pullable)))
    }
}

/// A handler that opens `location` on top of the current page.
fn go_to(cx: &StateContext<PostsState>, location: String) -> Callback {
    let router = cx.router();
    Callback::new(move || router.push(&location))
}
