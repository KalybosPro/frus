//! What the shop sells. Plain data: a fixed catalogue stands in for a server, so the
//! example is about moving between pages and nothing else.

use frus::{IconData, Icons};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    Shoes,
    Clothing,
    Bags,
}

impl Category {
    pub(crate) const ALL: [Category; 3] = [Category::Shoes, Category::Clothing, Category::Bags];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Category::Shoes => "Shoes",
            Category::Clothing => "Clothing",
            Category::Bags => "Bags",
        }
    }

    pub(crate) fn icon(self) -> IconData {
        match self {
            Category::Shoes => Icons::DIRECTIONS_RUN,
            Category::Clothing => Icons::CHECKROOM,
            Category::Bags => Icons::SHOPPING_BAG,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Review {
    pub author: &'static str,
    /// From 1 to 5.
    pub stars: u8,
    pub text: &'static str,
}

/// One article. `Clone` because a page hands it to the next one as the navigation's
/// `extra` (see the size page).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Product {
    pub id: u32,
    pub name: &'static str,
    pub category: Category,
    /// Prices are whole cents: money is never a float.
    pub price_cents: u32,
    pub summary: &'static str,
    pub sizes: &'static [&'static str],
    pub reviews: &'static [Review],
}

impl Product {
    /// The product whose id is `id`.
    pub(crate) fn find(id: u32) -> Option<&'static Product> {
        CATALOG.iter().find(|product| product.id == id)
    }

    /// The product a location names: the `:id` of `/products/:id` is text, and text from
    /// an address can be anything — `/products/abc` must not crash the app.
    pub(crate) fn from_param(id: Option<&str>) -> Option<&'static Product> {
        id?.parse().ok().and_then(Product::find)
    }

    /// The average rating, or `None` when nobody has reviewed it yet.
    pub(crate) fn rating(&self) -> Option<f32> {
        if self.reviews.is_empty() {
            return None;
        }
        let total: u32 = self.reviews.iter().map(|r| u32::from(r.stars)).sum();
        Some(total as f32 / self.reviews.len() as f32)
    }
}

const SHOE_SIZES: &[&str] = &["39", "40", "41", "42", "43", "44"];
const CLOTHING_SIZES: &[&str] = &["XS", "S", "M", "L", "XL"];
const ONE_SIZE: &[&str] = &["One size"];

pub(crate) const CATALOG: &[Product] = &[
    Product {
        id: 1,
        name: "Trail runner",
        category: Category::Shoes,
        price_cents: 8900,
        summary: "A grippy sole and a light upper for muddy paths.",
        sizes: SHOE_SIZES,
        reviews: &[
            Review {
                author: "Ama",
                stars: 5,
                text: "Comfortable from the first run.",
            },
            Review {
                author: "Kofi",
                stars: 4,
                text: "Runs a little small: take the next size up.",
            },
        ],
    },
    Product {
        id: 2,
        name: "City sneakers",
        category: Category::Shoes,
        price_cents: 6500,
        summary: "Plain white leather that goes with everything.",
        sizes: SHOE_SIZES,
        reviews: &[Review {
            author: "Lena",
            stars: 4,
            text: "Smart enough for the office.",
        }],
    },
    Product {
        id: 3,
        name: "Road racer",
        category: Category::Shoes,
        price_cents: 12000,
        summary: "A carbon plate and a soft foam for race day.",
        sizes: SHOE_SIZES,
        reviews: &[
            Review {
                author: "Yao",
                stars: 5,
                text: "Took four minutes off my half marathon.",
            },
            Review {
                author: "Mia",
                stars: 3,
                text: "Fast, but they wear out quickly.",
            },
        ],
    },
    Product {
        id: 4,
        name: "Linen shirt",
        category: Category::Clothing,
        price_cents: 4200,
        summary: "Breathes in the heat, creases with dignity.",
        sizes: CLOTHING_SIZES,
        reviews: &[],
    },
    Product {
        id: 5,
        name: "Rain jacket",
        category: Category::Clothing,
        price_cents: 11900,
        summary: "Taped seams and a hood that stays put in the wind.",
        sizes: CLOTHING_SIZES,
        reviews: &[Review {
            author: "Sam",
            stars: 5,
            text: "Dry after a whole afternoon of rain.",
        }],
    },
    Product {
        id: 6,
        name: "Canvas tote",
        category: Category::Bags,
        price_cents: 2400,
        summary: "Holds a week of groceries, folds into a pocket.",
        sizes: ONE_SIZE,
        reviews: &[],
    },
    Product {
        id: 7,
        name: "Leather backpack",
        category: Category::Bags,
        price_cents: 14500,
        summary: "A padded sleeve for a laptop and room for a day out.",
        sizes: ONE_SIZE,
        reviews: &[Review {
            author: "Noor",
            stars: 4,
            text: "Beautiful, if a little heavy.",
        }],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_product_is_found_by_its_id() {
        assert_eq!(Product::find(3).map(|p| p.name), Some("Road racer"));
        assert!(Product::find(99).is_none());
    }

    #[test]
    fn a_location_parameter_that_is_not_an_id_finds_nothing() {
        assert_eq!(Product::from_param(Some("2")).map(|p| p.id), Some(2));
        assert!(Product::from_param(Some("abc")).is_none());
        assert!(Product::from_param(Some("-1")).is_none());
        assert!(Product::from_param(None).is_none());
    }

    #[test]
    fn the_rating_is_the_average_of_the_reviews() {
        assert_eq!(Product::find(1).and_then(Product::rating), Some(4.5));
        assert_eq!(Product::find(4).and_then(Product::rating), None);
    }

    #[test]
    fn ids_are_unique() {
        for (i, a) in CATALOG.iter().enumerate() {
            assert!(CATALOG[i + 1..].iter().all(|b| b.id != a.id), "{}", a.id);
        }
    }
}
