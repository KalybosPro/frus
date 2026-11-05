//! The application's routes.

/// The list of categories.
pub const HOME: &str = "/";
/// One category's widgets: `/category/<id>`, under [`HOME`] — a sub-route's path is
/// relative to its parent's, so that a link straight to a category has the list of
/// categories beneath it to go back to.
pub const CATEGORY: &str = "category/:id";
/// One widget's demo: `/widget/<id>`, under [`HOME`] too.
pub const DEMO: &str = "widget/:id";

pub fn category_location(id: &str) -> String {
    format!("/category/{id}")
}

pub fn demo_location(id: &str) -> String {
    format!("/widget/{id}")
}
