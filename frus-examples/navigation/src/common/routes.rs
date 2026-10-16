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
pub(crate) const HOME: &str = "/";
/// Search, with its query in the address: `/search?q=shoes&sort=price`.
pub(crate) const SEARCH: &str = "/search";
pub(crate) const CART: &str = "/cart";

// ---- other top-level pages ---------------------------------------------------------

/// Guarded: the router sends it to [`CART`] while the cart is empty.
pub(crate) const CHECKOUT: &str = "/checkout";
pub(crate) const ORDER_PLACED: &str = "/order-placed";

// ---- under HOME: `/products/:id`, `/products/:id/reviews`, `/products/:id/size` ----

pub(crate) const PRODUCT: &str = "products/:id";
pub(crate) const REVIEWS: &str = "reviews";
pub(crate) const SIZE: &str = "size";

// ---- names --------------------------------------------------------------------------

pub(crate) const SEARCH_NAME: &str = "search";
pub(crate) const PRODUCT_NAME: &str = "product";
pub(crate) const REVIEWS_NAME: &str = "reviews";
pub(crate) const SIZE_NAME: &str = "size";

// ---- query parameters ---------------------------------------------------------------

/// `?q=` on [`SEARCH`]: what was typed.
pub(crate) const QUERY: &str = "q";
/// `?sort=` on [`SEARCH`].
pub(crate) const SORT: &str = "sort";
