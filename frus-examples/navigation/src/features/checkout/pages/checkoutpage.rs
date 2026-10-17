use frus::{
    Align, Button, Column, Icons, ListTile, Scaffold, SingleChildScrollView, State, StateContext,
    StatefulWidget, StatelessWidget, Switch, Variant, Widget,
};

use crate::common::{
    cart::Cart,
    routes::ORDER_PLACED,
    shared::{empty_state, page_app_bar},
};
use crate::features::checkout::{
    states::checkoutstate::CheckoutState, widgets::order_summary::OrderSummary,
};
use crate::models::order::Order;

/// The checkout (`/checkout`). It does not have to check for an empty cart: the router's
/// redirect lets nobody in with one, and sends them back to the cart if it empties while
/// they are here.
pub struct CheckoutPage {
    cart: Cart,
}

impl CheckoutPage {
    pub fn new(cart: Cart) -> Self {
        Self { cart }
    }
}

impl StatefulWidget for CheckoutPage {
    type State = CheckoutState;

    fn create_state(&self) -> CheckoutState {
        CheckoutState::default()
    }
}

impl State for CheckoutState {
    type Widget = CheckoutPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let cart = cx.widget().cart.clone();
        let router = cx.router();
        let theme = cx.theme();

        let subtotal = cart.subtotal_cents();
        let shipping = self.shipping_cents(subtotal);
        let total = self.total_cents(subtotal);
        let items = cart.items();

        // Seen at most for the moment between the cart emptying and the redirect moving
        // the page away.
        if items.is_empty() {
            return Scaffold::new()
                .background(theme.background)
                .body(empty_state(
                    cx,
                    Icons::SHOPPING_CART,
                    "Your cart is empty",
                    None,
                ))
                .build();
        }

        // **`go_with_extra`**: the confirmation is handed the order it confirms. Then the
        // cart is emptied: the router hears it, asks the guard about the confirmation
        // page, and leaves it where it is.
        let place = {
            let (cart, router, items) = (cart.clone(), router.clone(), items.clone());
            move || {
                let order = Order {
                    items: items.clone(),
                    shipping_cents: shipping,
                    total_cents: total,
                };
                router.go_with_extra(ORDER_PLACED, order);
                cart.clear();
            }
        };

        // Emptying the cart from here shows the **refresh listenable** at work: nobody
        // navigates, yet the guard runs again and the page moves to the cart.
        let empty = {
            let cart = cart.clone();
            move || cart.clear()
        };

        let content = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Stretch)
            .child(OrderSummary::new(items, shipping, total).into_widget())
            .child(
                ListTile::new()
                    .title("Express delivery")
                    .subtitle("Tomorrow, for a little more")
                    .trailing(
                        Switch::new(self.express).on_toggle(cx.handler(CheckoutState::set_express)),
                    ),
            )
            .child(Button::new("Place order").on_press(place))
            .child(
                Button::new("Empty the cart")
                    .variant(Variant::Text)
                    .on_press(empty),
            );

        Scaffold::new()
            .background(theme.background)
            .app_bar(page_app_bar(cx, "Checkout").build())
            .body(SingleChildScrollView::new().padding(16.0).child(content))
            .build()
    }
}
