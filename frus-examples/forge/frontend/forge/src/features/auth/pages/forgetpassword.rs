use frus::{Center, Colors, Scaffold, State, StatefulWidget, Text};

use crate::features::auth::states::forgetpassword::ForgetPasswordState;

pub(crate) struct ForgetPasswordPage;

impl ForgetPasswordPage {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for ForgetPasswordPage {
    type State = ForgetPasswordState;

    fn create_state(&self) -> Self::State {
        ForgetPasswordState
    }
}

impl State for ForgetPasswordState {
    type Widget = ForgetPasswordPage;

    fn build(&self, _cx: &frus::StateContext<Self>) -> Box<dyn frus::Widget> {
        Scaffold::new()
            .background(Colors::WHITE)
            .body(Center::new(Text::new("Forget Password Page")))
            .build()
    }
}
