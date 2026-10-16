use std::rc::Rc;

use frus::{
    Align, AppBar, BuildContext, Button, Card, Column, Expanded, Icon, Icons, Row, Scaffold,
    SizedBox, StatelessWidget, Text, Variant, Widget,
};

use crate::common::colors::success;
use crate::common::routes::SIGNUP;
use crate::common::shared::centered_page;
use crate::models::profile::Profile;

/// What the form submitted, read back to the person who submitted it.
///
/// A `StatelessWidget`: it remembers nothing, it shows the profile it is given. The profile
/// came with the navigation (`push_with_extra`); opened any other way — a typed address,
/// `cargo run -- /summary` — there is none, and the page shows sample data under a notice
/// rather than an empty card.
pub(crate) struct SummaryPage {
    profile: Option<Rc<Profile>>,
}

impl SummaryPage {
    pub(crate) fn new(profile: Option<Rc<Profile>>) -> Self {
        Self { profile }
    }
}

impl StatelessWidget for SummaryPage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let router = cx.router().clone();
        let (profile, is_preview) = match &self.profile {
            Some(profile) => ((**profile).clone(), false),
            None => (Profile::sample(), true),
        };

        let first_name = profile
            .full_name
            .split_whitespace()
            .next()
            .unwrap_or("there")
            .to_string();

        let mut content = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Center)
            .child(Icon::new(Icons::CHECK_CIRCLE).size(64.0).color(success()))
            .child(Text::styled(
                format!("Welcome, {first_name}!"),
                theme.text.headline_small,
            ))
            .child(Text::new("Your account is ready. This is what we saved.").color(theme.muted));
        if is_preview {
            content = content.child(
                Text::styled(
                    "Preview: nothing was submitted, so this shows sample data.",
                    theme.text.body_small,
                )
                .color(theme.muted),
            );
        }

        let interests = if profile.interests.is_empty() {
            "—".to_string()
        } else {
            profile.interests.join(", ")
        };
        let rows = [
            ("Name", profile.full_name.clone()),
            ("Email", profile.email.clone()),
            ("Phone", profile.phone.clone()),
            ("Date of birth", profile.birth_date.to_string()),
            ("Country", profile.country.clone()),
            ("Plan", profile.plan.name().to_string()),
            ("Interests", interests),
            (
                "Newsletter",
                if profile.newsletter { "Yes" } else { "No" }.to_string(),
            ),
            (
                "Bio",
                if profile.bio.is_empty() {
                    "—".to_string()
                } else {
                    profile.bio.clone()
                },
            ),
        ];
        let details = rows
            .into_iter()
            .fold(Column::new().spacing(12.0), |column, (label, value)| {
                column.child(detail_row(cx, label, value))
            });

        // Back to the form: `pop` when it is underneath (its state, and the values typed
        // into it, are still there), `go` when this page was opened on its own.
        let back = Button::new("Back to the form")
            .variant(Variant::Outlined)
            .on_press(move || {
                if router.can_pop() {
                    router.pop();
                } else {
                    router.go(SIGNUP);
                }
            });

        let content = content
            .child(
                Card::new()
                    .outlined()
                    .margin(0.0)
                    .padding(16.0)
                    .child(details),
            )
            .child(back);

        Scaffold::new()
            .background(theme.background)
            .app_bar(AppBar::new().title(Text::new("Account created")).build())
            .body(centered_page(content))
            .build()
    }
}

/// One "label — value" line. The label column has a fixed width so the values line up; a
/// long value wraps inside the room that is left.
fn detail_row(cx: &BuildContext, label: &str, value: String) -> Row {
    let theme = cx.theme();
    Row::new()
        .spacing(12.0)
        .cross_axis_alignment(Align::Start)
        .child(
            SizedBox::new(Text::styled(label, theme.text.label_large).color(theme.muted))
                .width(120.0),
        )
        .child(Expanded::new(
            Text::styled(value, theme.text.body_large).wrap(),
        ))
}
