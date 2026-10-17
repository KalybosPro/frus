//! The application's routes.

/// The list of categories.
pub const HOME: &str = "/";
/// One category's widgets: `/category/<id>`.
pub const CATEGORY: &str = "/category/:id";
/// One widget's demo: `/widget/<id>`.
pub const DEMO: &str = "/widget/:id";

pub fn category_location(id: &str) -> String {
    format!("/category/{id}")
}

pub fn demo_location(id: &str) -> String {
    format!("/widget/{id}")
}
