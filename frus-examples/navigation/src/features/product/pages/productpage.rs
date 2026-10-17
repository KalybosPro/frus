use frus::{
    Align, Button, Callback, Card, Column, FontWeight, Icon, IconButton, Icons, ListTile, Row,
    Scaffold, SingleChildScrollView, State, StateContext, StatefulWidget, StatelessWidget, Text,
    Variant, Widget,
};

use crate::common::{
    cart::Cart,
    colors::price,
    routes::{CART, HOME, PRODUCT_NAME, REVIEWS_NAME, SIZE_NAME},
    shared::{empty_state, format_price, page_app_bar},
};
use crate::features::product::{
    states::productstate::ProductState, widgets::quantity_stepper::QuantityStepper,
};
use crate::models::product::{Product, CATALOG};

/// A product (`/products/:id`). The router hands it the `:id` exactly as the address
/// wrote it; turning that into a product — or into "no such product" — is the page's job.
pub struct ProductPage {
    cart: Cart,
    id: Option<String>,
}

impl ProductPage {
    pub fn new(cart: Cart, id: Option<&str>) -> Self {
        Self {
            cart,
            id: id.map(str::to_string),
        }
    }
}

impl StatefulWidget for ProductPage {
    type State = ProductState;

    fn create_state(&self) -> ProductState {
        ProductState::new()
    }
}

/// The product after `product` in the catalogue, wrapping round.
pub fn next_after(product: &Product) -> &'static Product {
    let at = CATALOG.iter().position(|p| p.id == product.id).unwrap_or(0);
    &CATALOG[(at + 1) % CATALOG.len()]
}

impl State for ProductState {
    type Widget = ProductPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let page = cx.widget();
        let router = cx.router();
        let theme = cx.theme();

        // The route matched (`/products/:id` answers any id), but the id names nothing:
        // the page says so. Only a location no *route* answers gets the router's
        // not-found page.
        let Some(product) = Product::from_param(page.id.as_deref()) else {
            let home = Button::new("Back to the shop").on_press(move || router.go(HOME));
            return Scaffold::new()
                .background(theme.background)
                .app_bar(page_app_bar(cx, "Unknown product").build())
                .body(empty_state(
                    cx,
                    Icons::SEARCH_OFF,
                    format!("There is no product {}", page.id.as_deref().unwrap_or("")),
                    Some(home),
                ))
                .build();
        };
        let id = product.id.to_string();
        let size = page.cart.chosen_size(product.id);

        // **`replace`**: the next product takes this one's place on the stack, so the
        // back button skips every product browsed this way and goes straight back to
        // where browsing started.
        let next = {
            let router = router.clone();
            let next_id = next_after(product).id.to_string();
            Callback::new(move || {
                if let Some(location) = router.location_for(PRODUCT_NAME, &[("id", &next_id)], &[])
                {
                    router.replace(location);
                }
            })
        };

        // **`push_with_extra`**: the size page receives the product itself and reads it
        // with `state.extra::<Product>()`. The page answers by writing the chosen size
        // into the cart and popping; this page reads it back from there.
        let pick_size = {
            let router = router.clone();
            let (id, product) = (id.clone(), product.clone());
            Callback::new(move || {
                if let Some(location) = router.location_for(SIZE_NAME, &[("id", &id)], &[]) {
                    router.push_with_extra(location, product.clone());
                }
            })
        };

        // **`push_named`**: reviews on top of the product, back returns to it.
        let reviews = {
            let router = router.clone();
            let id = id.clone();
            Callback::new(move || router.push_named(REVIEWS_NAME, &[("id", &id)], &[]))
        };

        let add = {
            let (cart, size) = (page.cart.clone(), size.clone());
            cx.callback(move |s: &mut ProductState| {
                if let Some(size) = &size {
                    cart.add(product, size, s.quantity);
                    s.added();
                }
            })
        };

        let options = Card::new().outlined().child(
            Column::new()
                .child(
                    ListTile::new()
                        .leading(Icon::new(Icons::STRAIGHTEN))
                        .title("Size")
                        .subtitle(size.clone().unwrap_or_else(|| "Choose a size".to_string()))
                        .trailing(Icon::new(Icons::CHEVRON_RIGHT).color(theme.muted))
                        .on_tap(pick_size),
                )
                .child(
                    ListTile::new()
                        .leading(Icon::new(Icons::RATE_REVIEW))
                        .title(format!("Reviews ({})", product.reviews.len()))
                        .subtitle(match product.rating() {
                            Some(stars) => format!("{stars:.1} out of 5"),
                            None => "No reviews yet".to_string(),
                        })
                        .trailing(Icon::new(Icons::CHEVRON_RIGHT).color(theme.muted))
                        .on_tap(reviews),
                ),
        );

        let buy = Row::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Center)
            .child(
                QuantityStepper::new(
                    self.quantity,
                    cx.callback(ProductState::fewer),
                    cx.callback(ProductState::more),
                )
                .into_widget(),
            )
            .child(
                Button::new(if size.is_some() {
                    "Add to cart"
                } else {
                    "Choose a size first"
                })
                .enabled(size.is_some())
                .on_press(add),
            );

        let mut content = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Stretch)
            .child(Text::new(product.name).size(26.0).weight(FontWeight::Bold))
            .child(
                Text::new(format_price(product.price_cents))
                    .size(20.0)
                    .weight(FontWeight::SemiBold)
                    .color(price()),
            )
            .child(Text::new(product.summary).size(15.0))
            .child(options)
            .child(buy);
        if self.added {
            // `go`: the cart is a section, so the stack becomes just the cart.
            let router = router.clone();
            content = content.child(
                Button::new("Added — view the cart")
                    .variant(Variant::Tonal)
                    .on_press(move || router.go(CART)),
            );
        }

        Scaffold::new()
            .background(theme.background)
            .app_bar(
                page_app_bar(cx, product.category.label())
                    .action(
                        IconButton::new(Icons::ARROW_FORWARD)
                            .label("Next product")
                            .on_press(next),
                    )
                    .build(),
            )
            .body(SingleChildScrollView::new().padding(16.0).child(content))
            .build()
    }
}
