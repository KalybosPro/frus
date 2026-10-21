use frus::{
    Align, Callback, Colors, Column, Scaffold, SingleChildScrollView, SizedBox, State,
    StateContext, StatefulWidget, StatelessWidget, Widget,
};

use crate::{
    common::auth::Auth,
    features::home::pages::{
        states::home_state::HomeState,
        widgets::{
            greetings::Greetings, homeappbar::HomeAppBar,
            recent_project_widget::RecentProjectWidget, stats_grid::StatsGrid,
        },
    },
};

pub(crate) struct HomePage {
    auth: Auth,
}

impl HomePage {
    pub(crate) fn new(auth: Auth) -> Self {
        Self { auth }
    }
}

impl StatefulWidget for HomePage {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState {}
    }
}

impl State for HomeState {
    type Widget = HomePage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let auth = cx.widget().auth.clone();
        let logout = Callback::new(move || auth.logout());

        let header = HomeAppBar.into_widget();

        let view = Column::new()
            .spacing(24.0)
            .cross_axis_alignment(Align::Start)
            .child(Greetings.into_widget())
            .child(StatsGrid.into_widget())
            .child(RecentProjectWidget.into_widget());

        let content = Column::new()
            .child(SizedBox::empty().height(24.0))
            .child(view);

        let page = SingleChildScrollView::new()
            .padding_each(0.0, 20.0, 50.0, 20.0)
            .child(content);

        Scaffold::new()
            .app_bar(header)
            .background(Colors::WHITE)
            .body(page)
            .build()
    }
}
