use frus::{
    Align, AppBar, Callback, Chip, Column, FontWeight, SingleChildScrollView, State, StateContext,
    StatefulWidget, StatelessWidget, Text, Widget, Wrap,
};

use crate::common::{
    cart::Cart,
    routes::PRODUCT_NAME,
    shell::{section_scaffold, Section},
};
use crate::features::home::{states::homestate::HomeState, widgets::product_card::ProductCard};
use crate::models::product::Category;

/// The front page (`/`): the catalogue, filtered by category.
pub struct HomePage {
    cart: Cart,
}

impl HomePage {
    pub fn new(cart: Cart) -> Self {
        Self { cart }
    }
}

impl StatefulWidget for HomePage {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState::default()
    }
}

impl State for HomeState {
    type Widget = HomePage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let router = cx.router();

        // `None` then each category: one chip per filter.
        let filters = std::iter::once(None)
            .chain(Category::ALL.into_iter().map(Some))
            .fold(Wrap::new().gap(8.0), |row, category| {
                let label = category.map_or("All", Category::label);
                row.child(
                    Chip::new(label)
                        .selected(self.category == category)
                        .on_press(cx.callback(move |s: &mut HomeState| s.show(category))),
                )
            });

        // **`go_named`**: the page names the route and its parameter; the router writes
        // `/products/3`. Because that route sits under `/`, the stack becomes
        // [home, product] and the product page gets a back button.
        let products =
            self.visible()
                .into_iter()
                .fold(Column::new().spacing(8.0), |list, product| {
                    let router = router.clone();
                    let id = product.id.to_string();
                    let open =
                        Callback::new(move || router.go_named(PRODUCT_NAME, &[("id", &id)], &[]));
                    list.child(ProductCard::new(product, open).into_widget())
                });

        let body = SingleChildScrollView::new().padding(16.0).child(
            Column::new()
                .spacing(16.0)
                .cross_axis_alignment(Align::Stretch)
                .child(
                    Text::new("Today's picks")
                        .size(22.0)
                        .weight(FontWeight::Bold),
                )
                .child(filters)
                .child(products),
        );

        let cart = cx.widget().cart.clone();
        section_scaffold(
            cx,
            &cart,
            Section::Shop,
            AppBar::new().title(Text::new("Corner Shop")),
            body,
        )
    }
}
