#[derive(Debug, Clone, PartialEq)]
pub(crate) struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password: String,
}
