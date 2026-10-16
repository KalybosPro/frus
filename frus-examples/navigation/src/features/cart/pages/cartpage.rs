use frus::{
    Align, AppBar, Button, Callback, Column, FontWeight, Icons, Justify, Row,
    SingleChildScrollView, State, StateContext, StatefulWidget, StatelessWidget, Text, Variant,
    Widget,
};

use crate::common::{
    cart::Cart,
    routes::{CHECKOUT, HOME},
    shared::{empty_state, format_price},
    shell::{section_scaffold, Section},
};
use crate::features::cart::{states::cartstate::CartState, widgets::cart_line::CartLine};

/// The cart (`/cart`), a section.
pub(crate) struct CartPage {
    cart: Cart,
}

impl CartPage {
    pub(crate) fn new(cart: Cart) -> Self {
        Self { cart }
    }
}

impl StatefulWidget for CartPage {
    type State = CartState;

    fn create_state(&self) -> CartState {
        CartState::default()
    }
}

impl State for CartState {
    type Widget = CartPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let page = cx.widget();
        let cart = page.cart.clone();
        let router = cx.router();

        let mut content = Column::new()
            .spacing(12.0)
            .cross_axis_alignment(Align::Stretch);

        if let Some(removed) = &self.removed {
            let undo = {
                let cart = cart.clone();
                cx.callback(move |s: &mut CartState| {
                    if let Some(item) = s.take_removed() {
                        cart.restore(item);
                    }
                })
            };
            content = content.child(
                Row::new()
                    .main_axis_alignment(Justify::SpaceBetween)
                    .cross_axis_alignment(Align::Center)
                    .child(Text::new(format!("Removed {}", removed.name)))
                    .child(Button::new("Undo").variant(Variant::Text).on_press(undo)),
            );
        }

        let items = cart.items();
        if items.is_empty() {
            let router = router.clone();
            let browse = Button::new("Browse the shop").on_press(move || router.go(HOME));
            content = content.child(empty_state(
                cx,
                Icons::SHOPPING_CART,
                "Your cart is empty",
                Some(browse),
            ));
        } else {
            for item in items {
                let (id, size, quantity) = (item.product_id, item.size.clone(), item.quantity);
                let set = |quantity: u32| {
                    let (cart, size) = (cart.clone(), size.clone());
                    Callback::new(move || cart.set_quantity(id, &size, quantity))
                };
                let remove = {
                    let (cart, size) = (cart.clone(), size.clone());
                    cx.callback(move |s: &mut CartState| s.remember(cart.remove(id, &size)))
                };
                content = content.child(
                    CartLine::new(
                        item,
                        set(quantity.saturating_sub(1).max(1)),
                        set(quantity + 1),
                        remove,
                    )
                    .into_widget(),
                );
            }

            // **`push`**: the checkout goes on top of the cart, so its back button
            // returns here. The guard in `routers.rs` decides whether it may open at all.
            let checkout = {
                let router = router.clone();
                Callback::new(move || router.push(CHECKOUT))
            };
            content = content
                .child(
                    Row::new()
                        .main_axis_alignment(Justify::SpaceBetween)
                        .child(Text::new("Subtotal").weight(FontWeight::SemiBold))
                        .child(
                            Text::new(format_price(cart.subtotal_cents()))
                                .weight(FontWeight::SemiBold),
                        ),
                )
                .child(Button::new("Checkout").on_press(checkout));
        }

        section_scaffold(
            cx,
            &cart,
            Section::Cart,
            AppBar::new().title(Text::new("Cart")),
            SingleChildScrollView::new().padding(16.0).child(content),
        )
    }
}
