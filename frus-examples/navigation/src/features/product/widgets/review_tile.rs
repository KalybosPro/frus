use frus::{BuildContext, Icon, Icons, ListTile, Row, StatelessWidget, Widget};

use crate::common::colors::rating;
use crate::models::product::Review;

/// One review: the stars, who wrote it, what they said.
pub struct ReviewTile {
    review: Review,
}

impl ReviewTile {
    pub fn new(review: Review) -> Self {
        Self { review }
    }
}

impl StatelessWidget for ReviewTile {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let stars = (1..=5).fold(Row::new(), |row, n| {
            let icon = if n <= self.review.stars {
                Icons::STAR
            } else {
                Icons::STAR_BORDER
            };
            row.child(Icon::new(icon).size(16.0).color(rating()))
        });
        Box::new(
            ListTile::new()
                .title(self.review.author)
                .subtitle(self.review.text)
                .trailing(stars),
        )
    }
}
