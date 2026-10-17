use frus::{
    AppBar, BuildContext, Callback, Column, Icon, Icons, ListTile, Scaffold, SingleChildScrollView,
    StatelessWidget, Text, Widget,
};

use crate::common::{catalog::categories, routes::category_location};

/// The gallery's first page: one row per category of widgets.
pub struct HomePage;

impl StatelessWidget for HomePage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        // The router is a cheap handle: each row's handler keeps its own copy.
        let router = cx.router();

        let list = categories()
            .into_iter()
            .fold(Column::new(), |list, category| {
                let count = (category.demos)().len();
                let router = router.clone();
                let location = category_location(category.id);
                list.child(
                    ListTile::new()
                        .leading(Icon::new(category.icon).color(theme.primary))
                        .title(category.name)
                        .subtitle(format!("{} · {count} widgets", category.summary))
                        .trailing(Icon::new(Icons::CHEVRON_RIGHT).color(theme.muted))
                        .on_tap(Callback::new(move || router.push(&location))),
                )
            });

        Scaffold::new()
            .background(theme.background)
            .app_bar(AppBar::new().title(Text::new("frus widgets")).build())
            .body(
                SingleChildScrollView::new()
                    .padding_each(8.0, 0.0, 24.0, 0.0)
                    .child(list),
            )
            .build()
    }
}
