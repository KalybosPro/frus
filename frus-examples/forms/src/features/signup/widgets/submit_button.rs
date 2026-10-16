use frus::{BuildContext, Button, Callback, CircularProgressIndicator, StatelessWidget, Widget};

/// "Create account" — or, while the request is in flight, a disabled button with a spinner.
///
/// Disabling it is what stops a second press from sending the form twice; the spinner is
/// what tells the person their first press was heard.
pub(crate) struct SubmitButton {
    submitting: bool,
    on_press: Callback,
}

impl SubmitButton {
    pub(crate) fn new(submitting: bool, on_press: Callback) -> Self {
        Self {
            submitting,
            on_press,
        }
    }
}

impl StatelessWidget for SubmitButton {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let button = if self.submitting {
            Button::new("Creating your account…")
                .icon(
                    CircularProgressIndicator::new()
                        .size(16.0)
                        .stroke_width(2.0),
                )
                .enabled(false)
        } else {
            Button::new("Create account")
        };
        Box::new(button.height(48.0).on_press(self.on_press.clone()))
    }
}
