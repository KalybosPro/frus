use frus::{
    AppBar, BuildContext, Callback, Center, Column, Icon, Icons, ListTile, Scaffold,
    SingleChildScrollView, StatelessWidget, Text, Widget,
};

use crate::common::{catalog::category, routes::demo_location};

/// One category: the widgets it holds, each opening its demo.
pub(crate) struct CategoryPage {
    id: String,
}

impl CategoryPage {
    pub(crate) fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl StatelessWidget for CategoryPage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let Some(category) = category(&self.id) else {
            return Box::new(Center::new(Text::new(format!("No category `{}`", self.id))));
        };

        let router = cx.router();
        let list = (category.demos)()
            .into_iter()
            .fold(Column::new(), |list, demo| {
                let router = router.clone();
                let location = demo_location(demo.id);
                list.child(
                    ListTile::new()
                        .title(demo.name)
                        .subtitle(demo.summary)
                        .trailing(Icon::new(Icons::CHEVRON_RIGHT).color(theme.muted))
                        .on_tap(Callback::new(move || router.push(&location))),
                )
            });

        Scaffold::new()
            .background(theme.background)
            .app_bar(AppBar::new().title(Text::new(category.name)).build())
            .body(
                SingleChildScrollView::new()
                    .padding_each(8.0, 0.0, 24.0, 0.0)
                    .child(list),
            )
            .build()
    }
}
