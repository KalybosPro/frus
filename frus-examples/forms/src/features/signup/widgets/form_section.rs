use frus::{Align, BuildContext, Card, Column, Text, Widget};

/// One titled group of fields, on an outlined card: "Account", "About you"…
///
/// A plain function rather than a `StatelessWidget`: it takes its fields **by value** and
/// places them once, while a widget's `build(&self)` may run many times and could only
/// borrow them.
pub(crate) fn form_section(
    cx: &BuildContext,
    title: &str,
    fields: Vec<Box<dyn Widget>>,
) -> Box<dyn Widget> {
    let theme = cx.theme();
    let column = fields.into_iter().fold(
        Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Stretch)
            .child(Text::styled(title, theme.text.title_medium).color(theme.on_surface)),
        |column, field| column.child(field),
    );
    Box::new(
        Card::new()
            .outlined()
            .margin(0.0)
            .padding(16.0)
            .child(column),
    )
}
