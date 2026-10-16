use frus::{
    Align, BuildContext, Expanded, LinearProgressIndicator, Row, SizedBox, StatelessWidget, Text,
    Widget,
};

use crate::common::colors;
use crate::features::signup::controllers::validation::password_strength;

/// The meter under the password field: a bar that fills and turns green as the password
/// gets harder to guess, and a word that says the same for anyone who cannot see colour.
///
/// Configured by the page with the password as it stands; made again on every keystroke.
pub(crate) struct PasswordStrengthMeter {
    password: String,
}

impl PasswordStrengthMeter {
    pub(crate) fn new(password: String) -> Self {
        Self { password }
    }
}

impl StatelessWidget for PasswordStrengthMeter {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        // Nothing to rate yet: take no room at all.
        if self.password.is_empty() {
            return Box::new(SizedBox::empty());
        }
        let strength = password_strength(&self.password);
        let color = colors::strength(strength);
        let theme = cx.theme();

        Box::new(
            Row::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                .child(Expanded::new(
                    LinearProgressIndicator::new(strength.fraction())
                        .color(color)
                        .min_height(6.0)
                        .radius(3.0)
                        .semantics_label("Password strength")
                        .semantics_value(strength.label()),
                ))
                .child(Text::styled(strength.label(), theme.text.label_medium).color(color)),
        )
    }
}
