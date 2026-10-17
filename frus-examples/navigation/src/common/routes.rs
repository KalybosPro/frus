//! Every place in the app, in one file: a page never writes a path by hand.
//!
//! Two kinds of constant:
//!
//! - **paths**, the patterns the router matches. A top-level path starts with `/`; a
//!   sub-route's path is relative to its parent's (`products/:id` under `/` is
//!   `/products/:id`), and `:id` is a **parameter** the page reads with
//!   `state.param("id")`;
//! - **names**, for the routes a page reaches with `go_named` / `push_named`: the
//!   router fills in the parameters and encodes the query, so a page says *which* route
//!   and *with what*, and the path can change without touching any page.

// ---- top-level sections (the bottom bar / the rail) --------------------------------

/// The shop's front page.
pub const HOME: &str = "/";
/// Search, with its query in the address: `/search?q=shoes&sort=price`.
pub const SEARCH: &str = "/search";
pub const CART: &str = "/cart";

// ---- other top-level pages ---------------------------------------------------------

/// Guarded: the router sends it to [`CART`] while the cart is empty.
pub const CHECKOUT: &str = "/checkout";
pub const ORDER_PLACED: &str = "/order-placed";

// ---- under HOME: `/products/:id`, `/products/:id/reviews`, `/products/:id/size` ----

pub const PRODUCT: &str = "products/:id";
pub const REVIEWS: &str = "reviews";
pub const SIZE: &str = "size";

// ---- names --------------------------------------------------------------------------

pub const SEARCH_NAME: &str = "search";
pub const PRODUCT_NAME: &str = "product";
pub const REVIEWS_NAME: &str = "reviews";
pub const SIZE_NAME: &str = "size";

// ---- query parameters ---------------------------------------------------------------

/// `?q=` on [`SEARCH`]: what was typed.
pub const QUERY: &str = "q";
/// `?sort=` on [`SEARCH`].
pub const SORT: &str = "sort";
