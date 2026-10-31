# Taking input: fields, controls and forms

A control in frus shows a value that **your code** holds, and tells your code when the person
changes it. It does not keep a value of its own that you then have to read back: the state is
yours, and the control is how it is shown. That is what lets a form be checked, reset or filled
from anywhere.

## A text field

`TextField::new(value)` shows `value`; `on_input` is called with the new text at every key.
In a function component, the text lives in a `use_state`:

```rust
use frus::{component, Column, Text, TextField, Widget};

fn greeting() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let name = cx.use_state(String::new);
        let typed = name.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .child(
                    TextField::new(name.get())
                        .label("Your name")
                        .on_input(move |text: String| typed.set(text)),
                )
                .child(Text::new(format!("Hello, {}", name.get()))),
        )
    }))
}
```

A field says what it is for, and the phone's keyboard follows: `.keyboard_type(KeyboardType::Email)`,
`.obscure(true)` for a password, `.prefix_icon(Icons::SEARCH)`, `.placeholder("…")`,
`.error("Enter an address")` under it, `.on_submit(..)` when the person presses Enter or the
keyboard's action key.

```rust
use frus::{component, Icons, KeyboardType, TextField, Widget};

fn email_field() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let email = cx.use_state(String::new);
        let typed = email.clone();
        let value = email.get();
        let mut field = TextField::new(&value)
            .label("Email")
            .keyboard_type(KeyboardType::Email)
            .prefix_icon(Icons::EMAIL)
            .on_input(move |text: String| typed.set(text));
        if !value.is_empty() && !value.contains('@') {
            field = field.error("Enter an address like name@example.com");
        }
        Box::new(field)
    }))
}
```

## When code writes the text: a controller

When something other than the keyboard changes the text — a "Clear" button, a suggestion that
fills the field — give the field a `TextEditingController`. What is typed lands in it, and what
the program writes appears in the field:

```rust
use frus::{component, Button, Column, Row, TextField, Widget};

fn message_box() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let message = cx.use_text_controller("");
        let (shout, clear) = (message.clone(), message.clone());
        Box::new(
            Column::new()
                .spacing(8.0)
                .child(TextField::new("").label("Message").controller(&message))
                .child(
                    Row::new()
                        .spacing(8.0)
                        .child(Button::new("Upper case").on_press(move || {
                            shout.set_text(shout.text().to_uppercase())
                        }))
                        .child(Button::new("Clear").on_press(move || clear.clear())),
                ),
        )
    }))
}
```

## Choices: checkbox, switch, radio, slider, dropdown

Each takes the current value and calls you with the new one. With a `StatefulWidget`, the
values are fields of the state, and `cx.handler(..)` turns "change this field" into the
control's callback:

```rust
use frus::{
    Align, Checkbox, Column, DropdownButton, RadioGroup, Slider, State, StateContext,
    StatefulWidget, Switch, Text, Widget,
};

const SIZES: [&str; 3] = ["Small", "Medium", "Large"];

struct Preferences;

#[derive(Default)]
struct PreferencesState {
    newsletter: bool,
    dark: bool,
    size: usize,
    volume: f32,
    country: usize,
    country_open: bool,
}

impl StatefulWidget for Preferences {
    type State = PreferencesState;
    fn create_state(&self) -> PreferencesState {
        PreferencesState { volume: 0.5, ..Default::default() }
    }
}

const COUNTRIES: [&str; 3] = ["Côte d'Ivoire", "Ghana", "Senegal"];

impl State for PreferencesState {
    type Widget = Preferences;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let sizes = SIZES.iter().fold(
            RadioGroup::new(self.size, cx.handler(|s: &mut Self, i: usize| s.size = i)),
            |group, size| group.option(*size),
        );
        let country = DropdownButton::new(
            COUNTRIES[self.country],
            cx.callback(|s: &mut Self| s.country_open = !s.country_open),
        )
        .selected(self.country)
        .options(
            self.country_open,
            &COUNTRIES,
            cx.handler(|s: &mut Self, i: usize| {
                s.country = i;
                s.country_open = false;
            }),
        );

        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Checkbox::new(self.newsletter)
                        .label("Send me the newsletter")
                        .on_toggle(cx.handler(|s: &mut Self, on: bool| s.newsletter = on)),
                )
                .child(
                    Switch::new(self.dark)
                        .on_toggle(cx.handler(|s: &mut Self, on: bool| s.dark = on)),
                )
                .child(sizes)
                .child(
                    Slider::new(self.volume)
                        .on_change(cx.handler(|s: &mut Self, v: f32| s.volume = v)),
                )
                .child(Text::new(format!("Volume {:.0}%", self.volume * 100.0)))
                .child(country),
        )
    }
}
```

The dropdown is controlled twice over: the state holds the choice **and** whether its list is
open. That is what lets a screen close it, or open it from a keyboard shortcut.

A control is turned off with `.enabled(false)`; it then looks disabled and ignores the person.
There are more: `RangeSlider`, `SegmentedButton`, `Rating`, `Stepper`, date and time pickers,
`Autocomplete`, `SearchBar`. The widget gallery (`frus-examples/widgets`) shows each one.

## A form, checked

`frus::form` checks a set of values against rules and says what is wrong with each. It only
reads values: it works on whatever the state holds.

```rust
use frus::form::{Form, Rule};

fn sign_up(name: &str, email: &str, password: &str) -> Form {
    Form::new()
        .field("name", name, Rule::required("Enter your name"))
        .field(
            "email",
            email,
            Rule::all([
                Rule::required("Enter your email"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field("password", password, Rule::min_len(8, "At least 8 characters"))
}

let form = sign_up("Ada", "ada@example", "short");
assert!(!form.is_valid());
assert_eq!(form.error("password"), Some("At least 8 characters"));
assert_eq!(form.first_invalid(), Some("email"));
```

A screen usually shows the errors only once the person has tried to send, and then moves the
focus to the first field that is wrong. Each field carries a key (`keyed`) so that
`host::focus(key)` can find it:

```rust
use frus::form::{Form, Rule};
use frus::{host, keyed, Button, Column, State, StateContext, StatefulWidget, TextField, Widget};

struct Login;

#[derive(Default)]
struct LoginState {
    email: String,
    password: String,
    submitted: bool,
}

impl LoginState {
    fn form(&self) -> Form {
        Form::new()
            .field("email", &self.email, Rule::email("Enter a valid email address"))
            .field("password", &self.password, Rule::required("Enter your password"))
    }

    fn submit(&mut self) {
        self.submitted = true;
        if let Some(first) = self.form().first_invalid() {
            host::focus(("login", first));
        }
    }
}

impl StatefulWidget for Login {
    type State = LoginState;
    fn create_state(&self) -> LoginState {
        LoginState::default()
    }
}

impl State for LoginState {
    type Widget = Login;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let form = self.form();
        let error = |key| if self.submitted { form.error(key) } else { None };

        let mut email = TextField::new(&self.email)
            .label("Email")
            .on_input(cx.handler(|s: &mut Self, t: String| s.email = t));
        if let Some(message) = error("email") {
            email = email.error(message);
        }
        let mut password = TextField::new(&self.password)
            .label("Password")
            .obscure(true)
            .on_input(cx.handler(|s: &mut Self, t: String| s.password = t));
        if let Some(message) = error("password") {
            password = password.error(message);
        }

        Box::new(
            Column::new()
                .spacing(12.0)
                .child(keyed(("login", "email"), email))
                .child(keyed(("login", "password"), password))
                .child(Button::new("Sign in").on_press(cx.callback(LoginState::submit))),
        )
    }
}
```

The checking is a plain function of the state, so it is tested without a screen — see
[Testing](testing.md). Sending the form to a server is an `async` handler: see
[Talking to a server](network.md).
