use frus::{BuildContext, Column, Icons, Scaffold, SingleChildScrollView, StatelessWidget, Widget};

use crate::common::shared::{empty_state, page_app_bar};
use crate::features::product::widgets::review_tile::ReviewTile;
use crate::models::product::Product;

/// A product's reviews (`/products/:id/reviews`), a route nested two levels deep. It
/// remembers nothing, so it is a `StatelessWidget`: the `:id` is all it needs.
pub(crate) struct ReviewsPage {
    id: Option<String>,
}

impl ReviewsPage {
    pub(crate) fn new(id: Option<&str>) -> Self {
        Self {
            id: id.map(str::to_string),
        }
    }
}

impl StatelessWidget for ReviewsPage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let product = Product::from_param(self.id.as_deref());
        let title = product.map_or("Reviews".to_string(), |p| format!("Reviews of {}", p.name));

        let body: Box<dyn Widget> = match product {
            Some(product) if !product.reviews.is_empty() => {
                Box::new(SingleChildScrollView::new().padding(8.0).child(
                    product.reviews.iter().fold(Column::new(), |list, review| {
                        list.child(ReviewTile::new(*review).into_widget())
                    }),
                ))
            }
            Some(_) => Box::new(empty_state(cx, Icons::RATE_REVIEW, "No reviews yet", None)),
            None => Box::new(empty_state(cx, Icons::SEARCH_OFF, "Unknown product", None)),
        };

        Scaffold::new()
            .background(cx.theme().background)
            .app_bar(page_app_bar(cx, title).build())
            .body(body)
            .build()
    }
}
