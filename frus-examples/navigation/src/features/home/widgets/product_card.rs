use frus::{BuildContext, Callback, Card, Icon, ListTile, StatelessWidget, Text, Widget};

use crate::common::{colors::price, shared::format_price};
use crate::models::product::Product;

/// One product in a list: what it is, what it costs, and what happens on a tap — which
/// the **parent** decides. The front page opens the product with `go`, the search page
/// with `push`; the card does not know or care.
pub(crate) struct ProductCard {
    product: &'static Product,
    on_tap: Callback,
}

impl ProductCard {
    pub(crate) fn new(product: &'static Product, on_tap: Callback) -> Self {
        Self { product, on_tap }
    }
}

impl StatelessWidget for ProductCard {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let product = self.product;
        Box::new(
            Card::new().outlined().child(
                ListTile::new()
                    .leading(Icon::new(product.category.icon()).color(cx.theme().primary))
                    .title(product.name)
                    .subtitle(product.summary)
                    .trailing(
                        Text::new(format_price(product.price_cents))
                            .color(price())
                            .size(15.0),
                    )
                    .on_tap(self.on_tap.clone()),
            ),
        )
    }
}
