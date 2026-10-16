//! `TextField` — a text input. It is **controlled**: the application holds the text, the
//! field shows it and reports every change through `on_input`.

use frus::{
    Align, Color, Column, Icons, KeyboardType, State, StateContext, StatefulWidget, Text,
    TextAlign, TextField, TextFieldStyle, TextInputAction, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(TextFieldDemo.into_widget())
}

/// Wide enough to read, narrow enough for a phone.
const WIDTH: f32 = 300.0;

struct TextFieldDemo;

#[derive(Default)]
struct TextFieldDemoState {
    name: String,
    email: String,
    password: String,
    show_password: bool,
    search: String,
    notes: String,
    amount: String,
    phone: String,
    code: String,
    plain: String,
    styled: String,
    submitted: String,
}

impl StatefulWidget for TextFieldDemo {
    type State = TextFieldDemoState;

    fn create_state(&self) -> TextFieldDemoState {
        TextFieldDemoState {
            email: "ada@".into(),
            ..Default::default()
        }
    }
}

impl State for TextFieldDemoState {
    type Widget = TextFieldDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The error is computed from the state on every build: the field only shows it.
        let email_error = (!self.email.is_empty() && !self.email.contains('.'))
            .then_some("Enter an address like name@example.com");

        let mut email = TextField::new(&self.email)
            .label("Email")
            .keyboard_type(KeyboardType::Email)
            .prefix_icon(Icons::EMAIL)
            .width(WIDTH)
            .on_input(cx.handler(|s: &mut TextFieldDemoState, t: String| s.email = t));
        if let Some(message) = email_error {
            email = email.error(message);
        }

        sections(vec![
            // Label above the value, placeholder while empty, helper text below.
            // `on_submit` fires on Enter.
            Box::new(section(
                "Label, placeholder and helper",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(4.0)
                    .child(
                        TextField::new(&self.name)
                            .label("Name")
                            .placeholder("Ada Lovelace")
                            .helper("As it appears on your card")
                            .width(WIDTH)
                            .action(TextInputAction::Done)
                            .on_input(
                                cx.handler(|s: &mut TextFieldDemoState, t: String| s.name = t),
                            )
                            .on_submit(cx.callback(|s: &mut TextFieldDemoState| {
                                s.submitted = s.name.clone()
                            })),
                    )
                    .child(Text::new(format!("Submitted: {}", self.submitted))),
            )),
            // An error replaces the helper and turns the field red.
            Box::new(section("Error and prefix icon", email)),
            // `obscure` masks the value; a clickable suffix icon toggles it.
            Box::new(section(
                "Password with a visibility toggle",
                TextField::new(&self.password)
                    .label("Password")
                    .obscure(!self.show_password)
                    .prefix_icon(Icons::LOCK)
                    .suffix_icon(if self.show_password {
                        Icons::VISIBILITY_OFF
                    } else {
                        Icons::VISIBILITY
                    })
                    .on_suffix(
                        cx.callback(|s: &mut TextFieldDemoState| {
                            s.show_password = !s.show_password
                        }),
                    )
                    .width(WIDTH)
                    .on_input(cx.handler(|s: &mut TextFieldDemoState, t: String| s.password = t)),
            )),
            // A clear button: the suffix only shows once there is something to clear.
            Box::new(section("Clear button", {
                let field = TextField::new(&self.search)
                    .placeholder("Search")
                    .prefix_icon(Icons::SEARCH)
                    .width(WIDTH)
                    .on_input(cx.handler(|s: &mut TextFieldDemoState, t: String| s.search = t));
                if self.search.is_empty() {
                    field
                } else {
                    field
                        .suffix_icon(Icons::CLOSE)
                        .on_suffix(cx.callback(|s: &mut TextFieldDemoState| s.search.clear()))
                }
            })),
            // Several lines, and a character limit with its counter.
            Box::new(section(
                "Multiline with a limit",
                TextField::new(&self.notes)
                    .label("Notes")
                    .rows(4)
                    .max_length(200)
                    .width(WIDTH)
                    .on_input(cx.handler(|s: &mut TextFieldDemoState, t: String| s.notes = t)),
            )),
            // The keyboard type picks the phone's keyboard; `digits_only` and
            // `input_filter` decide what may be typed at all.
            Box::new(section(
                "Keyboard types and input filters",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(12.0)
                    .child(
                        TextField::new(&self.amount)
                            .label("Amount")
                            .keyboard_type(KeyboardType::Decimal)
                            .text_align(TextAlign::Right)
                            .width(WIDTH)
                            .on_input(
                                cx.handler(|s: &mut TextFieldDemoState, t: String| s.amount = t),
                            ),
                    )
                    .child(
                        TextField::new(&self.phone)
                            .label("Phone")
                            .keyboard_type(KeyboardType::Phone)
                            .digits_only()
                            .width(WIDTH)
                            .on_input(
                                cx.handler(|s: &mut TextFieldDemoState, t: String| s.phone = t),
                            ),
                    )
                    .child(
                        TextField::new(&self.code)
                            .label("Code (upper case)")
                            .input_filter(|c| {
                                c.is_ascii_alphanumeric().then(|| c.to_ascii_uppercase())
                            })
                            .max_length(6)
                            .width(WIDTH)
                            .on_input(
                                cx.handler(|s: &mut TextFieldDemoState, t: String| s.code = t),
                            ),
                    ),
            )),
            // Outlined (the default), filled, or borderless — the last one for a field
            // inside something that is already a container.
            Box::new(section(
                "Variants, disabled and read-only",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(12.0)
                    .child(
                        TextField::new(&self.plain)
                            .label("Filled")
                            .filled()
                            .width(WIDTH)
                            .on_input(
                                cx.handler(|s: &mut TextFieldDemoState, t: String| s.plain = t),
                            ),
                    )
                    .child(
                        TextField::new(&self.plain)
                            .placeholder("Borderless")
                            .borderless()
                            .width(WIDTH)
                            .on_input(
                                cx.handler(|s: &mut TextFieldDemoState, t: String| s.plain = t),
                            ),
                    )
                    .child(
                        TextField::new("Cannot edit")
                            .label("Disabled")
                            .enabled(false)
                            .width(WIDTH),
                    )
                    .child(
                        TextField::new("Select me, but no typing")
                            .label("Read-only")
                            .read_only()
                            .width(WIDTH),
                    ),
            )),
            // `TextFieldStyle` overrides any colour or measurement; unset ones come from
            // the theme.
            Box::new(section(
                "Custom style",
                TextField::new(&self.styled)
                    .label("Styled")
                    .style(TextFieldStyle {
                        fill: Some(Color::hex("#FEF3C7")),
                        border_color: Some(Color::hex("#F59E0B")),
                        focused_border_color: Some(Color::hex("#B45309")),
                        label_color: Some(Color::hex("#92400E")),
                        focused_label_color: Some(Color::hex("#B45309")),
                        text_color: Some(Color::hex("#451A03")),
                        radius: Some(16.0),
                        ..Default::default()
                    })
                    .width(WIDTH)
                    .on_input(cx.handler(|s: &mut TextFieldDemoState, t: String| s.styled = t)),
            )),
        ])
    }
}
