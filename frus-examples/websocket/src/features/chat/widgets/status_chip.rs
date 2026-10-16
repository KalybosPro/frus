use frus::{
    Align, BuildContext, Container, FontWeight, Icon, Icons, Row, StatelessWidget, Text, Widget,
};

use crate::common::colors::{failed, offline, on_status, online, pending};
use crate::features::chat::states::chatstate::Status;

/// The connection's status, as a coloured pill: what the reader checks first when
/// nothing seems to happen.
///
/// Built from a `Container` rather than a `Chip`: a chip is a control (a filter, an
/// entry), and its leading icon takes the theme's accent — here both the fill and the
/// icon carry the status, so a plain decorated box says it more directly.
pub(crate) struct StatusChip {
    status: Status,
}

impl StatusChip {
    pub(crate) fn new(status: Status) -> Self {
        Self { status }
    }
}

impl StatelessWidget for StatusChip {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let (label, icon, color) = match &self.status {
            Status::Connecting => ("Connecting", Icons::SYNC, pending()),
            Status::Connected => ("Connected", Icons::CLOUD_DONE, online()),
            Status::Closed => ("Closed", Icons::CLOUD_OFF, offline()),
            Status::Failed { .. } => ("Error", Icons::ERROR_OUTLINE, failed()),
        };

        Box::new(
            Container::new()
                .color(color)
                .radius(16.0)
                .padding_each(6.0, 12.0, 6.0, 10.0)
                .child(
                    Row::new()
                        .spacing(6.0)
                        .cross_axis_alignment(Align::Center)
                        .child(Icon::new(icon).size(16.0).color(on_status()))
                        .child(
                            Text::new(label)
                                .size(13.0)
                                .weight(FontWeight::Medium)
                                .color(on_status()),
                        ),
                ),
        )
    }
}
