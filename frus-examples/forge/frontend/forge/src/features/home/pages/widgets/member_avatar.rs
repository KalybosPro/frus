use frus::{
    BuildContext, CircleAvatar, Color, FontWeight,
    StatelessWidget, Text, Widget,
};

pub(crate) struct MemberAvatar {
    pub name: String,
    pub color: Color,
}

impl MemberAvatar {
    pub fn new(name: String, color: Color) -> Self {
        Self { name, color }
    }
}

impl StatelessWidget for MemberAvatar {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let content = CircleAvatar::new()
            .child(
                Text::new(self.name.to_string().split_at(1).0.to_uppercase())
                    .color(self.color)
                    .size(10.0)
                    .weight(FontWeight::SemiBold),
            )
            .radius(14.0)
            .background_color(self.color.with_alpha(0.15));

        Box::new(content)
    }
}
