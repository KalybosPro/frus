use frus::form::{Form, Rule};

pub(crate) struct LoginState {
    pub email: String,
    pub password: String,
    pub visibility: bool,
    /// Errors are shown only after the first submit attempt.
    pub submitted: bool,
    pub loading: bool,
}

impl LoginState {
    pub(crate) fn set_email(&mut self, value: String) {
        self.email = value
    }

    pub(crate) fn set_password(&mut self, value: String) {
        self.password = value
    }

    pub(crate) fn toggle_visibility(&mut self) {
        self.visibility = !self.visibility
    }

    pub(crate) fn form(&self) -> Form {
        Form::new()
            .field(
                "email",
                &self.email,
                Rule::all([
                    Rule::required("Email is required"),
                    Rule::email("Enter a valid email address"),
                ]),
            )
            .field(
                "password",
                &self.password,
                Rule::all([
                    Rule::required("Password is required"),
                    Rule::min_len(8, "At least 8 characters"),
                ]),
            )
    }
}
