use frus::{Center, Colors, Scaffold, State, StatefulWidget, Text};

use crate::features::auth::states::registerstate::RegisterState;

pub(crate) struct RegisterPage;

impl RegisterPage {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for RegisterPage {
    type State = RegisterState;

    fn create_state(&self) -> Self::State {
        RegisterState
    }
}

impl State for RegisterState {
    type Widget = RegisterPage;

    fn build(&self, _cx: &frus::StateContext<Self>) -> Box<dyn frus::Widget> {
        Box::new(
            Scaffold::new()
                .background(Colors::WHITE)
                .body(Center::new(Text::new("Register Page")))
                .build(),
        )
    }
}
