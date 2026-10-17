//! The router: which page each location builds, the page shown for a location no route
//! answers, and the guard that keeps an empty cart out of the checkout.

use frus::{GoRoute, GoRouter, StatefulWidget, StatelessWidget};

use crate::common::cart::Cart;
use crate::common::routes::*;
use crate::features::cart::pages::cartpage::CartPage;
use crate::features::checkout::pages::checkoutpage::CheckoutPage;
use crate::features::checkout::pages::orderplacedpage::OrderPlacedPage;
use crate::features::home::pages::homepage::HomePage;
use crate::features::home::pages::notfoundpage::NotFoundPage;
use crate::features::product::pages::productpage::ProductPage;
use crate::features::product::pages::reviewspage::ReviewsPage;
use crate::features::product::pages::sizepage::SizePage;
use crate::features::search::pages::searchpage::SearchPage;
use crate::features::search::states::searchstate::Sort;
use crate::models::order::Order;
use crate::models::product::Product;

/// Every page of the shop, by location.
///
/// A route's builder is told the `GoRouterState` — what the location said — and hands the
/// page what it needs: the path parameters, the query, the object that travelled with the
/// navigation, and the shared cart.
pub fn router(cart: Cart) -> GoRouter {
    // Each builder owns its own handle to the cart: they are `Fn` closures, called every
    // time their page is built.
    let (home, search, product, size, cart_page, checkout, guard) = (
        cart.clone(),
        cart.clone(),
        cart.clone(),
        cart.clone(),
        cart.clone(),
        cart.clone(),
        cart.clone(),
    );

    GoRouter::new(vec![
        // `/` and the routes **under** it. `go("/products/3")` makes the stack
        // [home, product 3], so the product page has somewhere to go back to — even when
        // the app was opened at that address.
        GoRoute::new(HOME, move |_, _| HomePage::new(home.clone()).into_widget()).routes(vec![
            GoRoute::new(PRODUCT, move |_, state| {
                ProductPage::new(product.clone(), state.param("id")).into_widget()
            })
            .name(PRODUCT_NAME)
            .routes(vec![
                GoRoute::new(REVIEWS, |_, state| {
                    ReviewsPage::new(state.param("id")).into_widget()
                })
                .name(REVIEWS_NAME),
                // The product page pushes this one **with the product as `extra`**, so it
                // does not look it up again. `extra` lives only in memory: opened from an
                // address (or after a reload on the web) there is none, and the page falls
                // back to the `:id` in the path.
                GoRoute::new(SIZE, move |_, state| {
                    let product = state
                        .extra::<Product>()
                        .map(|product| (*product).clone())
                        .or_else(|| Product::from_param(state.param("id")).cloned());
                    SizePage::new(size.clone(), product).into_widget()
                })
                .name(SIZE_NAME),
            ]),
        ]),
        // The query is read here, so the page is *configured* by the address: change the
        // address and the page shows the new search.
        GoRoute::new(SEARCH, move |_, state| {
            SearchPage::new(
                search.clone(),
                state.query(QUERY).unwrap_or_default(),
                Sort::from_query(state.query(SORT)),
            )
            .into_widget()
        })
        .name(SEARCH_NAME),
        GoRoute::new(CART, move |_, _| {
            CartPage::new(cart_page.clone()).into_widget()
        }),
        GoRoute::new(CHECKOUT, move |_, _| {
            CheckoutPage::new(checkout.clone()).into_widget()
        }),
        GoRoute::new(ORDER_PLACED, |_, state| {
            OrderPlacedPage::new(state.extra::<Order>()).into_widget()
        }),
    ])
    .initial_location(start_location())
    // A location no route answers — a mistyped address, an old link — builds this page
    // instead of failing.
    .error_builder(|_, state| NotFoundPage::new(state.location()).into_widget())
    // Asked about **every** navigation, before any page is built.
    .redirect(move |state| guard_redirect(guard.is_empty(), state.location()))
    // …and again whenever the cart becomes empty, with nobody navigating.
    .refresh_listenable(cart.emptied())
}

/// Where the app opens.
///
/// On the desktop a location given on the command line opens the app there —
/// `cargo run -- /products/3`, `cargo run -- "/search?q=shoes"` — which is handy while
/// working on one page. On the web the address bar does this on its own.
pub fn start_location() -> String {
    std::env::args()
        .nth(1)
        .filter(|location| location.starts_with('/'))
        .unwrap_or_else(|| HOME.to_string())
}

/// The checkout guard, as a **plain function**: what it decides depends only on its
/// arguments, so it is tested in `tests/routers_test.rs` without a router or a window.
///
/// `None` lets the navigation through; `Some(location)` sends it there instead.
pub fn guard_redirect(cart_is_empty: bool, location: &str) -> Option<String> {
    // The location includes the query (`/checkout?from=drawer`): compare the path only.
    let path = location.split(['?', '#']).next().unwrap_or(location);
    (cart_is_empty && path == CHECKOUT).then(|| CART.to_string())
}
