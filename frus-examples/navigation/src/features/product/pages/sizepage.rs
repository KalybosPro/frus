use frus::{
    BuildContext, Callback, Column, Icon, Icons, ListTile, Scaffold, SingleChildScrollView,
    StatelessWidget, Widget,
};

use crate::common::{
    cart::Cart,
    shared::{empty_state, page_app_bar},
};
use crate::models::product::Product;

/// "Pick a size" (`/products/:id/size`): a page that **returns a result**.
///
/// The product page pushes it; a tap writes the size into the shared cart and pops. The
/// product page, underneath all along, reads the size back from the cart when it is
/// shown again. The result travels through shared state, not through the router.
pub(crate) struct SizePage {
    cart: Cart,
    product: Option<Product>,
}

impl SizePage {
    /// `product` is the navigation's `extra` when there was one, or the product the
    /// address names.
    pub(crate) fn new(cart: Cart, product: Option<Product>) -> Self {
        Self { cart, product }
    }
}

impl StatelessWidget for SizePage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let router = cx.router();
        let body: Box<dyn Widget> = match &self.product {
            None => Box::new(empty_state(cx, Icons::SEARCH_OFF, "Unknown product", None)),
            Some(product) => {
                let chosen = self.cart.chosen_size(product.id);
                let list = product.sizes.iter().fold(Column::new(), |list, size| {
                    let (cart, router, id) = (self.cart.clone(), router.clone(), product.id);
                    let pick = Callback::new(move || {
                        cart.choose_size(id, size);
                        router.pop();
                    });
                    let mut tile = ListTile::new()
                        .title(*size)
                        .selected(chosen.as_deref() == Some(*size))
                        .on_tap(pick);
                    if chosen.as_deref() == Some(*size) {
                        tile = tile.trailing(Icon::new(Icons::CHECK));
                    }
                    list.child(tile)
                });
                Box::new(SingleChildScrollView::new().padding(8.0).child(list))
            }
        };

        let title = self
            .product
            .as_ref()
            .map_or("Pick a size".to_string(), |p| format!("Size of {}", p.name));
        Scaffold::new()
            .background(cx.theme().background)
            .app_bar(page_app_bar(cx, title).build())
            .body(body)
            .build()
    }
}
